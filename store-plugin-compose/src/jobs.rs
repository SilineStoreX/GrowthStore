use crate::proc::{invoke_shell_script, ComposeServiceInfo};
use anyhow::anyhow;
use chimes_store_core::{
    config::{auth::JwtUserClaims, ConditionItem, IPaging, OrdianlItem, QueryCondition},
    pin_async_process, pin_blockon_async,
    service::{
        invoker::InvocationContext,
        sched::{JobInvoker, JobStateInfo, JobStateQuery, SchedulerManager},
        sdk::InvokeUri,
        starter::MxStoreService,
    },
    utils::redis::{redis_del, redis_lock_expire_nx},
    SyncUnsafeCell,
};
use rbatis::Page;
use serde_json::{json, Value};
use std::{
    any::Any,
    collections::{hash_map::Entry, HashMap},
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

struct ComposeJobInvoker {
    uuid: String,
    uri: String,
    simulator: String,
    desc: String,
}

impl JobInvoker for ComposeJobInvoker {
    fn exec(&self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send + 'static>> {
        let cs_simulate = self.simulator.clone();
        let inv_full_uri = self.uri.clone();
        let uuid = self.uuid.clone();
        log::info!("Uuid {} to be scheduled.", self.uuid);
        Box::pin(async move {
            // CronSchedulerHolder::update_begin(&uuid).await;
            log::info!("trigger to invoke the time ComposeJobInvoker .....");
            let ctx = Arc::new(Mutex::new(InvocationContext::new_userclaims(
                JwtUserClaims::username(&cs_simulate),
            )));
            match MxStoreService::invoke_return_one(inv_full_uri.clone(), ctx, vec![]).await {
                Ok(_) => {
                    log::info!("Compose Job {uuid} executed successfully.");
                    // CronSchedulerHolder::update_completed(&uuid, "SUCCESS", None).await;
                    Ok(())
                }
                Err(err) => {
                    //log::error!("schedule execute on error {:?}", err);
                    log::error!("Compose Job {uuid} execute failed by error {err:?}");
                    //CronSchedulerHolder::update_completed(&uuid, "FAILED", Some(err.to_string()))
                    //    .await;
                    Err(err)
                }
            }
        })
    }

    fn get_invoke_uri(&self) -> String {
        self.uri.clone()
    }

    fn get_invoke_params(&self) -> Option<Value> {
        None
    }

    fn get_description(&self) -> String {
        self.desc.clone()
    }
}

struct ProxyJobInvoker(String);

impl ProxyJobInvoker {
    pub fn exec(
        &self,
        runner: Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send + 'static>>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
        let uuid = self.0.clone();
        log::info!("Uuid {} to be scheduled.", self.0);
        Box::pin(async move {
            if let Err(err) = CronSchedulerHolder::lock_jobuuid(&uuid) {
                log::info!("Could not lock the job {uuid} by error {err}");
                return;
            }

            CronSchedulerHolder::update_begin(&uuid).await;
            log::info!("trigger to invoke the time ProxyJobInvoker .....");
            match runner.await {
                Ok(_) => {
                    CronSchedulerHolder::update_completed(&uuid, "SUCCESS", None).await;
                    log::info!("Proxy Job {uuid} executed successfully.");
                }
                Err(err) => {
                    log::error!("Proxy Job {uuid} execute failed by error {err:?}");
                    CronSchedulerHolder::update_completed(&uuid, "FAILED", Some(err.to_string()))
                        .await;
                }
            }

            if let Err(err) = CronSchedulerHolder::release_jobuuid(&uuid) {
                log::info!("Could not release the lock {uuid} by error {err}");
            }
        })
    }
}

pub(crate) struct CronSchedulerHolder {
    pub(crate) sched: Option<JobScheduler>,
    job_invoke_uri: Option<String>,
    job_map: HashMap<String, Uuid>,
    job_others: HashMap<String, Uuid>,
    job_params: HashMap<String, JobStateInfo>,
}

impl CronSchedulerHolder {
    fn new() -> CronSchedulerHolder {
        CronSchedulerHolder {
            sched: None,
            job_invoke_uri: None,
            job_map: HashMap::new(),
            job_others: HashMap::new(),
            job_params: HashMap::new(),
        }
    }

    pub(crate) fn get_() -> &'static mut CronSchedulerHolder {
        // 使用MaybeUninit延迟初始化
        static SCHEDULER_HOLDER_ON: OnceLock<SyncUnsafeCell<CronSchedulerHolder>> = OnceLock::new();

        unsafe {
            let ss =
                SCHEDULER_HOLDER_ON.get_or_init(|| SyncUnsafeCell::new(CronSchedulerHolder::new()));
            &mut *ss.get()
            // SCHEDULER_HOLDER_ON.get_mut().unwrap().0.get_mut()
        }
    }

    /**
     * 设置用于管理任务执行状态的URI
     * 通过这个状态管理，可以查询Repeated和Cron任务执行的次数，以及每次执行的情况
     * 通过这个状态管理可以查询oneshot任务的执行状态
     */
    pub fn set_states_mangement_uri(uri: &Option<String>) {
        Self::get_().job_invoke_uri = uri.clone();
    }

    pub fn update(sched: Option<JobScheduler>) {
        Self::get_().sched = sched;
    }

    pub fn get() -> &'static CronSchedulerHolder {
        Self::get_()
    }

    #[allow(dead_code)]
    fn get_state_management_uri() -> Option<String> {
        Self::get_().job_invoke_uri.clone()
    }

    fn lock_jobuuid(uuid: &str) -> Result<(), anyhow::Error> {
        if let Some(state_invoke_uri) = Self::get().job_invoke_uri.clone() {
            let uri = InvokeUri::parse(&state_invoke_uri)?;
            let ns = uri.namespace;
            let _ = redis_lock_expire_nx(&ns, uuid, uuid, 0, Some(true))?;
            Ok(())
        } else {
            Ok(())
        }
    }

    fn release_jobuuid(uuid: &str) -> Result<(), anyhow::Error> {
        if let Some(state_invoke_uri) = Self::get().job_invoke_uri.clone() {
            let uri = InvokeUri::parse(&state_invoke_uri)?;
            let ns = uri.namespace;
            let _ = redis_del(&ns, uuid)?;
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn job_execute(
        uuid: String,
        run: Arc<Box<dyn JobInvoker + Send + Sync>>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
        Box::pin(async move {
            if let Err(err) = Self::lock_jobuuid(&uuid) {
                log::info!("Could not get the lock for {uuid} by error {err}");
                return;
            }

            Self::update_begin(&uuid).await;
            match run.exec().await {
                Ok(_) => {
                    log::warn!("Job Invoked Successfully by CronSchedulerHolder.");
                    Self::update_completed(&uuid, "SUCCESS", None).await;
                }
                Err(err) => {
                    log::error!("executing the task {uuid} by error  {err}");
                    Self::update_completed(&uuid, "FAILED", Some(err.to_string())).await;
                }
            }

            if let Err(err) = Self::release_jobuuid(&uuid) {
                log::info!("Could not release lock for {uuid} by error {err}");
            }
        })
    }

    #[allow(dead_code)]
    pub async fn init() {
        match &mut Self::get_().sched {
            Some(t) => {
                if !t.inited().await {
                    if let Err(err) = t.init().await {
                        log::info!("JobScheduler start error {err:?}");
                    }
                }
            }
            None => {
                log::error!("Scheduler was not set.");
            }
        }
    }

    pub async fn start() {
        log::warn!("calling to start cron-scheduler.");
        match &Self::get().sched {
            Some(t) => {
                if let Err(err) = t.start().await {
                    log::error!("JobScheduler start error {:?}", err.to_string());
                }
            }
            None => {
                log::error!("Scheduler was not inited.");
            }
        }
    }

    #[allow(dead_code)]
    pub async fn shutdown() {
        if let Some(t) = &mut Self::get_().sched {
            if let Err(err) = t.shutdown().await {
                log::info!("Job shutdonw error: {err:?}");
            } else {
                Self::update(None);
            }
        }
    }

    fn fix_cron_express(express: &str) -> String {
        if express.trim().is_empty() {
            return express.trim().to_owned();
        }
        let cp = express.split(' ');
        let mut cpx = vec![];
        for it in cp {
            cpx.push(it);
            if cpx.len() >= 6 {
                break;
            }
        }

        while cpx.len() < 6 {
            cpx.push("*");
        }

        cpx.join(" ")
    }

    /**
     * Remove the nsuri前缀的job
     */
    pub async fn remove_jobs(nsuri: &str) {
        if Self::get_().sched.is_some() {
            let names = Self::get_()
                .job_map
                .clone()
                .into_iter()
                .filter(|(f, _)| f.starts_with(nsuri))
                .collect::<Vec<(String, Uuid)>>();
            for nm in names {
                if let Some(sched) = &mut Self::get_().sched {
                    if let Err(err) = sched.remove(&nm.1).await {
                        log::info!("error to remove sched {} by {:?}", &nm.0, err);
                    } else {
                        Self::get_().job_map.remove(&nm.0);
                        Self::get_().job_params.remove(&nm.1.to_string());
                    }
                }
            }
        }
    }

    async fn update_pending(uuid: &str) {
        if let Entry::Occupied(mut t) = Self::get_().job_params.entry(uuid.to_string()) {
            t.get_mut().states = Some("PENDING".to_string());
        }
    }

    pub async fn update_begin(uuid: &str) {
        if let Entry::Occupied(mut t) = Self::get_().job_params.entry(uuid.to_string()) {
            t.get_mut().begin_execute();
        }
    }

    pub async fn update_completed(uuid: &str, state: &str, msg: Option<String>) {
        let uuid_text = uuid.to_owned();
        let sch_type =
            if let Entry::Occupied(mut t) = Self::get_().job_params.entry(uuid_text.clone()) {
                t.get_mut().update_elapsed(Some(state.to_owned()), msg);
                let ts = t.get();
                Self::update_store(ts.clone()).await;
                ts.sched_type.unwrap_or(-1)
            } else {
                -1
            };

        //　for 1 or 2, we will update the state to PENDING after 2 minutes
        if sch_type == 1 || sch_type == 2 {
            pin_async_process!(async move {
                let _ = tokio::time::timeout(Duration::from_secs(120), async move {
                    Self::update_pending(&uuid_text).await;
                })
                .await
                .is_ok();
            });
        } else {
            let _ = Self::get_().job_params.remove(uuid);
        }
    }

    pub async fn update_store(st: JobStateInfo) {
        if let Some(job_invoker) = Self::get().job_invoke_uri.clone() {
            if let Ok(invk) = InvokeUri::parse(&job_invoker) {
                if invk.schema == *"object" {
                    let full_store_uri = format!("{job_invoker}#insert");
                    let ctx = Arc::new(Mutex::new(InvocationContext::new()));
                    // log::warn!("JobState: {}", serde_json::to_string(&st).unwrap_or_default());
                    if let Err(err) =
                        MxStoreService::invoke_return_one(full_store_uri, ctx, vec![json!(st)])
                            .await
                    {
                        log::warn!("could not save the state into db. {err}");
                    }
                }
            }
        }
    }

    pub async fn execute_query(qc: QueryCondition) -> Result<Page<Value>, anyhow::Error> {
        if let Some(job_invoker) = Self::get().job_invoke_uri.clone() {
            if let Ok(invk) = InvokeUri::parse(&job_invoker) {
                if invk.schema == *"object" {
                    let full_store_uri = format!("{job_invoker}#paged_query");
                    let ctx = Arc::new(Mutex::new(InvocationContext::new()));
                    return MxStoreService::invoke_return_page(
                        full_store_uri,
                        ctx,
                        vec![json!(qc)],
                    )
                    .await;
                }
            }
        }
        Err(anyhow!(
            "The Schedulers management URI was not configured properly."
        ))
    }

    pub async fn update_addstate(uuid: &str, state: &JobStateInfo) {
        let st = state.clone();
        // st.uuid = Some(uuid.to_owned());
        Self::get_().job_params.insert(uuid.to_owned(), st);
    }

    pub async fn add_jobs(nsuri: &str, cs: ComposeServiceInfo) {
        // if let Some(cron_express) = cs.cron_express {
        let (job_duration, job_express) = if let Some(interval) = cs.interval_second {
            (Some(interval as u64), String::new())
        } else if let Some(cronexpr) = cs.cron_express {
            (None, Self::fix_cron_express(&cronexpr))
        } else {
            log::error!("undefined the job repeatable duration or job cron-express");
            return;
        };
        let desc = cs.desc.unwrap_or_default();

        if let Some(hold) = &Self::get().sched {
            let full_uri = format!("{}#{}", nsuri, cs.name).clone();
            if let Some(jobid) = Self::get().job_map.get(&full_uri) {
                if let Err(err) = hold.remove(jobid).await {
                    log::info!("Could not remove the job {jobid} by error {err:?}");
                }

                let _ = Self::get_().job_params.remove(&jobid.to_string());
            }
            let full_uri_copy = full_uri.clone();
            // let desc = cs.desc.unwrap_or_default();
            // let job_express = Self::fix_cron_express(&cron_express);
            if cs.lang == "shell" {
                let hold_routine = move |_uuid: Uuid, _sched| {
                    let shell_script = cs.script.clone();
                    pin_async_process!(async move {
                        // Self::update_begin(&uuid.to_string()).await;
                        match invoke_shell_script(&shell_script) {
                            Ok(_) => {
                                log::info!(
                                    "Shell Job Invoked Successfully by CronSchedulerHolder."
                                );
                                // Self::update_completed(&uuid.to_string(), "SUCCESS", None).await;
                            }
                            Err(err) => {
                                log::warn!(
                                    "Shell Job Invoked failed by CronSchedulerHolder. {err}"
                                );
                                // Self::update_completed(
                                //     &uuid.to_string(),
                                //     "FAILED",
                                //     Some(err.to_string()),
                                // )
                                // .await;
                            }
                        }
                    })
                };

                let (job_result, schtype, scheexpr) = if let Some(secs) = job_duration {
                    (
                        Job::new_repeated(Duration::from_secs(secs), hold_routine),
                        1,
                        secs.to_string(),
                    )
                } else {
                    // Job::new(&job_express, hold_routine)
                    (
                        Job::new_tz(&job_express, chrono::Local, hold_routine),
                        2,
                        job_express.clone(),
                    )
                };

                let state = JobStateInfo::new(
                    &full_uri_copy,
                    &None,
                    &full_uri_copy,
                    &None,
                    schtype,
                    &scheexpr,
                    &desc,
                );

                match job_result {
                    Ok(job) => match hold.add(job).await {
                        Ok(uuid) => {
                            Self::get_().job_map.insert(full_uri_copy, uuid);
                            Self::update_addstate(&uuid.to_string(), &state).await;
                        }
                        Err(err) => {
                            log::error!("Could not add the job {full_uri_copy} by error {err:?}.");
                        }
                    },
                    Err(err) => {
                        log::error!("Could not create the job {full_uri_copy} by error {err:?}.");
                    }
                }
            } else {
                let simulate = cs
                    .schedule_simulate
                    .clone()
                    .unwrap_or("anonymous".to_owned());
                let desc_clone = desc.clone();
                let hold_routine = move |sid: Uuid, _sched| {
                    let cji = ComposeJobInvoker {
                        uuid: sid.to_string(),
                        uri: full_uri.clone(),
                        simulator: simulate.clone(),
                        desc: desc_clone.clone(),
                    };
                    Self::job_execute(sid.to_string(), Arc::new(Box::new(cji)))
                };

                let (job_result, schtype, scheexpr) = if let Some(secs) = job_duration {
                    (
                        Job::new_repeated_async(Duration::from_secs(secs), hold_routine),
                        1,
                        secs.to_string(),
                    )
                } else {
                    (
                        Job::new_async_tz(&job_express, chrono::Local, hold_routine),
                        2,
                        job_express.clone(),
                    )
                };

                let state = JobStateInfo::new(
                    &full_uri_copy,
                    &None,
                    &full_uri_copy,
                    &None,
                    schtype,
                    &scheexpr,
                    &desc,
                );

                match job_result {
                    Ok(job) => match hold.add(job).await {
                        Ok(uuid) => {
                            log::error!("Job {uuid} was created.");
                            Self::get_().job_map.insert(full_uri_copy, uuid);
                            Self::update_addstate(&uuid.to_string(), &state).await;
                        }
                        Err(err) => {
                            log::error!("Could not add the job {full_uri_copy} by error {err:?}.");
                        }
                    },
                    Err(err) => {
                        log::error!("Could not create the job {full_uri_copy} by error {err:?}.");
                    }
                }
            }
        }
        //}
    }
}

impl SchedulerManager for CronSchedulerHolder {
    fn add_job_delay(
        &self,
        job_id: &str,
        cron_express: &str,
        duration_sec: Option<u64>,
        delay: Option<u64>,
        run: Box<dyn JobInvoker + Send + Sync + 'static>,
    ) -> Pin<Box<dyn Future<Output = Result<Uuid, anyhow::Error>> + Send + 'static>> {
        let job_id_text = job_id.to_owned();
        let cron_ = Self::fix_cron_express(cron_express);

        Box::pin(async move {
            if let Some(hold) = &Self::get().sched {
                let jobinvoke_uri = run.get_invoke_uri();
                let jobparams = run.get_invoke_params();
                let parent_uuid_option = run.get_parent_uuid();
                let desc = run.get_description();
                let routine = move |uuid: Uuid, _lock| {
                    log::debug!("calling the job by async.. {uuid}.");
                    ProxyJobInvoker(uuid.to_string()).exec(run.exec())
                };

                let (result_job, schedtype, schedexpress) = if let Some(secs) = duration_sec {
                    (
                        Job::new_repeated_async(Duration::from_secs(secs), routine),
                        1,
                        secs.to_string(),
                    )
                } else if cron_.is_empty() {
                    (
                        Job::new_one_shot_async(Duration::from_secs(delay.unwrap_or(1)), routine),
                        0,
                        "0".to_string(),
                    )
                } else {
                    // FIXED: Change the UTC TimeZone to Local
                    (
                        Job::new_async_tz(cron_.as_str(), chrono::Local, routine),
                        2,
                        cron_,
                    )
                };

                let state = JobStateInfo::new(
                    &job_id_text,
                    &parent_uuid_option,
                    &jobinvoke_uri,
                    &jobparams,
                    schedtype,
                    &schedexpress,
                    &desc,
                );

                match result_job {
                    Ok(job) => {
                        if Self::get_().job_others.contains_key(&job_id_text) {
                            if let Err(err) = Self::get_().remove_job(&job_id_text).await {
                                log::error!("Remove the exists job by the same name. {err}");
                            }
                        }
                        log::debug!("The job was create init.");
                        match hold.add(job).await {
                            Ok(uuid) => {
                                log::debug!("The job was scheduled.");
                                Self::get_().job_others.insert(job_id_text, uuid);
                                Self::update_addstate(&uuid.to_string(), &state).await;
                                Ok(uuid)
                            }
                            Err(err) => {
                                log::error!(
                                    "Could not add the job {job_id_text} by error {err:?}."
                                );
                                Err(anyhow!("could not {err}"))
                            }
                        }
                    }
                    Err(err) => {
                        log::error!("Could not create the job {job_id_text} by error {err:?}.");
                        Err(anyhow!("could not {err}"))
                    }
                }
            } else {
                Err(anyhow!("The Scheduler was not initialized."))
            }
        })
    }

    fn remove_job(
        &self,
        job_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Uuid, anyhow::Error>> + Send + 'static>> {
        if let Some(hold) = &Self::get().sched {
            if let Some(uuid) = Self::get_().job_others.remove(job_id) {
                Box::pin(async move {
                    if let Err(err) = hold.remove(&uuid).await {
                        log::info!("error on remove the job {uuid} error {err}");
                    }
                    Self::get_().job_params.remove(&uuid.to_string());
                    Ok(uuid)
                })
            } else {
                Box::pin(async move { Err(anyhow!("Not found the job.")) })
            }
        } else {
            Box::pin(async move { Err(anyhow!("The Scheduler was not initialized.")) })
        }
    }

    fn start(&self) {
        pin_blockon_async!(async move {
            CronSchedulerHolder::start().await;
            Box::new(0) as Box<dyn Any + Send + Sync>
        })
        .unwrap_or(0);
    }

    /**
     * 如果没有参数，则直接从job_params中获取
     * 如果查询的是非OneShot的任务
     * --判断是否有Uuid条件，如果没有，直接返回
     */
    fn query_states(
        &self,
        query: &JobStateQuery,
    ) -> Pin<Box<dyn Future<Output = Result<Page<JobStateInfo>, anyhow::Error>> + Send>> {
        let fs = query.oneshot.unwrap_or(false);
        let uuid = query.uuid.clone();
        if fs {
            let vlist: Vec<JobStateInfo> = self.job_params.clone().into_values().collect();
            let paged = Page::new(0, 10000, vlist.len() as u64, vlist);
            // paged.set_records(vlist);
            Box::pin(async move { Ok(paged) })
        } else {
            // 查询数据库
            let ps = query.page_size.unwrap_or(20);
            let curr = query.current.unwrap_or_default();
            let parent_uuid = query.parent_uuid.clone();
            let sched_type = query.sched_type.unwrap_or(-1);

            let mut qc = QueryCondition {
                and: vec![],
                paging: Some(IPaging {
                    size: ps as u64,
                    current: curr as u64,
                }),
                sorts: vec![OrdianlItem {
                    field: "execute_time".to_owned(),
                    sort_asc: false,
                }],
                ..Default::default()
            };

            if sched_type >= 0 {
                if sched_type == 0 {
                    qc.and.push(ConditionItem {
                        field: "sched_type".to_owned(),
                        op: "=".to_owned(),
                        value: json!(sched_type),
                        ..Default::default()
                    });
                } else {
                    qc.and.push(ConditionItem {
                        field: "sched_type".to_owned(),
                        op: "!=".to_owned(),
                        value: json!(0),
                        ..Default::default()
                    });
                }

                if parent_uuid.is_none() {
                    qc.and.push(ConditionItem {
                        field: "parent_uuid".to_owned(),
                        op: "IS NULL".to_owned(),
                        value: Value::Null,
                        ..Default::default()
                    });
                }
            }

            if let Some(uuid_str) = uuid {
                qc.and.push(ConditionItem {
                    field: "uuid".to_owned(),
                    op: "=".to_owned(),
                    value: json!(uuid_str),
                    ..Default::default()
                });
            }

            if let Some(prt_uuid) = parent_uuid {
                qc.and.push(ConditionItem {
                    field: "parent_uuid".to_owned(),
                    op: "=".to_owned(),
                    value: Value::String(prt_uuid),
                    ..Default::default()
                });
            }

            if let Some(states) = query.states.clone() {
                qc.and.push(ConditionItem {
                    field: "states".to_owned(),
                    op: "=".to_owned(),
                    value: Value::String(states),
                    ..Default::default()
                });
            }

            if let Some(begin) = query.execute_begin.clone() {
                if let Some(end) = query.execute_end.clone() {
                    qc.and.push(ConditionItem {
                        field: "execute_time".to_owned(),
                        op: "between".to_owned(),
                        value: json!(begin),
                        value2: json!(end),
                        ..Default::default()
                    });
                }
            }

            Box::pin(async move {
                Self::execute_query(qc).await.map(|s| {
                    let l = s
                        .records
                        .iter()
                        .map(|t| {
                            serde_json::from_value::<JobStateInfo>(t.to_owned())
                                .map_err(|e| {
                                    log::warn!("Error convert {e:?}");
                                    e
                                })
                                .unwrap_or_default()
                        })
                        .filter(|p| p.uuid.is_some())
                        .collect();

                    Page::new(s.page_no, s.page_size, s.total, l)
                })
            })
        }
    }
}
