use crate::pin_blockon_async;
use crate::utils::global_data::{bool_from_str, i64_from_str, value_from_str};
use anyhow::anyhow;
use chrono::Local;
use rbatis::Page;
use salvo::oapi::{ToParameters, ToSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    any::Any,
    future::Future,
    pin::Pin,
    sync::{atomic::AtomicU64, Arc, Mutex, OnceLock},
};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, ToParameters, ToSchema)]
#[salvo(extract(default_source(from = "body"),))]
pub struct JobStateQuery {
    #[serde(default)]
    #[serde(deserialize_with = "bool_from_str")]
    pub oneshot: Option<bool>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub sched_type: Option<i64>,
    pub uuid: Option<String>,
    pub parent_uuid: Option<String>,
    pub states: Option<String>,
    pub execute_begin: Option<rbatis::rbdc::DateTime>,
    pub execute_end: Option<rbatis::rbdc::DateTime>,
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub page_size: Option<i64>,
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub current: Option<i64>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct JobStateInfo {
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub id: Option<i64>,
    pub parent_uuid: Option<String>,
    pub uuid: Option<String>,
    pub description: Option<String>,
    pub invoke_uri: Option<String>,

    #[serde(default)]
    #[serde(deserialize_with = "value_from_str")]
    pub params: Option<Value>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub sched_type: Option<i64>, // 0: one-shot, 1: cron, 2: repeate
    pub sched_express: Option<String>, // for 1, a cron-express, for 2, a duration of repeates.
    pub states: Option<String>,        // pending, executing, done, failed, done and failed are all
    pub message: Option<String>,       // 出错或提示等
    pub execute_time: Option<chrono::DateTime<Local>>, //  for 0, it's submit time,  for 1, 2, it's the last-execute time

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub elapsed: Option<i64>, // 执行时长，微秒

    #[serde(skip_deserializing)]
    pub times: Arc<AtomicU64>,
}

impl JobStateInfo {
    pub fn new(
        uuid: &str,
        parent_uuid: &Option<String>,
        invk_uri: &str,
        params: &Option<Value>,
        sched_type: i64,
        sched_express: &str,
        desc: &str,
    ) -> Self {
        Self {
            uuid: Some(uuid.to_owned()),
            parent_uuid: parent_uuid.clone(),
            description: Some(desc.to_owned()),
            invoke_uri: Some(invk_uri.to_owned()),
            params: params.clone(),
            sched_type: Some(sched_type),
            sched_express: Some(sched_express.to_owned()),
            states: Some("PENDING".to_string()),
            times: Arc::new(AtomicU64::new(0)),
            ..Default::default()
        }
    }

    pub fn begin_execute(&mut self) {
        self.execute_time = Some(Local::now());
        self.states = Some("EXECUTING".to_string());
        self.times
            .fetch_add(1, std::sync::atomic::Ordering::Release);
    }

    pub fn update_elapsed(&mut self, states: Option<String>, msg: Option<String>) {
        self.states = states;
        self.message = msg;
        if let Some(ts) = self.execute_time {
            let now = Local::now();
            // let now.timestamp_micros()
            let t = now.signed_duration_since(ts).num_microseconds();
            self.elapsed = t;
        } else {
            self.elapsed = Some(0);
        }
    }
}

pub trait JobInvoker {
    fn get_parent_uuid(&self) -> Option<String> {
        None
    }

    fn get_description(&self) -> String;
    fn get_invoke_uri(&self) -> String;
    fn get_invoke_params(&self) -> Option<Value>;
    fn exec(&self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send + 'static>>;
}

pub type FnConvertIntoJobInvoker =
    fn(args: Value) -> Result<(String, Box<dyn JobInvoker + Send + Sync>), anyhow::Error>;

static FUNC_CONVERT_INTO_JOB_INVOKER: OnceLock<FnConvertIntoJobInvoker> = OnceLock::new();

pub fn setup_convert_into_job_invoker(func: FnConvertIntoJobInvoker) {
    let _ = FUNC_CONVERT_INTO_JOB_INVOKER.get_or_init(|| func);
}

pub fn convert_into_job_invoker(
    arg: Value,
) -> Result<(String, Box<dyn JobInvoker + Send + Sync>), anyhow::Error> {
    if let Some(func) = FUNC_CONVERT_INTO_JOB_INVOKER.get() {
        func(arg)
    } else {
        Err(anyhow!(
            "Function of convert into job invoker was not initialized."
        ))
    }
}

pub trait SchedulerManager: Send + Sync {
    fn add_job(
        &self,
        job_id: &str,
        cron: &str,
        duration_sec: Option<u64>,
        run: Box<dyn JobInvoker + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = Result<Uuid, anyhow::Error>> + Send + 'static>> {
        self.add_job_delay(job_id, cron, duration_sec, None, run)
    }

    fn add_job_delay(
        &self,
        job_id: &str,
        cron: &str,
        duration_sec: Option<u64>,
        delay: Option<u64>,
        run: Box<dyn JobInvoker + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = Result<Uuid, anyhow::Error>> + Send + 'static>>;

    fn remove_job(
        &self,
        job_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Uuid, anyhow::Error>> + Send + 'static>>;

    fn query_states(
        &self,
        query: &JobStateQuery,
    ) -> Pin<Box<dyn Future<Output = Result<Page<JobStateInfo>, anyhow::Error>> + Send>>;

    fn start(&self);
}

pub struct SchedulerHolder {
    scheduler: Mutex<Option<&'static dyn SchedulerManager>>,
}

unsafe impl Send for SchedulerHolder {}

unsafe impl Sync for SchedulerHolder {}

impl SchedulerHolder {
    // fn get_() -> &'static mut SchedulerHolder {
    //     static mut SCHEDULER_CTX_HOLDER: MaybeUninit<SchedulerHolder> = MaybeUninit::uninit();
    //     // Once带锁保证只进行一次初始化
    //     static SCHEDULER_CTX_HOLDER_ONCE: Once = Once::new();

    //     SCHEDULER_CTX_HOLDER_ONCE.call_once(|| unsafe {
    //         SCHEDULER_CTX_HOLDER.as_mut_ptr().write(SchedulerHolder {
    //             scheduler: Mutex::new(None),
    //         });
    //     });

    //     unsafe { &mut (*SCHEDULER_CTX_HOLDER.as_mut_ptr()) }
    // }

    pub fn get() -> &'static SchedulerHolder {
        static SCHEDULER_CTX_HOLDER_: OnceLock<SchedulerHolder> = OnceLock::new();
        SCHEDULER_CTX_HOLDER_.get_or_init(|| SchedulerHolder {
            scheduler: Mutex::new(None),
        })
    }

    pub fn update(sch: &'static dyn SchedulerManager) {
        Self::get().scheduler.lock().unwrap().replace(sch);
    }

    pub fn addjob(
        &self,
        jobid: &str,
        cronexpress: &str,
        duration_sec: Option<u64>,
        invoker: Box<dyn JobInvoker + Send + Sync>,
    ) {
        if let Some(ts) = *Self::get().scheduler.lock().unwrap() {
            let job_id = jobid.to_owned();
            let cronxpr = cronexpress.to_owned();
            pin_blockon_async!(async move {
                log::debug!("create a job by {job_id} for {cronxpr}");
                if let Err(err) = ts.add_job(&job_id, &cronxpr, duration_sec, invoker).await {
                    log::info!("error on add job {err}  for {job_id}.");
                }
                Box::new(0) as Box<dyn Any + Send + Sync>
            })
            .unwrap_or(0);
        }
    }

    pub fn addjob_delay(
        &self,
        jobid: &str,
        cronexpress: &str,
        duration_sec: Option<u64>,
        delay: Option<u64>,
        invoker: Box<dyn JobInvoker + Send + Sync>,
    ) {
        if let Some(ts) = *Self::get().scheduler.lock().unwrap() {
            let job_id = jobid.to_owned();
            let cronxpr = cronexpress.to_owned();
            pin_blockon_async!(async move {
                log::debug!("create a job by {job_id} for {cronxpr}");
                if let Err(err) = ts
                    .add_job_delay(&job_id, &cronxpr, duration_sec, delay, invoker)
                    .await
                {
                    log::warn!("error on add job {err}  for {job_id}.");
                }
                Box::new(0) as Box<dyn Any + Send + Sync>
            })
            .unwrap_or(0);
        }
    }

    pub fn removejob(&self, jobid: &str) {
        if let Some(ts) = *Self::get().scheduler.lock().unwrap() {
            let job_id = jobid.to_owned();
            pin_blockon_async!(async move {
                if let Err(err) = ts.remove_job(&job_id).await {
                    log::debug!("error on remove job {err} for {job_id}.");
                }
                Box::new(0) as Box<dyn Any + Send + Sync>
            })
            .unwrap_or(0);
        }
    }

    pub fn query_states(
        &'static self,
        query: JobStateQuery,
    ) -> Pin<Box<dyn Future<Output = Result<Page<JobStateInfo>, anyhow::Error>> + Send>> {
        if let Some(ts) = *Self::get().scheduler.lock().unwrap() {
            // let job_id = jobid.to_owned();
            return Box::pin(async move {
                return ts.query_states(&query).await;
            });
        }
        Box::pin(async { Ok(Page::new_total(0, 20, 0)) })
    }

    pub fn start(&self) {
        if let Some(ts) = *Self::get().scheduler.lock().unwrap() {
            ts.start();
        }
    }
}
