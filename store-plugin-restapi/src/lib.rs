use chimes_store_core::{
    config::PluginConfig,
    service::{plugin::get_schema_registry, starter::MxStoreService},
};
use proc::RestapiPluginService;
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
    "restapi"
}


pub fn plugin_router_register() -> Vec<Router> {
    vec![
        Router::with_path("/restapi/<ns>/<name>/<method>/single").get(api::execute_single_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/single").post(api::execute_single_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/single").put(api::execute_single_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/list").get(api::execute_vec_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/list").post(api::execute_vec_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/list").put(api::execute_vec_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/page").get(api::execute_paged_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/page").post(api::execute_paged_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/page").put(api::execute_paged_request),
    ]
}

pub fn plugin_anonymous_router_register() -> Vec<Router> {
    vec![
        Router::with_path("/restapi/<ns>/<name>/<method>/single").get(api::execute_single_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/single").post(api::execute_single_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/single").put(api::execute_single_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/list").get(api::execute_vec_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/list").post(api::execute_vec_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/list").put(api::execute_vec_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/page").get(api::execute_paged_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/page").post(api::execute_paged_request),
        Router::with_path("/restapi/<ns>/<name>/<method>/page").put(api::execute_paged_request),
    ]
}

/**
 * 初始化插件
 */
pub fn plugin_init(ns: &str, conf: &PluginConfig) {
    match RestapiPluginService::new(ns, conf) {
        Ok(wplc) => {
            log::info!(
                "Process the config of plugin and init the plugin for {}.",
                conf.name
            );
            let nsuri = format!("{}://{}/{}", conf.protocol, ns, conf.name);
            // let services = wplc.get_services();

            MxStoreService::register_plugin(&nsuri, Box::new(wplc));
            get_schema_registry().register_plugin_invocation("restapi");
        }
        Err(err) => {
            log::warn!(
                "Plugin restapi was not be apply to {ns}. The config of this plugin was not be parsed. The error is {:?}", 
                err
            );
        }
    }
}
