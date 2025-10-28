use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use anyhow::anyhow;
use change_case::{camel_case, pascal_case, snake_case};
use chimes_store_core::dbs::probe::ColumnInfo;
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::sched::{JobInvoker, SchedulerHolder};
use chimes_store_core::service::sdk::InvokeUri;
use chimes_store_core::service::starter::MxStoreService;
use chimes_store_core::utils::redis::redis_set_expire;
use chimes_store_core::utils::{get_local_timestamp, CallbackResult};
use chrono::{DateTime, Local, NaiveDateTime};
use rbatis::rbdc::Uuid;
use reqwest::StatusCode;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::WebConfig;

pub mod change_case;
pub mod zip;

#[cfg(windows)]
mod windows_performance;

#[cfg(not(target_os = "windows"))]
mod linux_performance;

mod performance;
pub use performance::*;

pub mod reqwest_client;
pub mod backwardreader;

pub struct AppConfig {
    web_config: WebConfig,
}

impl AppConfig {
    pub fn get() -> &'static Mutex<AppConfig> {
        // 使用MaybeUninit延迟初始化
        static _CONF: OnceLock<Mutex<AppConfig>> = OnceLock::new();

        _CONF.get_or_init(|| {
            log::warn!("First time init app_config.");
            Mutex::new(AppConfig {
                web_config: WebConfig::default(),
            })
        })
    }

    #[allow(dead_code)]
    pub fn update(webc: &WebConfig) {
        // log::warn!("Update the WebConfig's value.");
        Self::get().lock().unwrap().web_config = webc.clone();
    }

    #[allow(dead_code)]
    pub fn config() -> WebConfig {
        // log::warn!("Get the WebConfig's value.");
        Self::get().lock().unwrap().web_config.clone()
    }
}

#[allow(dead_code)]
pub fn num_to_string(n: i64) -> String {
    let base_codec = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
        'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '2', '3', '4', '5', '7', '8', '9',
    ];
    let len = base_codec.len() as i64;
    let mut t = n;
    let mut result = "".to_string();
    while t > 0 {
        let idx = (t % len) as usize;
        let ch = base_codec[idx];
        t /= len;
        result.insert(0, ch);
    }
    result
}

#[allow(dead_code)]
pub fn number_to_string(n: i64) -> String {
    let base_codec = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'];
    let len = base_codec.len() as i64;
    let mut t = n;
    let mut result = "".to_string();
    while t > 0 {
        let idx = (t % len) as usize;
        let ch = base_codec[idx];
        t /= len;
        result.insert(0, ch);
    }
    result
}

#[allow(dead_code)]
pub fn generate_rand_string(len: usize) -> String {
    let mut retkey = "".to_string();

    while retkey.len() < len {
        let rng = rand::random::<u16>();
        let key = num_to_string(rng as i64);
        retkey += key.as_str();
    }

    retkey.chars().take(len).collect()
}

#[allow(dead_code)]
pub fn generate_rand_numberstring(len: usize) -> String {
    let mut retkey = "".to_string();

    while retkey.len() < len {
        let rng = rand::random::<u16>();
        let key = number_to_string(rng as i64);
        retkey += key.as_str();
    }

    retkey.chars().take(len).collect()
}

#[allow(dead_code)]
pub fn num_to_string_v2(n: i64) -> String {
    let base_codec = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'j', 'k', 'l', 'm', 'n', 'p', 'q', 'r', 's', 't',
        'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M',
        'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '2', '3', '4', '5', '6', '7',
        '8', '9',
    ];
    let len = base_codec.len() as i64;
    let mut t = n;
    let mut result = "".to_string();
    while t > 0 {
        let idx = (t % len) as usize;
        let ch = base_codec[idx];
        t /= len;
        result.insert(0, ch);
    }
    result
}

#[allow(dead_code)]
pub fn generate_rand_string_v2(len: usize) -> String {
    let mut retkey = "".to_string();

    while retkey.len() < len {
        let rng = rand::random::<u16>();
        let key = num_to_string_v2(rng as i64);
        retkey += key.as_str();
    }

    retkey.chars().take(len).collect()
}

#[allow(dead_code)]
pub fn get_date_string() -> String {
    let now = SystemTime::now();
    let date: DateTime<Local> = now.into();
    let fmt = format!("{}", date.format("%Y%m%d"));
    fmt
}

#[allow(dead_code)]
pub fn format_date_string(dt: &NaiveDateTime) -> String {
    let fmt = format!("{}", dt.format("%Y年%m月%d日 %H:%M"));
    fmt
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ManageApiResult<T: ToSchema + 'static> {
    pub status: i32,
    pub message: String,
    pub data: Option<T>,
    pub timestamp: Option<u64>,
}

unsafe impl<T: ToSchema> Send for ManageApiResult<T> {}

unsafe impl<T: ToSchema> Sync for ManageApiResult<T> {}

impl<T: ToSchema> ManageApiResult<T> {
    pub fn ok(dt: T) -> Self {
        ManageApiResult {
            status: 200,
            message: "OK".to_string(),
            data: Option::Some(dt),
            timestamp: Some(get_local_timestamp()),
        }
    }

    pub fn error(code: i32, msg: &str) -> Self {
        ManageApiResult {
            status: code,
            message: msg.to_owned(),
            data: None,
            timestamp: Some(get_local_timestamp()),
        }
    }

    #[allow(dead_code)]
    pub fn new(code: i32, msg: &str, data: T, ts: u64) -> Self {
        ManageApiResult {
            status: code,
            message: msg.to_owned(),
            data: Some(data),
            timestamp: Some(ts),
        }
    }
}

pub fn mixup_case(_name: &str) -> String {
    let cp = generate_rand_string_v2(8);
    let chs = cp.as_bytes();
    if chs[0] < b'a' || chs[0] > b'z' {
        format!("p{}_", cp.to_lowercase())
    } else {
        format!("{}_", cp.to_lowercase())
    }
}

pub fn naming_property(col: &ColumnInfo, rule: &str) -> Option<String> {
    if let Some(col_name) = col.column_name.clone() {
        match rule {
            "camelcase" => Some(camel_case(&col_name.to_lowercase())),
            "snakecase" => Some(snake_case(&col_name.to_lowercase())),
            "pascalcase" => Some(pascal_case(&col_name.to_lowercase())),
            "mixup" => Some(mixup_case(&col_name)),
            _ => Some(col_name),
        }
    } else {
        None
    }
}

/**
 * 异步请求，用于执行长时间处理的请求式任务
 * 构造一个请求/查询的API，以及请求/回调的API请求模式
 * 在该请求中，我们通过标准的URI接口方式来进行操作，并向其传递相应的参数，由于任务可能需要执行比较长的时间，
 * 如生成批量数据，调用外部长时间的接口等，这个时候需要我们将这些任务转换成短时间能处理的调用方式来提升系统性能
 */
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AsyncRequest {
    pub namespace: Option<String>, // 指定一个命名空间来调用该URI接口，如果为空值，则使用URI中标识的Namespace
    pub uri: String,
    pub return_type: RequestReturnType,
    pub params: Option<Value>,
    pub callback: Option<String>, // callback should be called by POST
    pub uuid: Option<String>,
}

unsafe impl Send for AsyncRequest {}

unsafe impl Sync for AsyncRequest {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum RequestReturnType {
    Option,
    List,
    Page,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AsyncQuery {
    pub namespace: String, // 指定一个命名空间来调用该URI接口，如果为空值，则使用URI中标识的Namespace
    pub uuid: String,      // 被查询的UUID标识，UUID所保存的时间为5分钟
}

impl JobInvoker for AsyncRequest {
    fn exec(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), anyhow::Error>> + Send + 'static>,
    > {
        let uri = self.uri.clone();
        let params = self.params.clone();
        let ctx = Arc::new(Mutex::new(InvocationContext::new()));
        let ret_type = self.return_type.clone();
        let callback = self.callback.clone();
        let iuri = InvokeUri::parse(&uri).unwrap();
        let ns = self.namespace.clone().unwrap_or(iuri.namespace);
        let uuid = self.uuid.clone().unwrap();
        let delay_on_error = 5u64; // first time is 5 second
        let param_vec = match params {
            None => vec![],
            Some(val) => match val {
                Value::Array(t) => t,
                _ => vec![val],
            },
        };

        Box::pin(async move {
            let result = match ret_type {
                RequestReturnType::Option => {
                    MxStoreService::invoke_return_one(uri, ctx, param_vec).await
                }
                RequestReturnType::List => MxStoreService::invoke_return_vec(uri, ctx, param_vec)
                    .await
                    .map(|r| Some(Value::Array(r))),
                RequestReturnType::Page => MxStoreService::invoke_return_page(uri, ctx, param_vec)
                    .await
                    .map(|r| Some(json!(r))),
            };
            let apiret = result
                .map(|r| CallbackResult::ok(&ns, &uuid, r))
                .unwrap_or_else(|e| {
                    CallbackResult::<Option<Value>>::error(&ns, &uuid, 500, &format!("{e}"))
                });
            // call the callback api
            let key = format!("{}-{}", &ns, &uuid);
            let ret_text = serde_json::to_string(&apiret).unwrap_or_default();
            let _ = redis_set_expire(&ns, &key, &ret_text, 300).is_ok();
            if let Some(callback_url) = callback {
                let newjobid = Uuid::new().to_string();
                let newdelay = DelayCallbackRequest {
                    parent_uuid: uuid.clone(),
                    callback: callback_url.clone(),
                    delay: 5,
                    body: Some(ret_text),
                };
                SchedulerHolder::get().addjob_delay(
                    &newjobid,
                    "",
                    None,
                    Some(delay_on_error),
                    Box::new(newdelay),
                );
            }

            Ok(())
        })
    }

    fn get_description(&self) -> String {
        format!("执行长时间任务: {}", self.uri.clone())
    }

    fn get_invoke_uri(&self) -> String {
        self.uri.clone()
    }

    fn get_invoke_params(&self) -> Option<Value> {
        self.params.clone()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DelayCallbackRequest {
    pub parent_uuid: String,
    pub callback: String,
    pub delay: u64,
    pub body: Option<String>,
}

impl JobInvoker for DelayCallbackRequest {
    fn exec(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), anyhow::Error>> + Send + 'static>,
    > {
        let callback_url = self.callback.clone();
        let ret_text = self.body.clone().unwrap_or_default();
        let delay = self.delay;
        let parent_task_id = self.parent_uuid.clone();
        Box::pin(async move {
            match reqwest::Client::new()
                .post(&callback_url)
                .body(ret_text.clone())
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status() == StatusCode::OK {
                        log::info!("Callback {callback_url} was called successfully.");
                        Ok(())
                    } else {
                        log::info!(
                            "Callback {callback_url} was called failed. It will be called delay."
                        );
                        if delay < 360000 {
                            let jobid = Uuid::new().to_string();
                            let newdelay = DelayCallbackRequest {
                                parent_uuid: parent_task_id.clone(),
                                callback: callback_url.clone(),
                                delay: delay * delay,
                                body: Some(ret_text),
                            };
                            SchedulerHolder::get().addjob_delay(
                                &jobid,
                                "",
                                None,
                                Some(newdelay.delay),
                                Box::new(newdelay),
                            );
                        }
                        Err(anyhow!("retry callback delay {}", delay))
                    }
                }
                Err(err) => {
                    log::info!("Callback {callback_url} was called failed with error {err:?}.");
                    if delay < 360000 {
                        let jobid = Uuid::new().to_string();
                        let newdelay = DelayCallbackRequest {
                            parent_uuid: parent_task_id.clone(),
                            callback: callback_url.clone(),
                            delay: delay * delay,
                            body: Some(ret_text),
                        };
                        SchedulerHolder::get().addjob_delay(
                            &jobid,
                            "",
                            None,
                            Some(newdelay.delay),
                            Box::new(newdelay),
                        );
                    }
                    Err(anyhow!("retry callback delay {}", delay))
                }
            }
        })
    }

    fn get_parent_uuid(&self) -> Option<String> {
        if self.parent_uuid.is_empty() {
            None
        } else {
            Some(self.parent_uuid.clone())
        }
    }

    fn get_description(&self) -> String {
        format!("延迟{}秒执行回调{}任务", self.delay, self.callback.clone())
    }

    fn get_invoke_uri(&self) -> String {
        self.callback.clone()
    }

    fn get_invoke_params(&self) -> Option<Value> {
        self.body
            .clone()
            .map(|s| serde_json::from_str::<Value>(&s).unwrap_or(Value::Null))
    }
}
