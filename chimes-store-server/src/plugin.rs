use std::{mem::MaybeUninit, sync::Once};

use anyhow::{anyhow, Result};
use chimes_store_core::config::PluginConfig;
use libloading::Library;
use salvo::Router;

type FnGetProtocolName = unsafe extern "Rust" fn() -> &'static str;
type FnPluginRouteRegister = unsafe extern "Rust" fn() -> Vec<Router>;
type FnPluginInit = unsafe extern "Rust" fn(ns: &str, conf: &PluginConfig);
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

    pub fn plugin_init(&self, ns: &str, plc: &PluginConfig) -> Result<()> {
        if let Some(plugin_init_func) = self.fn_plugin_init {
            log::info!("call plugin_init for {ns}");
            unsafe { plugin_init_func(ns, plc); }
        }
        Ok(())
    }

    pub fn extension_init(&self) -> Result<()> {
        if let Some(extension_init_func) = self.fn_extension_init {
            unsafe { extension_init_func(); }
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub struct PluginRegistry {
    libs: Vec<Library>,
    registry: Vec<PluginRountine>,
}

impl PluginRegistry {
    pub fn get_mut() -> &'static mut PluginRegistry {
        // 使用MaybeUninit延迟初始化
        static mut PLUGIN_REGISTRY_CONF: MaybeUninit<PluginRegistry> = MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static PLUGIN_REGISTRY_ONCE: Once = Once::new();

        PLUGIN_REGISTRY_ONCE.call_once(|| unsafe {
            PLUGIN_REGISTRY_CONF.as_mut_ptr().write(PluginRegistry {
                libs: vec![],
                registry: vec![],
            });
        });
        unsafe { &mut (*PLUGIN_REGISTRY_CONF.as_mut_ptr()) }
    }

    pub fn get() -> &'static PluginRegistry {
        Self::get_mut()
    }

    #[allow(dead_code)]
    pub fn register(&'static mut self, lib: Library, pl: PluginRountine) -> &'static mut Self {
        self.libs.push(lib);
        self.registry.push(pl);
        self
    }

    pub fn register_static(&'static mut self, pl: PluginRountine) -> &'static mut Self {
        self.registry.push(pl);
        self
    }    

    pub fn install<F>(&'static self, fun: &mut F)
    where
        F: FnMut(PluginRountine),
    {
        for pl in self.registry.iter().cloned() {
            fun(pl);
        }
    }
}

/**
 * load the plugin
 * 通常，这个加载会在程序启动的时间加载，这个时间还没有初始他logger
 * 所以，这里直接使用println!来打印出错信息，而无法使用log::error!
 */
#[cfg(not(feature="plugin_rlib"))]
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
                fn_get_protocol_name: get_plugin_name,
                fn_plugin_route_regiter: plugin_router_register,
                fn_plugin_anonymous_route_regiter: plugin_anonymous_router_register,
                fn_plugin_init: plugin_init,
                fn_extension_init: exten_init,
            },
        ))
    }
}

#[cfg(not(feature="plugin_rlib"))]
pub fn static_load_plugin() -> Vec<PluginRountine> {
    vec![]
}

#[cfg(feature="plugin_rlib")]
pub fn load_plugin(_path: &str) -> Result<()> {
    Ok(())
}

#[cfg(feature="plugin_rlib")]
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
        fn_plugin_anonymous_route_regiter: Some(store_plugin_compose::plugin_anonymous_router_register),
        fn_plugin_init: Some(store_plugin_compose::plugin_init),
        fn_extension_init: None,
    };

    let restapi = PluginRountine {
        sort: 2,
        fn_get_protocol_name: Some(store_plugin_restapi::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_restapi::plugin_router_register),
        fn_plugin_anonymous_route_regiter: Some(store_plugin_restapi::plugin_anonymous_router_register),
        fn_plugin_init: Some(store_plugin_restapi::plugin_init),
        fn_extension_init: None,
    };

    let mqtt = PluginRountine {
        sort: 3,
        fn_get_protocol_name: Some(store_plugin_mqtt::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_mqtt::plugin_router_register),
        fn_plugin_anonymous_route_regiter: Some(store_plugin_mqtt::plugin_anonymous_router_register),
        fn_plugin_init: Some(store_plugin_mqtt::plugin_init),
        fn_extension_init: None,
    };

    let kafka = PluginRountine {
        sort: 4,
        fn_get_protocol_name: Some(store_plugin_kafka::get_plugin_name),
        fn_plugin_route_regiter: Some(store_plugin_kafka::plugin_router_register),
        fn_plugin_anonymous_route_regiter: Some(store_plugin_kafka::plugin_anonymous_router_register),
        fn_plugin_init: Some(store_plugin_kafka::plugin_init),
        fn_extension_init: None,
    };

    let mut plugins = vec![rhai, compose, restapi, kafka, mqtt];

    plugins.sort_by(|a, b| a.sort.cmp(&b.sort));
    
    plugins
}

