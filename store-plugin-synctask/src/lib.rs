use std::{future::Future, pin::Pin};

use chimes_store_core::{
    config::PluginConfig,
    service::{plugin::get_schema_registry, queue::TaskLogger, starter::MxStoreService},
};
use proc::SyncTaskPluginService;
use salvo::Router;
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
    "synctask"
}

pub fn plugin_router_register() -> Vec<Router> {
    vec![Router::with_path("/synctask/{ns}/{name}/taskqueue/get").get(api::taskqueue_count_get)]
}

/**
 * 初始化插件
 */
pub fn plugin_init(ns: &str, conf: &PluginConfig) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    let nsuri = format!("{}://{}/{}", conf.protocol, ns, conf.name);
    if let Some(plc) = MxStoreService::get_plugin_service(&nsuri) {
        if let Err(err) = plc.shutdown_plugin() {
            log::error!("error to shutdown plugin for {nsuri}. {err}");
        }
    }

    match SyncTaskPluginService::new(ns, conf) {
        Ok(wplc) => {
            log::info!(
                "Process the config of plugin and init the plugin for {}.",
                conf.name
            );
            let nsuri = format!("{}://{}/{}", conf.protocol, ns, conf.name);
            wplc.start_fetch();
            MxStoreService::register_plugin(&nsuri, Box::new(wplc));
            get_schema_registry().register_plugin_invocation("synctask");
            // SchedulerHolder::get_().start();
            // let nsuri = format!("object://{ns}/SyncTaskLogs");
            let nxss = ns.to_owned();
            Box::pin(async move {
                TaskLogger::update_store_uri(&format!("object://{nxss}/SyncTaskLogs#insert")).await;
            })
        }
        Err(err) => {
            log::warn!(
                "Plugin synctask was not be apply to {ns}. The config of this plugin was not be parsed. The error is {err:?}"
            );
            Box::pin(async {})
        }
    }
}
