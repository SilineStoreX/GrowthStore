use std::{
    future::Future,
    pin::Pin,
    sync::{Mutex, OnceLock},
};

use anyhow::{anyhow, Result};
use chimes_store_core::config::PluginConfig;
use libloading::Library;
use salvo::Router;

type FnGetProtocolName = unsafe extern "Rust" fn() -> &'static str;
type FnPluginRouteRegister = unsafe extern "Rust" fn() -> Vec<Router>;
type FnPluginInit = unsafe extern "Rust" fn(
    ns: &str,
    conf: &PluginConfig,
) -> Pin<Box<dyn Future<Output = ()> + Send>>;
type FnExtensionInit = unsafe extern "Rust" fn();

#[derive(Clone)]
pub struct PluginRountine {
    pub sort: i32,
    pub fn_get_protocol_name: Option<FnGetProtocolName>,
    pub fn_plugin_route_regiter: Option<FnPluginRouteRegister>,
    pub fn_plugin_anonymous_route_regiter: Option<FnPluginRouteRegister>,
    pub fn_plugin_init: Option<FnPluginInit>,
    pub fn_extension_init: Option<FnExtensionInit>,
}

impl PluginRountine {
    pub fn get_protocol_name(&self) -> Result<String, anyhow::Error> {
        if let Some(get_protocol_name_fun) = self.fn_get_protocol_name {
            unsafe {
                let cp = get_protocol_name_fun();
                Ok(cp.to_string())
            }
        } else {
            log::info!("the plugin was loaded but it didn't implement the get_protocol_name function. The plugin may not be worked properly.");
            Err(anyhow!(
                "Plugin did't implement the get_protocol_name function."
            ))
        }
    }

    pub async fn plugin_init(&self, ns: &str, plc: &PluginConfig) -> Result<()> {
        if let Some(plugin_init_func) = self.fn_plugin_init {
            log::debug!("call plugin_init for {ns}");
            unsafe {
                plugin_init_func(ns, plc).await;
            }
        }
        Ok(())
    }

    pub fn extension_init(&self) -> Result<()> {
        if let Some(extension_init_func) = self.fn_extension_init {
            unsafe {
                extension_init_func();
            }
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub struct PluginRegistry {
    libs: Mutex<Vec<Library>>,
    registry: Mutex<Vec<PluginRountine>>,
}

impl PluginRegistry {
    pub fn get() -> &'static PluginRegistry {
        // 使用MaybeUninit延迟初始化
        static PLUGIN_REGISTRY_CONF: OnceLock<PluginRegistry> = OnceLock::new();

        PLUGIN_REGISTRY_CONF.get_or_init(|| PluginRegistry {
            libs: Mutex::new(vec![]),
            registry: Mutex::new(vec![]),
        })
    }

    #[allow(dead_code)]
    pub fn register(&'static self, lib: Library, pl: PluginRountine) -> &'static Self {
        self.libs.lock().unwrap().push(lib);
        self.registry.lock().unwrap().push(pl);
        self
    }

    pub fn register_static(&'static self, pl: PluginRountine) -> &'static Self {
        self.registry.lock().unwrap().push(pl);
        self
    }

    pub fn iter(&'static self) -> Vec<PluginRountine> {
        self.registry.lock().unwrap().clone()
    }

    pub fn install<F>(&'static self, fun: &mut F)
    where
        F: FnMut(PluginRountine),
    {
        for pl in self.registry.lock().unwrap().iter().cloned() {
            fun(pl);
        }
    }
}

#[cfg(not(feature = "plugin_rlib"))]
static LOADED_PLUGIN_ID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);

/**
 * load the plugin
 * 通常，这个加载会在程序启动的时间加载，这个时间还没有初始他logger
 * 所以，这里直接使用println!来打印出错信息，而无法使用log::error!
 */
#[cfg(not(feature = "plugin_rlib"))]
pub fn load_plugin(path: &str) -> Result<(Library, PluginRountine)> {
    unsafe {
        let lib = match libloading::Library::new(path) {
            Ok(l) => l,
            Err(err) => {
                return Err(anyhow!(err));
            }
        };
        let get_plugin_name: Option<FnGetProtocolName> = match lib.get(b"get_plugin_name") {
            Ok(s) => Some(*s),
            Err(_err) => None,
        };

        let plugin_router_register: Option<FnPluginRouteRegister> =
            match lib.get(b"plugin_router_register") {
                Ok(s) => Some(*s),
                Err(_err) => None,
            };

        let plugin_anonymous_router_register: Option<FnPluginRouteRegister> =
            match lib.get(b"plugin_anonymous_router_register") {
                Ok(s) => Some(*s),
                Err(_err) => None,
            };

        let plugin_init: Option<FnPluginInit> = match lib.get(b"plugin_init") {
            Ok(s) => Some(*s),
            Err(_err) => None,
        };

        let exten_init: Option<FnExtensionInit> = match lib.get(b"extension_init") {
            Ok(s) => Some(*s),
            Err(_err) => None,
        };
        Ok((
            lib,
            PluginRountine {
                sort: LOADED_PLUGIN_ID.fetch_add(1, std::sync::atomic::Ordering::AcqRel),
                fn_get_protocol_name: get_plugin_name,
                fn_plugin_route_regiter: plugin_router_register,
                fn_plugin_anonymous_route_regiter: plugin_anonymous_router_register,
                fn_plugin_init: plugin_init,
                fn_extension_init: exten_init,
            },
        ))
    }
}

#[cfg(not(feature = "plugin_rlib"))]
pub fn static_load_plugin() -> Vec<PluginRountine> {
    vec![]
}

#[cfg(feature = "plugin_rlib")]
pub fn load_plugin(_path: &str) -> Result<()> {
    Ok(())
}

#[cfg(feature = "plugin_rlib")]
pub fn static_load_plugin() -> Vec<PluginRountine> {
    let rhai = PluginRountine {
        sort: 0,
        fn_get_protocol_name: Some(store_plugin_rhai::get_plugin_name),
        fn_plugin_route_regiter: None,
        fn_plugin_anonymous_route_regiter: None,
        fn_plugin_init: Some(store_plugin_rhai::plugin_init),
        fn_extension_init: Some(store_plugin_rhai::extension_init),
    };
    let compose = PluginRountine {
        sort: 1,
        fn_get_protocol_name: Some(store_plugin_compose::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_compose::plugin_router_register),
        fn_plugin_anonymous_route_regiter: Some(
            store_plugin_compose::plugin_anonymous_router_register,
        ),
        fn_plugin_init: Some(store_plugin_compose::plugin_init),
        fn_extension_init: None,
    };

    let restapi = PluginRountine {
        sort: 3,
        fn_get_protocol_name: Some(store_plugin_restapi::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_restapi::plugin_router_register),
        fn_plugin_anonymous_route_regiter: Some(
            store_plugin_restapi::plugin_anonymous_router_register,
        ),
        fn_plugin_init: Some(store_plugin_restapi::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "python")]
    let python = PluginRountine {
        sort: 2,
        fn_get_protocol_name: Some(store_plugin_python::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_python::plugin_router_register),
        fn_plugin_anonymous_route_regiter: Some(
            store_plugin_python::plugin_anonymous_router_register,
        ),
        fn_plugin_init: Some(store_plugin_python::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "filesystem")]
    let filesystem = PluginRountine {
        sort: 4,
        fn_get_protocol_name: Some(store_plugin_filesystem::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_filesystem::plugin_router_register),
        fn_plugin_anonymous_route_regiter: Some(
            store_plugin_filesystem::plugin_anonymous_router_register,
        ),
        fn_plugin_init: Some(store_plugin_filesystem::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "imagination")]
    let imagination = PluginRountine {
        sort: 3,
        fn_get_protocol_name: Some(store_plugin_imagination::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_imagination::plugin_router_register),
        fn_plugin_anonymous_route_regiter: None,
        fn_plugin_init: Some(store_plugin_imagination::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "mqtt")]
    let mqtt = PluginRountine {
        sort: 10,
        fn_get_protocol_name: Some(store_plugin_mqtt::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_mqtt::plugin_router_register),
        fn_plugin_anonymous_route_regiter: Some(
            store_plugin_mqtt::plugin_anonymous_router_register,
        ),
        fn_plugin_init: Some(store_plugin_mqtt::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "kafka")]
    let kafka = PluginRountine {
        sort: 11,
        fn_get_protocol_name: Some(store_plugin_kafka::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_kafka::plugin_router_register),
        fn_plugin_anonymous_route_regiter: Some(
            store_plugin_kafka::plugin_anonymous_router_register,
        ),
        fn_plugin_init: Some(store_plugin_kafka::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "es")]
    let elasticsearch = PluginRountine {
        sort: 12,
        fn_get_protocol_name: Some(store_plugin_es::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_es::plugin_router_register),
        fn_plugin_anonymous_route_regiter: None,
        fn_plugin_init: Some(store_plugin_es::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "rivermap")]
    let rivermap = PluginRountine {
        sort: 20,
        fn_get_protocol_name: Some(store_plugin_rivermap::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_rivermap::plugin_router_register),
        fn_plugin_anonymous_route_regiter: None,
        fn_plugin_init: Some(store_plugin_rivermap::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "rivermap")]
    let datagraph = PluginRountine {
        sort: 21,
        fn_get_protocol_name: Some(store_plugin_datagraph::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_datagraph::plugin_router_register),
        fn_plugin_anonymous_route_regiter: None,
        fn_plugin_init: Some(store_plugin_datagraph::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "synctask")]
    let synctask = PluginRountine {
        sort: 8,
        fn_get_protocol_name: Some(store_plugin_synctask::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_synctask::plugin_router_register),
        fn_plugin_anonymous_route_regiter: None,
        fn_plugin_init: Some(store_plugin_synctask::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "voice")]
    let voice = PluginRountine {
        sort: 31,
        fn_get_protocol_name: Some(store_plugin_voice::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_voice::plugin_router_register),
        fn_plugin_anonymous_route_regiter: None,
        fn_plugin_init: Some(store_plugin_voice::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "rag")]
    let rag = PluginRountine {
        sort: 40,
        fn_get_protocol_name: Some(store_plugin_rag::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_rag::plugin_router_register),
        fn_plugin_anonymous_route_regiter: None,
        fn_plugin_init: Some(store_plugin_rag::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "mcp")]
    let mcp = PluginRountine {
        sort: 9,
        fn_get_protocol_name: Some(store_plugin_mcp::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_mcp::plugin_router_register),
        fn_plugin_anonymous_route_regiter: None,
        fn_plugin_init: Some(store_plugin_mcp::plugin_init),
        fn_extension_init: None,
    };

    #[cfg(feature = "ragent")]
    let ragent = PluginRountine {
        sort: 41,
        fn_get_protocol_name: Some(store_plugin_ragent::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_ragent::plugin_router_register),
        fn_plugin_anonymous_route_regiter: None,
        fn_plugin_init: Some(store_plugin_ragent::plugin_init),
        fn_extension_init: None,
    };

    // #[cfg(feature = "deno")]
    // let deno = PluginRountine {
    //     sort: 0,
    //     fn_get_protocol_name: Some(store_plugin_deno::get_plugin_name),
    //     fn_plugin_route_regiter: None,
    //     fn_plugin_anonymous_route_regiter: None,
    //     fn_plugin_init: Some(store_plugin_deno::plugin_init),
    //     fn_extension_init: Some(store_plugin_deno::extension_init),
    // };

    let mut plugins = vec![rhai, compose, restapi];

    #[cfg(feature = "python")]
    plugins.push(python);

    #[cfg(feature = "filesystem")]
    plugins.push(filesystem);

    #[cfg(feature = "imagination")]
    plugins.push(imagination);

    #[cfg(feature = "mqtt")]
    plugins.push(mqtt);

    #[cfg(feature = "kafka")]
    plugins.push(kafka);

    #[cfg(feature = "es")]
    plugins.push(elasticsearch);

    #[cfg(feature = "rivermap")]
    plugins.push(rivermap);

    #[cfg(feature = "rivermap")]
    plugins.push(datagraph);

    #[cfg(feature = "synctask")]
    plugins.push(synctask);

    #[cfg(feature = "voice")]
    plugins.push(voice);

    #[cfg(feature = "rag")]
    plugins.push(rag);

    #[cfg(feature = "mcp")]
    plugins.push(mcp);

    #[cfg(feature = "ragent")]
    plugins.push(ragent);

    // #[cfg(feature = "deno")]
    // plugins.push(deno);

    plugins.sort_by(|a, b| a.sort.cmp(&b.sort));

    plugins
}
