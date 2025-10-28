use chimes_store_core::{
    config::{auth::AuthorizationConfig, PluginConfig},
    service::{plugin::get_schema_registry, sched::SchedulerHolder, starter::MxStoreService},
};
use jobs::CronSchedulerHolder;
use proc::ComposePluginService;
use salvo::Router;
use std::{future::Future, pin::Pin, thread, time::Duration};

mod api;
mod jobs;
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
        Router::with_path("/compose/{ns}/{name}/{method}/schema").get(api::get_schema),
        Router::with_path("/compose/{ns}/{name}/{method}/single").get(api::execute_single_request),
        Router::with_path("/compose/{ns}/{name}/{method}/single").post(api::execute_single_request),
        Router::with_path("/compose/{ns}/{name}/{method}/single").put(api::execute_single_request),
        Router::with_path("/compose/{ns}/{name}/{method}/list").get(api::execute_vec_request),
        Router::with_path("/compose/{ns}/{name}/{method}/list").post(api::execute_vec_request),
        Router::with_path("/compose/{ns}/{name}/{method}/list").put(api::execute_vec_request),
        Router::with_path("/compose/{ns}/{name}/{method}/page").get(api::execute_paged_request),
        Router::with_path("/compose/{ns}/{name}/{method}/page").post(api::execute_paged_request),
        Router::with_path("/compose/{ns}/{name}/{method}/upload").post(api::execute_upload_request),
        Router::with_path("/compose/{ns}/{name}/{method}/page").put(api::execute_paged_request),
    ]
}

//#[no_mangle]
pub fn plugin_anonymous_router_register() -> Vec<Router> {
    vec![
        Router::with_path("/compose/{ns}/{name}/{method}/schema").get(api::get_schema),
        Router::with_path("/compose/{ns}/{name}/{method}/single").get(api::execute_single_request),
        Router::with_path("/compose/{ns}/{name}/{method}/single").post(api::execute_single_request),
        Router::with_path("/compose/{ns}/{name}/{method}/single").put(api::execute_single_request),
        Router::with_path("/compose/{ns}/{name}/{method}/list").get(api::execute_vec_request),
        Router::with_path("/compose/{ns}/{name}/{method}/list").post(api::execute_vec_request),
        Router::with_path("/compose/{ns}/{name}/{method}/list").put(api::execute_vec_request),
        Router::with_path("/compose/{ns}/{name}/{method}/page").get(api::execute_paged_request),
        Router::with_path("/compose/{ns}/{name}/{method}/page").post(api::execute_paged_request),
        Router::with_path("/compose/{ns}/{name}/{method}/upload").post(api::execute_upload_request),
        Router::with_path("/compose/{ns}/{name}/{method}/page").put(api::execute_paged_request),
    ]
}

/**
 * 初始化插件
 */
pub fn plugin_init(ns: &str, conf: &PluginConfig) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    match ComposePluginService::new(ns, conf) {
        Ok(wplc) => {
            log::info!(
                "Process the  config of COMPOSE plugin and init the plugin for {}.",
                conf.name
            );
            let nsuri = format!("{}://{}/{}", conf.protocol, ns, conf.name);
            let services = wplc.get_services();

            MxStoreService::register_plugin(&nsuri, Box::new(wplc));
            get_schema_registry().register_plugin_invocation("compose");
            let sch_state_uri = AuthorizationConfig::get().schedule_state_uri.clone();

            Box::pin(async move {
                schedule_on(&nsuri).await;
                // thread::sleep(Duration::from_millis(300));
                CronSchedulerHolder::set_states_mangement_uri(&sch_state_uri);
                CronSchedulerHolder::remove_jobs(&nsuri).await;
                for cs in services {
                    if cs.schedule_on {
                        CronSchedulerHolder::add_jobs(&nsuri, cs).await;
                    }
                }

                // Box::new(0i64) as Box<dyn Any + Send + Sync>
            })
        }
        Err(err) => {
            log::warn!(
                "Plugin compose was not be apply to {ns}. The config of this plugin was not be parsed. The error is {err:?}"
            );
            Box::pin(async move {})
        }
    }
}

/**
 * Start the scheduler
 */
#[allow(dead_code)]
async fn schedule_on(_nsuri: &str) {
    if CronSchedulerHolder::get().sched.is_none() {
        log::info!("Scheduler is single one to be started at once.");
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
