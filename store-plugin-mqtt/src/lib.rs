use chimes_store_core::{
    config::PluginConfig,
    service::{plugin::get_schema_registry, starter::MxStoreService},
};
use proc::MqttPluginService;
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

pub  fn get_plugin_name() -> &'static str {
    "mqtt"
}

pub fn plugin_router_register() -> Vec<Router> {
    vec![Router::with_path("/mqtt/<ns>/<name>/<method>/publish").post(api::publish_mqtt_request)]
}

pub fn plugin_anonymous_router_register() -> Vec<Router> {
    vec![]
}

/**
 * 初始化插件
 */
pub fn plugin_init(ns: &str, conf: &PluginConfig) {
    match MqttPluginService::new(ns, conf) {
        Ok(mut wplc) => {
            log::info!(
                "Process the config of plugin and init the plugin for {}.",
                conf.name
            );

            if let Err(err) = wplc.start() {
                log::info!("error to start mqtt service. {err:?}");
            }

            let nsuri = format!("{}://{}/{}", conf.protocol, ns, conf.name);
            get_schema_registry().register_plugin_invocation("mqtt");            
            MxStoreService::register_plugin(&nsuri, Box::new(wplc));
        }
        Err(err) => {
            log::warn!(
                "Plugin weixin was not be apply to {ns}. The config of this plugin was not be parsed. The error is {:?}", 
                err
            );
        }
    }
}
