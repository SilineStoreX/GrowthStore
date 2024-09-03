use anyhow::anyhow;
use chimes_store_core::{
    config::{auth::JwtUserClaims, PluginConfig}, pin_blockon_async, pin_submit, service::{invoker::InvocationContext, plugin::get_schema_registry, sched::{JobInvoker, SchedulerHolder, SchedulerManager}, starter::MxStoreService}
};
use proc::{invoke_shell_script, ComposePluginService, ComposeServiceInfo};
use salvo::Router;
use std::{
    any::Any, collections::HashMap, future::Future, mem::MaybeUninit, pin::Pin, sync::{Arc, Mutex, Once}, thread, time::Duration
};
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

mod api;
mod proc;

/**
 * Plugin开发手记
 * 无法使用tokio这样的runtime来在dylib与bin之间共享代码
 * 所以，会造成async fn无法准确的执行
 * 在Plugin中，无法使用主程序中定义的全局变量
 * 函数是一样的，但因为导出的方式不同  
 */

pub fn get_plugin_name() -> &'static str {
    "compose"
}

//#[no_mangle]
pub fn plugin_router_register() -> Vec<Router> {
    vec![
        Router::with_path("/compose/<ns>/<name>/<method>/single").get(api::execute_single_request),
        Router::with_path("/compose/<ns>/<name>/<method>/single").post(api::execute_single_request),
        Router::with_path("/compose/<ns>/<name>/<method>/single").put(api::execute_single_request),
        Router::with_path("/compose/<ns>/<name>/<method>/list").get(api::execute_vec_request),
        Router::with_path("/compose/<ns>/<name>/<method>/list").post(api::execute_vec_request),
        Router::with_path("/compose/<ns>/<name>/<method>/list").put(api::execute_vec_request),
        Router::with_path("/compose/<ns>/<name>/<method>/page").get(api::execute_paged_request),
        Router::with_path("/compose/<ns>/<name>/<method>/page").post(api::execute_paged_request),
        Router::with_path("/compose/<ns>/<name>/<method>/upload").post(api::execute_upload_request),
        Router::with_path("/compose/<ns>/<name>/<method>/page").put(api::execute_paged_request),
    ]
}

//#[no_mangle]
pub fn plugin_anonymous_router_register() -> Vec<Router> {
    vec![
        Router::with_path("/compose/<ns>/<name>/<method>/single").get(api::execute_single_request),
        Router::with_path("/compose/<ns>/<name>/<method>/single").post(api::execute_single_request),
        Router::with_path("/compose/<ns>/<name>/<method>/single").put(api::execute_single_request),
        Router::with_path("/compose/<ns>/<name>/<method>/list").get(api::execute_vec_request),
        Router::with_path("/compose/<ns>/<name>/<method>/list").post(api::execute_vec_request),
        Router::with_path("/compose/<ns>/<name>/<method>/list").put(api::execute_vec_request),
        Router::with_path("/compose/<ns>/<name>/<method>/page").get(api::execute_paged_request),
        Router::with_path("/compose/<ns>/<name>/<method>/page").post(api::execute_paged_request),
        Router::with_path("/compose/<ns>/<name>/<method>/upload").post(api::execute_upload_request),
        Router::with_path("/compose/<ns>/<name>/<method>/page").put(api::execute_paged_request),
    ]
}

/**
 * 初始化插件
 */
pub fn plugin_init(ns: &str, conf: &PluginConfig) {
    match ComposePluginService::new(ns, conf) {
        Ok(wplc) => {
            log::info!(
                "Process the config of plugin and init the plugin for {}.",
                conf.name
            );
            let nsuri = format!("{}://{}/{}", conf.protocol, ns, conf.name);
            let services = wplc.get_services();

            MxStoreService::register_plugin(&nsuri, Box::new(wplc));
            get_schema_registry().register_plugin_invocation("compose");

            pin_blockon_async!(async move {
                schedule_on(&nsuri).await;
                // thread::sleep(Duration::from_millis(300));
                CronSchedulerHolder::remove_jobs(&nsuri).await;
                for cs in services {
                    if cs.schedule_on {
                        CronSchedulerHolder::add_jobs(&nsuri, cs).await;
                    }
                }
                
                Box::new(0i64) as Box<dyn Any + Send + Sync>
            })
            .unwrap_or(0i64);
        }
        Err(err) => {
            log::warn!(
                "Plugin compose was not be apply to {ns}. The config of this plugin was not be parsed. The error is {:?}", 
                err
            );
        }
    }
}

struct CronSchedulerHolder {
    sched: Option<JobScheduler>,
    job_map: HashMap<String, Uuid>,
    job_others: HashMap<String, Uuid>,
}

impl CronSchedulerHolder {
    fn new() -> CronSchedulerHolder {
        CronSchedulerHolder {
            sched: None,
            job_map: HashMap::new(),
            job_others: HashMap::new(),
        }
    }

    fn get_() -> &'static mut CronSchedulerHolder {
        // 使用MaybeUninit延迟初始化
        static mut SCHEDULER_HOLDER_ON: MaybeUninit<CronSchedulerHolder> = MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static SCHEDULER_HOLDER_ON_ONCE: Once = Once::new();

        SCHEDULER_HOLDER_ON_ONCE.call_once(|| unsafe {
            SCHEDULER_HOLDER_ON
                .as_mut_ptr()
                .write(CronSchedulerHolder::new());
        });

        unsafe { &mut (*SCHEDULER_HOLDER_ON.as_mut_ptr()) }
    }

    pub fn update(sched: Option<JobScheduler>) {
        Self::get_().sched = sched;
    }

    pub fn get() -> &'static CronSchedulerHolder {
        Self::get_()
    }

    #[allow(dead_code)]
    pub async fn init() {
        match &mut Self::get_().sched {
            Some(t) => {
                if !t.inited().await {
                    if let Err(err) = t.init().await {
                        log::info!("JobScheduler start error {:?}", err);
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
        match &mut Self::get_().sched {
            Some(t) => {
                if let Err(err) = t.shutdown().await {
                    log::info!("Job shutdonw error: {:?}", err);
                } else {
                    Self::update(None);
                }
            }
            None => {}
        }
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
                    }
                }
            }
        }
    }

    pub async fn add_jobs(nsuri: &str, cs: ComposeServiceInfo) {
        if let Some(cron_express) = cs.cron_express {
            if let Some(hold) = &Self::get().sched {
                let full_uri = format!("{}#{}", nsuri, cs.name).clone();
                if let Some(jobid) = Self::get().job_map.get(&full_uri) {
                    if let Err(err) = hold.remove(jobid).await {
                        log::info!("Could not remove the job {} by error {:?}", jobid, err);
                    }
                }
                let full_uri_copy = full_uri.clone();
                if cs.lang == "shell" {
                    let hold_routine = move |_uuid, _sched| {
                        let shell_script = cs.script.clone();
                        pin_submit!(async move {
                            invoke_shell_script(&shell_script);
                        })
                    };

                    match Job::new(cron_express.as_str(), hold_routine) {
                        Ok(job) => {
                            match hold
                                .add(job)
                                .await
                            {
                                Ok(uuid) => {
                                    Self::get_().job_map.insert(full_uri_copy, uuid);
                                }
                                Err(err) => {
                                    log::error!(
                                        "Could not add the job {} by error {:?}.",
                                        full_uri_copy,
                                        err
                                    );
                                }
                            }
                        },
                        Err(err) => {
                            log::error!(
                                "Could not create the job {} by error {:?}.",
                                full_uri_copy,
                                err
                            );
                        }
                    }
                } else {
                    let simulate = cs
                        .schedule_simulate
                        .clone()
                        .unwrap_or("anonymous".to_owned());
                    let hold_routine = move |_uuid, _sched| {
                        let inv_full_uri = full_uri.clone();
                        let cs_simulate = simulate.clone();
                        pin_submit!(async move {
                            // TODO: 是否需要提供一个模拟登录来执行的任务的机制
                            // use a default username to replace JwtUserClaims::anonymous();
                            let ctx = Arc::new(Mutex::new(InvocationContext::new_userclaims(
                                JwtUserClaims::username(&cs_simulate),
                            )));
                            match MxStoreService::invoke_return_one(
                                inv_full_uri.clone(),
                                ctx,
                                vec![],
                            )
                            .await
                            {
                                Ok(_) => {}
                                Err(err) => {
                                    log::error!("schedule execute on error {:?}", err);
                                }
                            }
                        })
                    };
                    match Job::new(cron_express.as_str(), hold_routine) {
                        Ok(job) => {
                            match hold
                                .add(job)
                                .await
                            {
                                Ok(uuid) => {
                                    Self::get_().job_map.insert(full_uri_copy, uuid);
                                }
                                Err(err) => {
                                    log::error!(
                                        "Could not add the job {} by error {:?}.",
                                        full_uri_copy,
                                        err
                                    );
                                }
                            }
                        },
                        Err(err) => {
                            log::error!(
                                "Could not create the job {} by error {:?}.",
                                full_uri_copy,
                                err
                            );
                        }
                    }
                }
            }
        }
    }
}


impl SchedulerManager for CronSchedulerHolder {
    fn add_job(&self, job_id: &str, cron_express: &str, run: Box<dyn JobInvoker + Send + Sync>) -> Pin<Box<dyn Future<Output = Result<Uuid, anyhow::Error>> + Send + 'static>>
    {
        let job_id_text = job_id.to_owned();
        let cron_  = cron_express.to_owned();

        Box::pin(async move {
            if let Some(hold) = &Self::get().sched {
                let routine = move |uuid, _lock| {
                    log::error!("calling the job by async.. {uuid}.");
                    run.exec()
                };
                match Job::new_async(cron_.as_str(), routine) {
                    Ok(job) => {
                        if Self::get_().job_others.contains_key(&job_id_text) {
                            if let Err(err) = Self::get_().remove_job(&job_id_text).await {
                                log::error!("Remove the exists job by the same name. {err}");
                            }
                        }
                        log::error!("The job was create init.");
                        match hold
                            .add(job)
                            .await
                        {
                            Ok(uuid) => {
                                log::error!("The job was scheduled.");
                                Self::get_().job_others.insert(job_id_text, uuid);
                                Ok(uuid)
                            }
                            Err(err) => {
                                log::error!(
                                    "Could not add the job {} by error {:?}.",
                                    job_id_text,
                                    err
                                );
                                Err(anyhow!("could not {err}"))
                            }
                        }
                    },
                    Err(err) => {
                        log::error!(
                            "Could not create the job {} by error {:?}.",
                            job_id_text,
                            err
                        );
                        Err(anyhow!("could not {err}"))
                    }
                }
            } else {
                Err(anyhow!("The Scheduler was not initialized."))
            }
        })
    }

    fn remove_job(&self, job_id: &str) -> Pin<Box<dyn Future<Output = Result<Uuid, anyhow::Error>> + Send + 'static>> {
        if let Some(hold) = &Self::get().sched {
            if let Some(uuid) = Self::get_().job_others.remove(job_id) {
                Box::pin(async move {
                    if let Err(err) = hold.remove(&uuid).await {
                        log::info!("error on remove the job {uuid} error {err}");
                    }
                    Ok(uuid)
                })
            } else {
                Box::pin(async move {
                    Err(anyhow!("Not found the job."))
                })
            }
        } else {
            Box::pin(async move {
                Err(anyhow!("The Scheduler was not initialized."))
            })
        }
    }
    
    fn start(&self) {
        pin_blockon_async!(async move {
            CronSchedulerHolder::start().await;
            Box::new(0) as Box<dyn Any + Send + Sync>
        }).unwrap_or(0);
        
    }

}

/**
 * Start the scheduler
 */
#[allow(dead_code)]
async fn schedule_on(_nsuri: &str) {
    if CronSchedulerHolder::get().sched.is_none() {
        let sched = tokio_cron_scheduler::JobScheduler::new().await.unwrap();
        CronSchedulerHolder::update(Some(sched));
        CronSchedulerHolder::start().await;
        SchedulerHolder::update(CronSchedulerHolder::get_());
        thread::sleep(Duration::from_millis(1000));
    }
}


#[allow(dead_code)]
async fn schedule_start(_nsuri: &str) {
    CronSchedulerHolder::start().await;
}

/**
 * Shutdown scheduler
 */
#[allow(dead_code)]
async fn schedule_off() {
    CronSchedulerHolder::shutdown().await;
}
