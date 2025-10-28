use anyhow::anyhow;
use chimes_store_core::config::PluginConfig;
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::queue::{SyncTaskQueue, SyncWriter};
use chimes_store_core::service::sched::SchedulerHolder;
use chimes_store_core::service::sdk::{InvokeUri, MethodDescription, RxPluginService};
use chimes_store_core::service::starter::load_config;
use chimes_store_core::utils::global_data::i64_from_str;
use chimes_store_utils::algorithm::md5_hash;
use rbatis::rbdc::DateTime;
use rbatis::Page;
use salvo::oapi::{
    schema, Array, Content, Object, OpenApi, Operation, PathItem, RefOr, RequestBody, Response,
    Schema,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt::Display;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use super::fetch::{GeneralStoreUriReader, SyncReader};
use super::filefetch::FileUriReader;
use super::write::{start_write_thread, GeneralStoreUriWriter};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SyncTaskVariableDefine {
    pub var_name: Option<String>,
    pub var_type: Option<String>,
    pub var_write: Option<String>,
    pub var_value: Option<String>,
    pub var_data_express: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SyncTaskDefinition {
    pub task_id: String,              // Task ID
    pub cron_express: Option<String>, // 定时获取的表达式

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub interval_second: Option<i64>, // 定时获取的表达式

    pub source_uri: Option<String>, // 执行数据获取的源，如果是文件类型，则应该以file://开头，后面为具体的路径
    pub check_delete: Option<String>, // 执行数据检查标识，通过检查，说明该数据应该执行删除操作
    pub source_request: Option<String>, // 如果 source_uri是file://开头的，这个地方应为一个JSON描述的对象，
    // {"parser": "jsonl", "remove_source": true, "filename_pattern": "store_*.log", "recuise": true}
    // 以此来使用对应的解释器来解释文件
    #[serde(default)]
    pub paged_request: bool,

    #[serde(default)]
    pub no_source: bool,

    #[serde(default)]
    pub update_variable_epoch: bool,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub page_size: Option<i64>,
    pub target_uri: Option<String>, // 执行数据落库的动作，当收到的数据的state=1时，执行该动作
    pub remove_uri: Option<String>, // 执行数据落删除的动作，当收到的数据的state=2时，执行该动作
    pub lang: Option<String>, // 执行数据转换时的脚本语言，缺省为Tera，如果为Tera，则会通过模板的方式来重新重成JSON对象，如果rhai，则由脚本引擎来实现之
    pub transform: Option<String>, // 执行数据转换处理的动作
    pub params: Option<String>, // 执行数据转换处理动作时所需要的一些额外参数，以JSON表示
    pub task_desc: Option<String>,
    pub check_pattern: Option<String>, // check the result was successfully
}

fn to_api_result_schema(t: RefOr<Schema>, array: bool) -> schema::Schema {
    let mut apiresult = Object::new();
    apiresult = apiresult.property(
        "status",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "message",
        Object::new().schema_type(schema::BasicType::String),
    );
    if array {
        apiresult = apiresult.property("data", Array::new().items(t));
    } else {
        apiresult = apiresult.property("data", t);
    }
    apiresult = apiresult.property(
        "timestamp",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    schema::Schema::Object(Box::new(apiresult))
}

impl SyncTaskDefinition {
    pub(crate) fn to_operation(&self, array: bool) -> Operation {
        let mut ins_op = Operation::new();
        ins_op = ins_op.request_body(RequestBody::new().add_content(
            "application/json",
            RefOr::Type(Schema::Object(Box::new(Object::new()))),
        ));
        ins_op = ins_op.summary(self.task_desc.clone().unwrap_or_default());

        let mut description = "方法在执行的时候会根据URL中所传递的Query参数，组装成第一个参数（根据参数名转成JSON Object），从Request Body接收第二个参数（必须为JSON对象），模板中须按这个规则来处理对应的参数。".to_string();
        description.push_str("在请求模板中需要使用args[0]来访问Query String传递过来的参数，使用args[1]来访问Request Body传递过来的参数。");
        description.push_str(&format!("该方法将请求{}接口。", self.task_id.clone()));

        ins_op = ins_op.description(description);

        let mut resp = Response::new(format!(
            "返回{}接口请求成功后处理的数据。",
            self.task_id.clone()
        ));
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(
                RefOr::Type(schema::Schema::Object(Box::new(Object::new()))),
                array,
            )),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));
        ins_op
    }
}

pub enum VariableValue {
    String(String),
    Long(i64),
    Double(f64),
    Date(DateTime),
    Bool(bool),
    Null,
}

impl Display for VariableValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            VariableValue::String(text) => text.to_owned(),
            VariableValue::Long(val) => val.to_string(),
            VariableValue::Double(val) => val.to_string(),
            VariableValue::Date(dt) => dt.format("YYYY-MM-DD hh:mm:ss.000000+00:00"),
            VariableValue::Bool(bl) => bl.to_string(),
            VariableValue::Null => String::new(),
        };

        f.write_str(&text)
    }
}

impl VariableValue {
    #[allow(dead_code)]
    pub fn to_json_value(&self) -> Value {
        match self {
            VariableValue::String(text) => Value::String(text.to_owned()),
            VariableValue::Long(val) => json!(val.to_owned()),
            VariableValue::Double(val) => json!(val.to_owned()),
            VariableValue::Date(dt) => {
                let dtfmt = dt.format("YYYY-MM-DD hh:mm:ss.000000+00:00");
                Value::String(dtfmt)
            }
            VariableValue::Bool(bl) => Value::Bool(bl.to_owned()),
            VariableValue::Null => Value::Null,
        }
    }

    pub fn from_json_value(
        json: Value,
        maybedate: bool,
        maybefloat: bool,
        maybeinter: bool,
    ) -> Self {
        match json {
            Value::Null => Self::Null,
            Value::Bool(bl) => Self::Bool(bl),
            Value::Number(num) => {
                if num.is_f64() {
                    Self::Double(num.as_f64().unwrap_or_default())
                } else {
                    Self::Long(num.as_i64().unwrap_or_default())
                }
            }
            Value::String(text) => {
                if maybedate {
                    let fmt1 = "YYYY-MM-DD hh:mm:ss.000000+00:00";
                    let fmt2 = "YYYY-MM-DD hh:mm:ss.000000";
                    let fmt3 = "YYYY-MM-DD hh:mm:ss";
                    let fmt4 = "YYYY-MM-DD";

                    if let Ok(dt) = DateTime::parse(fmt1, &text) {
                        Self::Date(dt)
                    } else if let Ok(dt) = DateTime::parse(fmt2, &text) {
                        Self::Date(dt)
                    } else if let Ok(dt) = DateTime::parse(fmt3, &text) {
                        Self::Date(dt)
                    } else if let Ok(dt) = DateTime::parse(fmt4, &text) {
                        Self::Date(dt)
                    } else {
                        Self::Null
                    }
                } else if maybefloat {
                    Self::Double(text.parse::<f64>().unwrap_or_default())
                } else if maybeinter {
                    Self::Long(text.parse::<i64>().unwrap_or_default())
                } else {
                    Self::String(text)
                }
            }
            Value::Array(_) => VariableValue::Null,
            Value::Object(_) => VariableValue::Null,
        }
    }
}

impl PartialOrd for VariableValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::String(l0), Self::String(r0)) => l0.partial_cmp(r0),
            (Self::Long(l0), Self::Long(r0)) => l0.partial_cmp(r0),
            (Self::Double(l0), Self::Double(r0)) => l0.partial_cmp(r0),
            (Self::Date(l0), Self::Date(r0)) => l0.partial_cmp(r0),
            (Self::Bool(l0), Self::Bool(r0)) => l0.partial_cmp(r0),
            _ => {
                if core::mem::discriminant(self) == core::mem::discriminant(other) {
                    Some(std::cmp::Ordering::Equal)
                } else {
                    None
                }
            }
        }
    }
}

// impl Ord for VariableValue {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         match (self, other) {
//             (Self::String(l0), Self::String(r0)) => l0.cmp(r0),
//             (Self::Long(l0), Self::Long(r0)) => l0.cmp(r0),
//             (Self::Double(l0), Self::Double(r0)) => if l0 == r0 {
//                 std::cmp::Ordering::Equal
//             } else if l0 > r0 {
//                 std::cmp::Ordering::Greater
//             } else {
//                 std::cmp::Ordering::Less
//             },
//             (Self::Date(l0), Self::Date(r0)) => l0.cmp(r0),
//             (Self::Bool(l0), Self::Bool(r0)) => l0.cmp(r0),
//             _ => if core::mem::discriminant(self) == core::mem::discriminant(other) {
//                 std::cmp::Ordering::Equal
//             } else {
//                 std::cmp::Ordering::Greater
//             },
//         }
//     }
// }

impl PartialEq for VariableValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::Long(l0), Self::Long(r0)) => l0 == r0,
            (Self::Double(l0), Self::Double(r0)) => l0 == r0,
            (Self::Date(l0), Self::Date(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SyncVariable {
    pub var_name: Option<String>,  // 变量名称
    pub var_type: Option<String>,  // 变量类型
    pub var_value: Option<String>, // 变量当前值
    pub var_write: Option<String>, // 变量写入逻辑，CURRENT_TIME, CURRENT_DATE, MAX, MIN, SQL, InvokeURI
    pub var_data_express: Option<String>,
}

impl SyncVariable {
    pub fn to_default_value(&self) -> Value {
        let vartype = self.var_type.clone().unwrap_or_default().to_lowercase();
        let varval = self.var_value.clone().unwrap_or_default();
        // let vartx = self.var_write.clone().unwrap_or_default().to_uppercase();
        // 对于日期/日期时间，要么是一个有效的日期/日期时间，这样可以解释出对应的值来，
        // 否则，就都是当前日期或当前日期时间
        // 对于bool，则可以填写yes/true表示为true，其它都为false
        match vartype.as_str() {
            "number" => {
                let val = varval.parse::<i64>().unwrap_or_default();
                json!(val)
            }
            "float" | "double" | "f64" => {
                let val = varval.parse::<f64>().unwrap_or_default();
                json!(val)
            }
            "date" => match rbatis::rbdc::Date::from_str(&varval) {
                Ok(ts) => {
                    json!(ts)
                }
                Err(_) => {
                    let dtnow = rbatis::rbdc::DateTime::now();
                    json!(rbatis::rbdc::Date::from(dtnow.0))
                }
            },
            "datetime" => match rbatis::rbdc::DateTime::from_str(&varval) {
                Ok(ts) => {
                    json!(ts)
                }
                Err(_) => {
                    json!(rbatis::rbdc::DateTime::now())
                }
            },
            "boolean" | "bool" => {
                let bl = varval.to_lowercase() == *"true" || varval.to_lowercase() == *"yes";
                Value::Bool(bl)
            }
            _ => Value::String(varval),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SyncTaskPluginConfig {
    pub store_uri: Option<String>, // 与之关联的用于管理同步的几个表的定义的URI，如果该值没有填写，则应该为与之相同的namespace
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub write_threads: Option<i64>, // 执行写库操作的线程数
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub interval: Option<i64>, // 轮询间隔时间

    #[serde(default)]
    pub global_variable: bool, // 是否使用全局管理的变量
    pub variable_opt_url: Option<String>, // 执行Variable操作的URI，应该为Object, 因为Object支持CRUD。 object://ns/GlobalVariable这样的URI
    #[serde(default)]
    pub variables: Vec<SyncVariable>, // 执行时的一些变量配置
    #[serde(default)]
    pub services: Vec<SyncTaskDefinition>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FetchMode {
    PerThread,    // 为每一个获取任务开启一个线程
    SingleThread, // 只用一个线程来轮询
}

#[allow(dead_code)]
pub struct SyncTaskPluginService {
    namespace: String,
    conf: PluginConfig,
    synctask: Mutex<Option<SyncTaskPluginConfig>>,
    running: Arc<AtomicBool>,
}

impl SyncTaskPluginService {
    pub fn new(ns: &str, conf: &PluginConfig) -> Result<Self, anyhow::Error> {
        log::debug!("Plugin config load from {}", conf.config.clone());
        let t = match load_config(conf.config.clone()) {
            Ok(r) => r,
            Err(err) => {
                log::debug!("Could not load the config file: {err:?}");
                Some(SyncTaskPluginConfig::default())
            }
        };

        Ok(Self {
            namespace: ns.to_owned(),
            conf: conf.to_owned(),
            synctask: Mutex::new(t),
            running: Arc::new(AtomicBool::new(false)),
        })
    }

    fn to_openapi_doc(&self, ns: &str) -> OpenApi {
        let mut openapi = OpenApi::new(self.conf.name.clone(), "0.1.0");
        let tconf = self.synctask.lock().unwrap().clone();
        if let Some(restconf) = tconf {
            for tsvc in restconf.services.iter().cloned() {
                let opt = tsvc.to_operation(false);
                let one_path = format!(
                    "/api/synctask/{}/{}/{}/single",
                    ns,
                    self.conf.name.clone(),
                    tsvc.task_id.clone()
                );
                openapi = openapi.add_path(
                    one_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                );
                openapi = openapi.add_path(
                    one_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Get, opt.clone()),
                );

                let opt = tsvc.to_operation(true);
                let list_path = format!(
                    "/api/restapi/{}/{}/{}/list",
                    ns,
                    self.conf.name.clone(),
                    tsvc.task_id.clone()
                );
                openapi = openapi.add_path(
                    list_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                );
                openapi = openapi.add_path(
                    list_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Get, opt.clone()),
                );
            }
            for tsvc in restconf.services.iter().cloned() {
                let opt = tsvc.to_operation(false);
                let one_path = format!(
                    "/api/passoff/restapi/{}/{}/{}/single",
                    ns,
                    self.conf.name.clone(),
                    tsvc.task_id.clone()
                );
                openapi = openapi.add_path(
                    one_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                );
                openapi = openapi.add_path(
                    one_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Get, opt.clone()),
                );

                let opt = tsvc.to_operation(true);
                let list_path = format!(
                    "/api/passoff/restapi/{}/{}/{}/list",
                    ns,
                    self.conf.name.clone(),
                    tsvc.task_id.clone()
                );
                openapi = openapi.add_path(
                    list_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                );
                openapi = openapi.add_path(
                    list_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Get, opt.clone()),
                );
            }
        }
        openapi
    }

    pub fn get_synctasks(&self) -> Option<SyncTaskPluginConfig> {
        self.synctask.lock().unwrap().clone()
    }

    pub fn stop(&self) {
        log::error!("SyncTaskPluginService was stop.");
        self.running
            .store(false, std::sync::atomic::Ordering::Release);
        self.removejobs();
        SyncTaskQueue::get_mut().notify_all();
    }

    pub fn removejobs(&self) {
        let synctask = self.get_synctasks();

        if let Some(tasks) = synctask {
            // 创建Writer
            for svc in tasks.services.clone() {
                let taskid = svc.task_id.clone();
                log::error!("remove the task writer for {taskid}");
                SyncTaskQueue::get_mut().remove_writer(&taskid);
            }

            // 创建Fetcher，并根据需要加入到定时任务列表，或启动一个独立的线程来建立与之相关联的事件响应线程
            for svc in tasks.services.clone() {
                if !svc.no_source {
                    let taskid = svc.task_id.clone();
                    log::error!("remove the job for {taskid}");
                    let jobid =
                        format!("{taskid}-{}", md5_hash(&svc.source_uri.unwrap_or_default()));
                    SchedulerHolder::get().removejob(&jobid);
                }
            }
        }
    }

    pub fn start_fetch(&self) {
        let synctask = self.get_synctasks();
        log::error!("now to start fetch by each task.");
        if let Some(tasks) = synctask {
            // 创建Writer
            for svc in tasks.services.clone() {
                let taskid = svc.task_id.clone();
                if let Some(taskwriter) = self.create_writer(&svc) {
                    SyncTaskQueue::get_mut().add_writer(&taskid, taskwriter);
                }
            }

            // 创建Fetcher，并根据需要加入到定时任务列表，或启动一个独立的线程来建立与之相关联的事件响应线程
            for svc in tasks.services.clone() {
                if !svc.no_source {
                    let taskid = svc.task_id.clone();

                    log::info!("create fetcher for {taskid}");
                    if let Some(taskfetcher) = self.create_fetcher(&svc) {
                        // 这个fetcher是cron定时任务
                        let jobid =
                            format!("{taskid}-{}", md5_hash(&svc.source_uri.unwrap_or_default()));

                        if let Some(interval) = taskfetcher.interval_second() {
                            let inv = taskfetcher.to_invoker();
                            log::warn!("the job should be repeated by {interval} sec.");
                            SchedulerHolder::get().addjob(&jobid, "", Some(interval), inv);
                        } else if let Some(cron) = taskfetcher.cron_express() {
                            log::warn!("the job should be invoked {cron}");
                            let inv = taskfetcher.to_invoker();
                            SchedulerHolder::get().addjob(&jobid, &cron, None, inv);
                        }
                    }
                }
            }
            start_write_thread(self.running.clone());
        }
    }

    pub fn create_writer(&self, task: &SyncTaskDefinition) -> Option<Box<dyn SyncWriter>> {
        Some(Box::new(GeneralStoreUriWriter::new(&self.namespace, task)))
    }

    pub fn create_fetcher(&self, task: &SyncTaskDefinition) -> Option<Box<dyn SyncReader>> {
        let cps = self.synctask.lock().unwrap().clone().unwrap();
        let request_uri = task.source_uri.clone().unwrap_or_default();
        if request_uri.starts_with("file://") {
            log::debug!("Create th FileUriReader for {request_uri}");
            let fsr = FileUriReader::new(&self.namespace, &cps.variables, task);
            Some(Box::new(fsr))
        } else {
            let gsr = GeneralStoreUriReader::new(&self.namespace, &cps.variables, task);
            Some(Box::new(gsr))
        }
    }
}

unsafe impl Send for SyncTaskPluginService {}

unsafe impl Sync for SyncTaskPluginService {}

impl RxPluginService for SyncTaskPluginService {
    fn invoke_return_option(
        &self,
        _uri: InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, anyhow::Error>> + Send>> {
        Box::pin(async move { Ok(None) })
    }

    fn invoke_return_vec(
        &self,
        _uri: InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, anyhow::Error>> + Send>> {
        Box::pin(async move { Ok(vec![]) })
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
        match serde_json::to_value(self.synctask.lock().unwrap().clone()) {
            Ok(t) => Some(t),
            Err(err) => {
                log::debug!("Convert to json with error: {err:?}");
                None
            }
        }
    }

    fn parse_config(&self, val: &Value) -> Result<(), anyhow::Error> {
        match serde_json::from_value::<SyncTaskPluginConfig>(val.to_owned()) {
            Ok(t) => {
                self.synctask.lock().unwrap().replace(t);
                Ok(())
            }
            Err(err) => {
                log::error!("Parse JSON value to config with error: {err:?}");
                Err(anyhow!(err))
            }
        }
    }

    fn add_service(&self, services: Vec<Value>) -> Result<(), anyhow::Error> {
        let st = services
            .iter()
            .map(|f| serde_json::from_value::<SyncTaskDefinition>(f.to_owned()).unwrap_or_default())
            .filter(|p| !p.task_id.is_empty())
            .collect::<Vec<SyncTaskDefinition>>();

        let st_varbs = services
            .iter()
            .map(|f| {
                serde_json::from_value::<SyncTaskVariableDefine>(f.to_owned()).unwrap_or_default()
            })
            .filter(|p| {
                p.var_name.is_some()
                    && p.var_name != Some(String::new())
                    && p.var_type.is_some()
                    && p.var_type != Some(String::new())
                    && p.var_write.is_some()
                    && p.var_write != Some(String::new())
            })
            .collect::<Vec<SyncTaskVariableDefine>>();

        let mut config_ = self.synctask.lock().unwrap().clone().unwrap();

        st_varbs
            .iter()
            .map(|v| SyncVariable {
                var_name: v.var_name.clone(),
                var_type: v.var_type.clone(),
                var_value: v.var_value.clone(),
                var_write: v.var_write.clone(),
                var_data_express: v.var_data_express.clone(),
            })
            .for_each(|mut v| {
                if let Some(idx) = config_
                    .variables
                    .iter()
                    .position(|f| f.var_name == v.var_name)
                {
                    let xt = config_.variables.remove(idx);
                    v.var_value = xt.var_value.clone();
                }
                config_.variables.push(v)
            });

        for t in st {
            // log::info!("Task: {}", serde_json::to_string_pretty(&t).unwrap_or_default());
            match config_
                .services
                .iter()
                .position(|cp| cp.task_id == t.task_id)
            {
                Some(idx) => {
                    config_.services.remove(idx);
                    config_.services.push(t);
                }
                None => {
                    config_.services.push(t);
                }
            }
        }

        *self.synctask.lock().unwrap() = Some(config_);

        Ok(())
    }

    fn save_config(&self, conf: &PluginConfig) -> Result<(), anyhow::Error> {
        let path: PathBuf = conf.config.clone().into();
        if let Some(conf_) = self.synctask.lock().unwrap().clone() {
            // log::info!("Save sync task: {}", serde_json::to_string_pretty(&conf_).unwrap_or_default());
            chimes_store_core::service::starter::save_config(&conf_, path)
        } else {
            Ok(())
        }
    }

    fn get_metadata(&self) -> Vec<chimes_store_core::service::sdk::MethodDescription> {
        vec![MethodDescription {
            uri: "synctask://{ns}/{name}".to_owned(),
            name: "exchange".to_owned(),
            func: None,
            params_vec: true,
            params1: vec![],
            params2: None,
            response: vec![],
            return_page: true,
            return_vec: false,
        }]
    }

    fn get_openapi(&self, ns: &str) -> Box<dyn std::any::Any> {
        Box::new(self.to_openapi_doc(ns))
    }

    fn shutdown_plugin(&self) -> Result<(), anyhow::Error> {
        self.stop();
        Ok(())
    }
}
