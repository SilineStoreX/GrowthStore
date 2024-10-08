use super::{
    plugin::PluginServiceInvocation,
    sdk::{Invocation, InvokeUri},
    starter::MxStoreService,
};
use crate::{service::invoker::InvocationContext, utils::build_path};
use anyhow::anyhow;
use async_std::fs::create_dir_all;
use rbatis::Page;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, mem::MaybeUninit, path::PathBuf, str::FromStr, sync::Once};

#[repr(transparent)]
pub struct SchemaRegistry {
    map: HashMap<String, Box<dyn Invocation + Send + Sync>>,
}

impl SchemaRegistry {
    pub fn get_mut() -> &'static mut SchemaRegistry {
        // 使用MaybeUninit延迟初始化
        static mut SCHEMA_REGISTRY_CONF: MaybeUninit<SchemaRegistry> = MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static SCHEMA_REGISTRY_ONCE: Once = Once::new();

        SCHEMA_REGISTRY_ONCE.call_once(|| unsafe {
            SCHEMA_REGISTRY_CONF.as_mut_ptr().write(SchemaRegistry {
                map: HashMap::new(),
            });
        });
        unsafe { &mut (*SCHEMA_REGISTRY_CONF.as_mut_ptr()) }
    }

    pub fn get() -> &'static SchemaRegistry {
        Self::get_mut()
    }

    pub fn register(
        &'static mut self,
        uri: &str,
        im: Box<dyn Invocation + Send + Sync>,
    ) -> &'static mut Self {
        log::info!("register protocol handler for {}", uri);
        if !self.map.contains_key(uri) {
            self.map.insert(uri.to_owned(), im);
        } else {
            log::debug!("duplicated plugin handle. The old plugin handle will be replaced.");
            self.map.remove(uri);
            self.map.insert(uri.to_owned(), im);
        }
        self
    }

    pub fn register_plugin_invocation(&'static mut self, uri: &str) -> &'static mut Self {
        self.register(uri, Box::new(PluginServiceInvocation(uri.to_owned())))
    }

    pub async fn invoke_direct_query(
        &'static self,
        ns: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        query: &str,
        args: &[Value]
    ) -> Result<Vec<Value>, anyhow::Error> {
        if let Some(t) = self.map.get("query") {
            t.invoke_direct_query(ns.to_owned(), ctx, query.to_owned(), args.to_vec()).await
        } else {
            Err(anyhow!("Direct Query only supports for custom query. Which namespace should define at least one custom query."))
        }
    }

    pub async fn invoke_return_option(
        &'static self,
        uri: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Result<Option<Value>, anyhow::Error> {
        let url = InvokeUri::parse(uri)?;
        if url.method == *"get_config" || url.method == *"save_config" {
            // do get config
            match MxStoreService::get(&url.namespace) {
                Some(nss) => {
                    if url.method == *"get_config" {
                        let col_method = url.schema.clone();
                        let val = match col_method.as_str() {
                            "object" => nss
                                .get_object(&url.object)
                                .map(|f| serde_json::to_value(f).unwrap_or(Value::Null)),
                            "query" => nss
                                .get_query(&url.object)
                                .map(|f| serde_json::to_value(f).unwrap_or(Value::Null)),
                            _ => match MxStoreService::get_plugin_service(&url.url_no_method()) {
                                Some(pss) => pss.get_config(),
                                None => None,
                            },
                        };
                        Ok(val)
                    } else if url.method == *"save_config" {
                        let col_method = url.schema.clone();
                        let val: Option<Value> = match col_method.as_str() {
                            "object" => None,
                            "query" => None,
                            _ => {
                                match MxStoreService::get_plugin_service(&url.url_no_method()) {
                                    Some(pss) => {
                                        let model_path = match args[1].clone().get("model_path") {
                                            Some(mp) => mp.as_str().unwrap_or_default().to_string(),
                                            None => {
                                                return Err(anyhow!(
                                                    "No model_path provided by sencond params"
                                                ));
                                            }
                                        };

                                        match pss.parse_config(&args[0].clone()) {
                                            Ok(_) => {
                                                if let Some(plc) =
                                                    nss.get_plugin_config(&url.object)
                                                {
                                                    let mut cpt = plc.clone();
                                                    log::info!(
                                                        "ModelPath: {} --- {}",
                                                        model_path,
                                                        plc.config
                                                    );
                                                    let model_path = PathBuf::from_str(&model_path)
                                                        .unwrap()
                                                        .join(url.namespace.clone());
                                                    if !model_path.exists() {
                                                        if let Err(err) =
                                                            create_dir_all(&model_path).await
                                                        {
                                                            log::info!("err on create dirs {err}");
                                                        }
                                                    }
                                                    cpt.config = match build_path(
                                                        model_path,
                                                        plc.config.clone(),
                                                    ) {
                                                        Ok(cpath) => {
                                                            cpath.to_string_lossy().to_string()
                                                        }
                                                        Err(err) => {
                                                            return Err(anyhow!("No valid model_path provided by sencond params, the error is {:?}", err));
                                                        }
                                                    };

                                                    let ret = match pss.save_config(&cpt) {
                                                        Ok(_) => {
                                                            // reinstall the plugin service.
                                                            pss.get_config()
                                                        }
                                                        Err(err) => {
                                                            return Err(anyhow!("Save plugin config to {} with an error {:?}", cpt.config, err));
                                                        }
                                                    };
                                                    MxStoreService::update_service_add_plugin(
                                                        &url.namespace.clone(),
                                                        &[plc],
                                                    );
                                                    ret
                                                } else {
                                                    return Err(anyhow!(
                                                        "No plugin config from {} be found.",
                                                        url.schema
                                                    ));
                                                }
                                            }
                                            Err(err) => {
                                                log::info!("Could not parse config and update the config for plugin {}, the error is {:?}.", col_method, err);
                                                None
                                            }
                                        }
                                    }
                                    None => None,
                                }
                            }
                        };
                        Ok(val)
                    } else {
                        Err(anyhow!("Not implemented"))
                    }
                }
                None => Err(anyhow!("Not implemented")),
            }
        } else {
            log::info!("Url: {} 00 {}", url.schema, url.url());
            match self.map.get(&url.schema) {
                Some(kt) => kt.invoke_return_option(&url, ctx, args).await,
                None => Err(anyhow!("Not implemented")),
            }
        }
    }

    pub async fn invoke_return_vec(
        &'static self,
        uri: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Result<Vec<Value>, anyhow::Error> {
        let url = InvokeUri::parse(uri)?;
        match self.map.get(&url.schema) {
            Some(kt) => kt.invoke_return_vec(&url, ctx, args).await,
            None => Err(anyhow!("Not implements")),
        }
    }

    pub async fn invoke_return_page(
        &'static self,
        uri: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Result<Page<Value>, anyhow::Error> {
        let url = InvokeUri::parse(uri)?;
        match self.map.get(&url.schema) {
            Some(kt) => kt.invoke_return_page(&url, ctx, args).await,
            None => Err(anyhow!("Not implements")),
        }
    }
}
