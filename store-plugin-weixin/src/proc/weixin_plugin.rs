use anyhow::anyhow;
use chimes_store_core::config::auth::JwtUserClaims;
use chimes_store_core::config::PluginConfig;
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::sdk::{InvokeUri, MethodDescription, RxPluginService};
use chimes_store_core::service::starter::load_config;
use rbatis::Page;
use salvo::oapi::OpenApi;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WeixinPluginConfig {
    pub app_id: Option<String>,
    pub app_secret: Option<String>,
}

#[allow(dead_code)]
pub struct WeixinPluginService {
    namespace: String,
    conf: PluginConfig,
    weixin: RefCell<Option<WeixinPluginConfig>>,
}

impl WeixinPluginService {
    pub fn new(ns: &str, conf: &PluginConfig) -> Result<Self, anyhow::Error> {
        log::debug!("Plugin config load from {}", conf.config.clone());
        let t = match load_config(conf.config.clone()) {
            Ok(r) => r,
            Err(err) => {
                log::debug!("Could not load the config file: {:?}", err);
                Some(WeixinPluginConfig::default())
            }
        };

        Ok(Self {
            namespace: ns.to_owned(),
            conf: conf.to_owned(),
            weixin: RefCell::new(t),
        })
    }
}

unsafe impl Send for WeixinPluginService {}

unsafe impl Sync for WeixinPluginService {}

impl RxPluginService for WeixinPluginService {
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
        match serde_json::to_value(self.weixin.clone()) {
            Ok(t) => Some(t),
            Err(err) => {
                log::debug!("Convert to json with error: {:?}", err);
                None
            }
        }
    }

    fn parse_config(&self, val: &Value) -> Result<(), anyhow::Error> {
        match serde_json::from_value::<WeixinPluginConfig>(val.to_owned()) {
            Ok(t) => {
                self.weixin.replace(Some(t));
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
        chimes_store_core::service::starter::save_config(&self.weixin.borrow().clone(), path)
    }

    fn get_metadata(&self) -> Vec<chimes_store_core::service::sdk::MethodDescription> {
        vec![MethodDescription {
            uri: "weixin://{ns}/{name}".to_owned(),
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

    fn get_openapi(&self, _ns: &str) -> Box<dyn std::any::Any> {
        Box::new(OpenApi::new("title", "version"))
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
