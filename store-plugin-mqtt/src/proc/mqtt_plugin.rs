use anyhow::anyhow;
use chimes_store_core::config::auth::JwtUserClaims;
use chimes_store_core::config::PluginConfig;
use chimes_store_core::pin_submit;
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::queue::SyncTaskQueue;
use chimes_store_core::service::script::ExtensionRegistry;
use chimes_store_core::service::sdk::{InvokeUri, MethodDescription, RxPluginService};
use chimes_store_core::service::starter::load_config;
use chimes_store_core::utils::global_data::i64_from_str;
use rbatis::Page;
use salvo::oapi::{Content, Object, OpenApi, Operation, PathItem, RefOr, RequestBody, Response, Schema, BasicType, ToArray};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use paho_mqtt::{self as mqtt};

use super::template::json_path_get;

fn to_api_result_schema(t: RefOr<Schema>, array: bool) -> Schema {
    let mut apiresult = Object::new();
    apiresult = apiresult.property(
        "status",
        Object::new().schema_type(BasicType::Integer),
    );
    apiresult = apiresult.property(
        "message",
        Object::new().schema_type(BasicType::String),
    );
    if array {
        apiresult = apiresult.property("data", t.to_array());
    } else {
        apiresult = apiresult.property("data", t);
    }
    apiresult = apiresult.property(
        "timestamp",
        Object::new().schema_type(BasicType::Integer),
    );
    Schema::Object(apiresult)
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MqttSubscribeInfo {
    pub topic: String,
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub qos: Option<i64>,

    pub description: Option<String>,

    #[serde(default)]
    pub consumer: bool,

    #[serde(default)]
    pub producer: bool,

    #[serde(default)]
    pub enable_synctask: bool,
    pub task_id: Option<String>,
    
    #[serde(default)]
    pub execute_delete: bool,      // 执行删除动作，如果为true，所有接收到的都都是删除操作 
    pub check_delete: Option<String>,  // 检查删除标识，JSONPath表示，检查成功，表示该记录应该执行删除操作


    pub lang: Option<String>,
    pub script: Option<String>,
}

impl MqttSubscribeInfo {
    pub(crate) fn to_operation(&self, array: bool) -> Operation {
        let mut ins_op = Operation::new();
        ins_op = ins_op.request_body(
            RequestBody::new()
                .add_content("application/json", RefOr::Type(Schema::Object(Object::new()))),
        );
        ins_op = ins_op.summary(self.description.clone().unwrap_or_default());

        let mut description = "使用定义的MQTT连接来发送MQTT消息".to_string();
        description.push_str("我们通过MQTT的publish方法来来发送时，主要是使用topic，qos，以及payload来作为参数即可。\n");
        description.push_str("Query：我们可以使用QueryString来传递topic和qos，如果Query在转成JSON时，包含了值，则会将其作为参数传过去.\n");
        description.push_str("Body: 主要是传递payload。当然也可以传递topic和qos。\n");
        description.push_str("Body有几种情况：\n");
        description.push_str("1、Body是一个JSON数组，且该数据的长度大于2，则第一个JSON为topic，第二个为qos，第三个为payload.\n");
        description.push_str("2、Body是一个JSON数组，且该数据的长度等于2，则第一个JSON为{topic, qos}，第二个为payload.\n");
        description.push_str("3、Body是一个JSON对象，它必须包含了topic, qos, payload三个属性（注qos为可选，如果没有传，则为0）.\n");

        ins_op = ins_op.description(description);

        let mut resp = Response::new("返回ApiResult结构的JSON对象。".to_owned());
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(
                RefOr::Type(Schema::Object(Object::new())),
                array,
            )),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));
        ins_op
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MqttPluginConfig {
    pub connection: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub keep_alive: Option<i64>,
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub min_retry: Option<i64>,
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub max_retry: Option<i64>,
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub timeout: Option<i64>,
    #[serde(default)]
    pub reserver_dot: bool,
    pub username: Option<String>,
    pub password: Option<String>,
    pub services: Vec<MqttSubscribeInfo>,
}

impl MqttPluginConfig {
    
    pub fn get_service(&self, topic: &str) -> Option<MqttSubscribeInfo> {
        self.services.clone().into_iter().find(|f| f.topic == *topic)
    }
}

#[allow(dead_code)]
pub struct MqttPluginService {
    namespace: String,
    conf: PluginConfig,
    mqtt: Mutex<Option<MqttPluginConfig>>,
    client: Arc<Option<mqtt::Client>>,
    running: Arc<AtomicBool>,
}

impl MqttPluginService {
    pub fn new(ns:&str, conf: &PluginConfig) -> Result<Self, anyhow::Error> {
        log::debug!("Plugin config load from {}", conf.config.clone());
        let t = match load_config(conf.config.clone()) {
            Ok(r) => r,
            Err(err) => {
                log::debug!("Could not load the config file: {:?}", err);
                Some(MqttPluginConfig::default())
            }
        };

        Ok(Self {
            namespace: ns.to_owned(),
            conf: conf.to_owned(),
            mqtt: Mutex::new(t),
            client: Arc::new(None),
            running: Arc::new(AtomicBool::new(false)),
        })
    }
}

unsafe impl Send for MqttPluginService {}

unsafe impl Sync for MqttPluginService {}

impl MqttPluginService {

    #[allow(dead_code)]
    pub fn handle_topic_process(mqtt: &MqttSubscribeInfo, _topic: &str, payload: &Value) -> Result<(), anyhow::Error>{
        let mut rt = None;
        if let Some(lang) = mqtt.lang.clone() {
            if let Some(scripter) = ExtensionRegistry::get_extension(&lang) {
                if let Some(eval) = scripter.fn_return_option_script {
                    rt = eval(&mqtt.script.clone().unwrap_or_default(), Arc::new(Mutex::new(InvocationContext::new())), &[payload.to_owned()])?;
                }
            }
        }
        
        if mqtt.enable_synctask {
            if let Some(val) = rt {
                if let Some(task_id) = mqtt.task_id.clone() {
                    let state_action = mqtt.execute_delete || if let Some(chk) = mqtt.check_delete.clone() {
                        json_path_get(&val, &chk).is_some()
                    } else {
                        false
                    };

                    let state = if state_action {
                        2
                    } else {
                        1
                    };

                    if let Err(err) = SyncTaskQueue::get_mut().push_task(&task_id, &val, state) {
                        log::info!("could not add the value into SyncTaskQueue {err}");
                    }
                }
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn start(&mut self) -> Result<(), anyhow::Error> {

        if let Some(mqttconf) = self.mqtt.lock().unwrap().clone() {
            if mqttconf.connection.is_none() {
                return Err(anyhow!("No connection provided."));
            }

            let mut builder = mqtt::ConnectOptionsBuilder::new();
            builder.clean_session(true);
            if mqttconf.min_retry.is_some() && mqttconf.max_retry.is_some() {
                builder.automatic_reconnect(Duration::from_secs(mqttconf.min_retry.unwrap() as u64), Duration::from_secs(mqttconf.max_retry.unwrap() as u64));
            }
            
            let timeout = mqttconf.timeout.unwrap_or(30);
            builder.connect_timeout(Duration::from_secs(timeout as u64));
            
            if mqttconf.keep_alive.is_some() {
                builder.keep_alive_interval(Duration::from_secs(mqttconf.keep_alive.unwrap() as u64));
            }

            let cli = mqtt::Client::new(mqttconf.connection.unwrap_or_default())?;
            let sr = cli.connect(builder.finalize())?;
            if sr.reason_code().is_ok() {
                self.client = Arc::new(Some(cli));
                let client = self.client.clone();
                let subs = mqttconf.services.clone().into_iter().filter(|p| p.consumer).map(|f| (f.topic, f.qos.unwrap_or(1))).collect::<Vec<(String, i64)>>();
                if let Ok(subres) = &client.as_ref().clone().unwrap().subscribe_many(&subs.clone().into_iter().map(|f| f.0).collect::<Vec<String>>(), &subs.clone().into_iter().map(|f|f.1 as i32).collect::<Vec<i32>>()) {
                    if subres.reason_code().is_ok() {
                        let handle_map = mqttconf.services.clone().into_iter().map(|f| (f.topic.clone(), f)).collect::<HashMap<String, MqttSubscribeInfo>>();
                        let running = self.running.clone();
                        // thread::spawn(move || {
                        pin_submit!(async move {
                            let mut receiver = client.as_ref().clone().unwrap().start_consuming();
                            let conn = AtomicBool::new(client.as_ref().clone().unwrap().is_connected());
                            running.store(true, std::sync::atomic::Ordering::Release);
                            loop {

                                {
                                    let still_running = running.load(std::sync::atomic::Ordering::Acquire);
                                    if !still_running {
                                        log::info!("exit mqtt message consume thread.");
                                        return;
                                    }
                                }

                                let reconn = client.as_ref().clone().unwrap().is_connected();
                                let oldconn = conn.load(std::sync::atomic::Ordering::Acquire);
                                // log::info!("client connected: {} / {}", reconn, oldconn);

                                if  !oldconn && reconn {
                                    // reconnected, we should restart subscribe and consuming
                                    if let Ok(sr) = &client.as_ref().clone().unwrap().subscribe_many(&subs.clone().into_iter().map(|f| f.0).collect::<Vec<String>>(), &subs.clone().into_iter().map(|f|f.1 as i32).collect::<Vec<i32>>()) {
                                        if sr.reason_code().is_ok() {
                                            log::info!("re-subscribe success");
                                            receiver = client.as_ref().clone().unwrap().start_consuming();
                                            conn.store(reconn, std::sync::atomic::Ordering::Release);
                                        }
                                    }
                                } else {
                                    conn.store(reconn, std::sync::atomic::Ordering::Release);
                                }

                                match receiver.recv_timeout(Duration::from_secs(timeout as u64)) {
                                    Ok(t) => {
                                        if let Some(msg) = t {
                                            let topic = msg.topic();
                                            let payload = msg.payload_str().to_string();
                                            log::info!("received msg for {}", topic);
                                            log::info!("payload: {payload}");
                                            if let Some(pt) = handle_map.get(topic) {
                                                match serde_json::from_str::<Value>(&payload) {
                                                    Ok(t) => {
                                                        if let Err(err) = Self::handle_topic_process(pt, topic, &t) {
                                                            log::info!("Process the message with error {err:?}");
                                                        }
                                                    },
                                                    Err(err) => {
                                                        log::info!("convert payload to json with error {err:?}");                                                        
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    Err(err) => {
                                        log::info!("err on receive {err:?}");
                                    }
                                }
                            }
                        });
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        self.running.store(false, std::sync::atomic::Ordering::Release);
        if let Some(cli) = self.client.as_ref() {
            cli.stop_consuming();
            if let Err(err) = cli.disconnect(None) {
                log::info!("disconnect with error {err}");
            }
        }
    }

    #[allow(dead_code)]
    pub fn publish(&self, topic: &str, qos: i32, payloads: &[Value]) -> Result<(), anyhow::Error> {
        match self.client.clone().as_ref() {
            Some(cli) => {
                for payload in payloads {
                    let payload_string = serde_json::to_string(payload)?;
                    let msg = mqtt::Message::new(topic, payload_string, qos);
                    cli.publish(msg)?;
                }
                Ok(())
            },
            None => {
                log::info!("Mqtt client was not init.");
                Err(anyhow!("Mqtt client was not init"))
            }
        }
    }

    fn to_openapi_doc(&self, ns: &str) -> OpenApi {
        let mut openapi = OpenApi::new(self.conf.name.clone(), "0.1.0");
        let tconf = self.mqtt.lock().unwrap().clone();
        if let Some(restconf) = tconf {
            for tsvc in restconf
                .services
                .iter()
                .cloned()
            {
                let topic = tsvc.topic.clone();
                let topic = topic.replace('/', ".");

                let opt = tsvc.to_operation(false);
                let one_path = format!(
                    "/api/mqtt/{}/{}/{}/publish",
                    ns,
                    self.conf.name.clone(),
                    topic
                );
                openapi = openapi.add_path(
                    one_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                );
            }
        }
        openapi
    }    
}

impl Drop for MqttPluginService {
    fn drop(&mut self) {
        self.stop();
    }
}

impl RxPluginService for MqttPluginService {
    fn invoke_return_option(
        &self,
        uri: InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, anyhow::Error>> + Send>> {
        let mqttconf = self.mqtt.lock().unwrap().clone().unwrap();
        let topic = uri.method.replace('+', "/");

        match mqttconf.get_service(&topic) {
            None => {
                Box::pin(async move {
                    Err(anyhow!("No {topic} producer be defined"))
                })
            },
            Some(subs) => {
                if !subs.producer {
                    Box::pin(async move {
                        Err(anyhow!("No {topic} producer be defined"))
                    })
                } else {
                    match self.publish(&topic, subs.qos.unwrap_or_default() as i32, &args) {
                        Ok(_) => {
                            Box::pin(async move {
                                Ok(None)
                            })
                        },
                        Err(err) => {
                            Box::pin(async move {
                                Err(err)
                            })                
                        }
                    }
                }
            }
        }
    }

    fn invoke_return_vec(
        &self,
        _uri: InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, anyhow::Error>> + Send>> {
        Box::pin(async move { Err(anyhow!("Not implemented")) })
    }

    fn invoke_return_page(
        &self,
        _uri: InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, anyhow::Error>> + Send>> {
        Box::pin(async move { Err(anyhow!("Not implemented")) })
    }

    fn get_config(&self) -> Option<Value> {
        match serde_json::to_value(self.mqtt.lock().unwrap().clone()) {
            Ok(t) => Some(t),
            Err(err) => {
                log::debug!("Convert to json with error: {:?}", err);
                None
            }
        }
    }

    fn parse_config(&self, val: &Value) -> Result<(), anyhow::Error> {
        log::info!("mqtt config: {val}");
        match serde_json::from_value::<MqttPluginConfig>(val.to_owned()) {
            Ok(t) => {
                self.mqtt.lock().unwrap().replace(t);
                Ok(())
            }
            Err(err) => {
                log::info!("Parse JSON value to config with error: {:?}", err);
                Err(anyhow!(err))
            }
        }
    }

    fn save_config(&self, conf: &PluginConfig) -> Result<(), anyhow::Error> {
        let path: PathBuf = conf.config.clone().into();
        chimes_store_core::service::starter::save_config(
            &self.mqtt.lock().unwrap().clone(),
            path,
        )
    }

    fn get_metadata(&self) -> Vec<chimes_store_core::service::sdk::MethodDescription> {
        vec![MethodDescription {
            uri: format!(
                "{}://{}/{}",
                self.conf.protocol.clone(),
                self.namespace.clone(),
                self.conf.name.clone()
            ),
            name: "publish".to_owned(),
            func: None,
            params_vec: true,
            params1: vec![],
            params2: None,
            response: vec![],
            return_page: false,
            return_vec: false,
        }]
    }

    fn get_openapi(&self, ns: &str) -> Box<dyn std::any::Any> {
        Box::new(self.to_openapi_doc(ns))
    }

    fn has_permission(
        &self,
        _uri: &InvokeUri,
        _jwt: &JwtUserClaims,
        _roles: &[String],
        _bypass: bool,
    ) -> bool {
        true
    }
}
