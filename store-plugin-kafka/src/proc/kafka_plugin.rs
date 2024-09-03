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
use kafka::client::{Compression, FetchOffset, GroupOffsetStorage, KafkaClient, RequiredAcks, DEFAULT_CONNECTION_IDLE_TIMEOUT_MILLIS};
use kafka::consumer::Consumer;
use kafka::producer::{AsBytes, Producer, Record};
use rbatis::Page;
use salvo::oapi::{Content, Object, OpenApi, Operation, PathItem, RefOr, RequestBody, Response, Schema, BasicType, ToArray};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
pub struct KafkaSubscribeInfo {
    pub topic: String,
    pub partitions: Option<String>,

    #[serde(default)]
    pub produce: bool, // 同时，支持Produce，当该值为true时，才可以调用publish方法

    #[serde(default)]
    pub consume: bool, // 同时，支持Consume，当该值为true时，才会加入到consume队列    

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub ack_timeout: Option<i64>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub required_acks: Option<i64>,
    
    #[serde(default)]
    pub enable_synctask: bool,
    pub task_id: Option<String>,

    #[serde(default)]
    pub execute_delete: bool,      // 执行删除动作，如果为true，所有接收到的都都是删除操作 
    pub check_delete: Option<String>,  // 检查删除标识，JSONPath表示，检查成功，表示该记录应该执行删除操作

    pub description: Option<String>,

    pub lang: Option<String>,
    pub script: Option<String>,
}


impl KafkaSubscribeInfo {
    pub(crate) fn to_operation(&self, array: bool) -> Operation {
        let mut ins_op = Operation::new();
        ins_op = ins_op.request_body(
            RequestBody::new()
                .add_content("application/json", RefOr::T(Schema::Object(Object::new()))),
        );
        ins_op = ins_op.summary(self.description.clone().unwrap_or_default());

        let mut description = "使用定义的Kafka连接来发送Kafka消息".to_string();
        description.push_str("我们通过Kafka的publish方法来来发送时，主要是使用topic，qos，以及payload来作为参数即可。\n");
        description.push_str("Query传递少量Kafka Procedure参数. 而topic则是通过URI传递（注意：如topic包含‘/’，需要将其用‘+’替代）\n");
        description.push_str("Body: 主要是传递payload数组\n");


        ins_op = ins_op.description(description);

        let mut resp = Response::new("返回ApiResult结构的JSON对象。".to_owned());
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(
                RefOr::T(Schema::Object(Object::new())),
                array,
            )),
        );
        ins_op = ins_op.add_response("200", RefOr::T(resp));
        ins_op
    }
}


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct KafkaPluginConfig {
    pub brokers: Option<String>,
    
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub keep_alive: Option<i64>,
    
    pub group: Option<String>,
    
    #[serde(default)]
    pub validate_crc: bool,
    
    #[serde(default)]
    pub commit_consumer: bool,
    pub fallback_offset: Option<String>,

    pub compression: Option<String>,
    
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub max_bytes_per_partition: Option<i64>,
    
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub max_wait_time: Option<i64>,
    
    pub offset_storage: Option<String>,
    
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub connection_idle_timeout: Option<i64>,
    
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub retry_max_bytes_limit: Option<i64>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub retry_backoff_time: Option<i64>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub fetch_min_bytes: Option<i64>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub retry_max_attempts: Option<i64>,

    pub services: Vec<KafkaSubscribeInfo>,
}

impl KafkaPluginConfig {
    fn get_service(&self, topic: &str) -> Option<KafkaSubscribeInfo> {
        self.services.clone().into_iter().find(|f| f.topic == *topic)
    }
}

#[allow(dead_code)]
pub struct KafkaPluginService {
    namespace: String,    
    conf: PluginConfig,
    kafka: Mutex<Option<KafkaPluginConfig>>,
    client: Mutex<Option<KafkaClient>>,
    producer: Arc<Mutex<Option<Producer>>>,
    running: Arc<AtomicBool>,
}

struct Trimmed(String);

impl AsBytes for Trimmed {
    fn as_bytes(&self) -> &[u8] {
        self.0.trim().as_bytes()
    }
}

impl Deref for Trimmed {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Trimmed {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn send_batch(producer: Arc<Mutex<Option<Producer>>>, batch: &[Record<'_, (), Trimmed>]) -> Result<(), anyhow::Error> {
    if let Ok(mut grant_producer) = producer.lock() {
        match grant_producer.as_mut() {
            Some(prod) => {
                match prod.send_all(batch) {
                    Ok(rs) => {
                        drop(grant_producer);
                        for r in rs {
                            for tpc in r.partition_confirms {
                                if let Err(code) = tpc.offset {
                                    //return Err(Error::Kafka(kafka::error::Error::Kafka(code)));
                                    log::info!("message confirmed with code: {code:?}.");
                                    return Err(anyhow!("{:?}", code));
                                }
                            }
                        }
                        Ok(())
                    },
                    Err(err) => {
                        drop(grant_producer);
                        log::info!("error on send {err}");
                        Err(anyhow!("{:?}", err))
                    }
                }
            },
            None => {
                Ok(())
            }
        }
    } else {
        Err(anyhow!("could not locked the mutex."))
    }
}

impl KafkaPluginService {

    #[allow(dead_code)]
    pub fn handle_topic_process(kafka: &KafkaSubscribeInfo, _topic: &str, payload: &Value) -> Result<(), anyhow::Error>{
        let mut rt = None;
        if let Some(lang) = kafka.lang.clone() {
            if let Some(scripter) = ExtensionRegistry::get_extension(&lang) {
                if let Some(eval) = scripter.fn_return_option_script {
                    rt = eval(&kafka.script.clone().unwrap_or_default(), Arc::new(Mutex::new(InvocationContext::new())), &[payload.to_owned()])?;
                }
            }
        }

        if kafka.enable_synctask {
            if let Some(val) = rt {
                if let Some(task_id) = kafka.task_id.clone() {
                    let state_action = kafka.execute_delete || if let Some(chk) = kafka.check_delete.clone() {
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

    pub fn new(ns: &str, conf: &PluginConfig) -> Result<Self, anyhow::Error> {
        log::debug!("Plugin config load from {}", conf.config.clone());
        let t = match load_config(conf.config.clone()) {
            Ok(r) => r,
            Err(err) => {
                log::debug!("Could not load the config file: {:?}", err);
                Some(KafkaPluginConfig::default())
            }
        };

        Ok(Self {
            namespace: ns.to_owned(),
            conf: conf.to_owned(),
            kafka: Mutex::new(t),
            client: Mutex::new(None),
            producer: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicBool::new(false)),
        })
    }

    fn create_client(&self) -> KafkaClient {
        let kafka = self.kafka.lock().unwrap().clone().unwrap();
        let hosts = kafka.brokers.unwrap_or_default().split(';').map(|f| f.to_owned()).collect::<Vec<String>>();
        let mut client = KafkaClient::new(hosts);
        client.set_fetch_crc_validation(kafka.validate_crc);
        
        client.set_connection_idle_timeout(Duration::from_millis(kafka.connection_idle_timeout.unwrap_or(DEFAULT_CONNECTION_IDLE_TIMEOUT_MILLIS as i64) as u64));
        if let Some(max_bytes_per_partition) = kafka.max_bytes_per_partition {
            client.set_fetch_max_bytes_per_partition(max_bytes_per_partition as i32);
        }

        if let Err(err) = client.set_fetch_max_wait_time(Duration::from_millis(kafka.max_wait_time.unwrap_or(30) as u64)) {
            log::debug!("error to set fetch max wait time {err}");
        }

        if let Some(fetch_min_bytes) = kafka.fetch_min_bytes {
            client.set_fetch_min_bytes(fetch_min_bytes as i32);
        }

        if let Some(retry_backoff_time) = kafka.retry_backoff_time {
            client.set_retry_backoff_time(Duration::from_millis(retry_backoff_time as u64));
        }

        if let Some(retry_max_attempts) = kafka.retry_max_attempts {
            client.set_retry_max_attempts(retry_max_attempts as u32);
        }

        if let Some(group_offset) = kafka.offset_storage {
            let group_offset_storage = match group_offset.to_lowercase().as_str() {
                "kafka" => Some(GroupOffsetStorage::Kafka),
                "zookeeper" => Some(GroupOffsetStorage::Zookeeper),
                _ => None
            };
            client.set_group_offset_storage(group_offset_storage);
        }

        if let Some(compress) = kafka.compression {
            let compression = match compress.to_lowercase().as_str() {
                "gzip" => Compression::GZIP,
                "snappy" => Compression::SNAPPY,
                _ => Compression::NONE,
            };
            client.set_compression(compression);
        }
        client
    }

    pub fn start(&mut self) -> Result<(), anyhow::Error> {
        


        let running = self.running.clone();
        let mut client = self.create_client();
        let kafka: KafkaPluginConfig = self.kafka.lock().unwrap().clone().unwrap();

        // std::thread::spawn(move || {
        pin_submit!(async move {


            if let Err(err) = client.load_metadata_all() {
                log::info!("load all metadata failed {err}");
            }
    
            let mut builder = Consumer::from_client(client);

    
            if let Some(limit) = kafka.retry_max_bytes_limit {
                builder = builder.with_retry_max_bytes_limit(limit as i32);
            }
    
            if let Some(fallback_offset) = kafka.fallback_offset.clone() {
                let fb_offset = match fallback_offset.to_lowercase().as_str() {
                    "latest" => FetchOffset::Latest,
                    "earliest" => FetchOffset::Earliest,
                    _ => FetchOffset::ByTime(fallback_offset.parse::<i64>().unwrap_or_default())
                };
    
                builder = builder.with_fallback_offset(fb_offset);
            }
    
            if let Some(grp) = kafka.group.clone() {
                builder = builder.with_group(grp);
            }
    
            for topic in kafka.services.iter().filter(|f| f.consume).cloned() {
                if let Some(patts) = topic.partitions {
                    let mut pat = patts.split(',').map(|f| f.parse::<i32>().unwrap_or(0)).collect::<Vec<i32>>();
                    pat.sort();
                    pat.dedup();
                    builder = builder.with_topic_partitions(topic.topic, &pat);
                } else {
                    builder = builder.with_topic(topic.topic);
                }
            }
    
            let mut consumer = match builder.create() {
                Ok(rc) => rc,
                Err(err) => {
                    log::info!("could not create the consumer by this client. {err}");
                    return;
                }
            };

            {
                running.store(true, std::sync::atomic::Ordering::Release);
            }

            loop {
                
                {
                    let still_running = running.load(std::sync::atomic::Ordering::Acquire);
                    if !still_running {
                        log::info!("exit kafka consumer procedure.");
                        return;
                    }
                }

                match consumer.poll() {
                    Ok(msgset) => {
                        // log::info!("received messages.");
                        if msgset.is_empty() {
                            std::thread::sleep(Duration::from_millis(200));
                        }
                        for ms in msgset.iter() {
                            let topic = ms.topic();
                            if let Some(subs) = kafka.get_service(topic) {
                                for m in ms.messages() {
                                    // m.value
                                    match serde_json::from_slice::<Value>(m.value) {
                                        Ok(payload) => {
                                            if let Err(err) = Self::handle_topic_process(&subs, topic, &payload) {
                                                log::error!("process script for {} with error {err}", topic);
                                            }
                                        },
                                        Err(err) => {
                                            log::error!("serde from slice with error. {err}. len of slice is {}", m.value.len());
                                        }
                                    }
                                }
                            }
                            let _ = consumer.consume_messageset(ms);
                        }

                        if kafka.commit_consumer && kafka.group.is_some() {
                            if let Err(err) = consumer.commit_consumed() {
                                log::error!("commit consume with error {err}");
                            }
                        }
                    },
                    Err(err) => {
                       log::info!("received messages failed. {err}");
                    }
                }
            }
        });
        Ok(())
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        // self.client.as_mut().unwrap().disconnect(None);
        // self.receiver.unwrap().close();
        self.running.store(false, std::sync::atomic::Ordering::Release);
    }

    #[allow(dead_code)]
    pub fn publish(&self, topic: &str, payloads: &[Value]) -> Result<(), anyhow::Error> {
        let pro = self.producer.lock().unwrap();
        if pro.is_none() {
            drop(pro);

            let kafka: KafkaPluginConfig = self.kafka.lock().unwrap().clone().unwrap();
            let subs = match kafka.get_service(topic) {
                Some(t) => {
                    if !t.produce {
                        return Err(anyhow!("No producer for {} defained.", topic));
                    }
                    t
                },
                None => {
                    return Err(anyhow!("No producer for {} defained.", topic));
                }
            };

            let required_acks = subs.required_acks.map(|f| match f {
                1 => RequiredAcks::One,
                2 => RequiredAcks::All,
                _ => RequiredAcks::None
            }).unwrap_or(RequiredAcks::None);

            let mut client = self.create_client();
            if let Err(err) = client.load_metadata_all() {
                log::error!("error to load metadata {err}");
            }

            let mut builder = Producer::from_client(client).with_required_acks(required_acks);
            
            if let Some(ack_timeout) = subs.ack_timeout {
                builder = builder.with_ack_timeout(Duration::from_millis(ack_timeout as u64));
            }

            let prod = builder.create()?;
            self.producer.lock().unwrap().replace(prod);
        } else {
            drop(pro);
        }        

        let rec_stash: Vec<Record<'_, (), Trimmed>> = payloads.iter()
                .map(|f| Record::from_value(topic, Trimmed(serde_json::to_string(f).unwrap_or_default())))
                .collect();

        if !rec_stash.is_empty() {
            send_batch(self.producer.clone(), &rec_stash)?;
        }

        Ok(())
    }

    fn to_openapi_doc(&self, ns: &str) -> OpenApi {
        let mut openapi = OpenApi::new(self.conf.name.clone(), "0.1.0");
        let tconf = self.kafka.lock().unwrap().clone();
        if let Some(restconf) = tconf {
            for tsvc in restconf
                .services
                .iter()
                .cloned()
            {
                let topic = tsvc.topic.clone().replace('/', "+");
                let opt = tsvc.to_operation(false);
                let one_path = format!(
                    "/api/kafka/{}/{}/{}/publish",
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

unsafe impl Send for KafkaPluginService {}

unsafe impl Sync for KafkaPluginService {}

impl Drop for KafkaPluginService {
    fn drop(&mut self) {
        self.stop();
    }
}

impl RxPluginService for KafkaPluginService {
    fn invoke_return_option(
        &self,
        uri: InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, anyhow::Error>> + Send>> {
        let topic = uri.method.replace('+', "/");
        log::info!("topic: {topic}");
        match self.publish(&topic, &args) {
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
        match serde_json::to_value(self.kafka.lock().unwrap().clone()) {
            Ok(t) => Some(t),
            Err(err) => {
                log::debug!("Convert to json with error: {:?}", err);
                None
            }
        }
    }

    fn parse_config(&self, val: &Value) -> Result<(), anyhow::Error> {
        match serde_json::from_value::<KafkaPluginConfig>(val.to_owned()) {
            Ok(t) => {
                self.kafka.lock().unwrap().replace(t);
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
            &self.kafka.lock().unwrap().clone(),
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
