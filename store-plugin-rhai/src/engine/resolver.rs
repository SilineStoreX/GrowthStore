use chimes_store_core::config::{ConditionItem, IPaging, OrdianlItem, QueryCondition};
use chimes_store_core::pin_blockon_async_v2;
use chimes_store_core::service::invoker::release_all_connections;
use chimes_store_core::service::sdk::InvokeUri;
use chimes_store_core::service::{invoker::InvocationContext, starter::MxStoreService};
use rbatis::{IPageRequest, Page};
use rhai::{
    Array, CustomType, Dynamic, Engine, EvalAltResult, ImmutableString, Module, ModuleResolver,
    NativeCallContext, Position, TypeBuilder,
};
use serde_json::{json, Map, Number, Value};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;
use std::sync::{Arc, OnceLock};
pub struct RhaiStoreServiceResolver {}

impl RhaiStoreServiceResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl ModuleResolver for RhaiStoreServiceResolver {
    fn resolve(
        &self,
        _engine: &rhai::Engine,
        source: Option<&str>,
        path: &str,
        _pos: rhai::Position,
    ) -> Result<rhai::Shared<rhai::Module>, Box<EvalAltResult>> {
        log::info!("Source: {source:?} of path: {path}");
        println!("Source: {source:?} of path: {path}");
        Ok(rhai::Shared::new(
            RhaiStoreModule::get_store_module(path).create_module(),
        ))
    }
}

#[derive(Clone)]
pub struct RhaiStoreModule(pub(crate) String);

impl RhaiStoreModule {
    // pub fn get_mut() -> &'static mut HashMap<String, RhaiStoreModule> {
    //     // 使用MaybeUninit延迟初始化
    //     static mut RHAI_STORED_MODULE_MAP: OnceLock<HashMap<String, RhaiStoreModule>> = OnceLock::new();
    //     unsafe {
    //         let _ = RHAI_STORED_MODULE_MAP.get_or_init(|| {
    //             HashMap::new()
    //         });

    //         RHAI_STORED_MODULE_MAP.get_mut().unwrap()
    //     }
    // }

    pub fn get_map() -> &'static Mutex<HashMap<String, RhaiStoreModule>> {
        // 使用MaybeUninit延迟初始化
        static RHAI_STORED_MODULE_MAP_LOCK: OnceLock<Mutex<HashMap<String, RhaiStoreModule>>> =
            OnceLock::new();

        RHAI_STORED_MODULE_MAP_LOCK.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub fn get_store_module(path: &str) -> RhaiStoreModule {
        match Self::get_map().lock().unwrap().entry(path.to_string()) {
            Entry::Vacant(t) => {
                let ct = t.insert(RhaiStoreModule(path.to_owned()));
                ct.clone()
            }
            Entry::Occupied(t) => t.get().clone(),
        }
    }

    fn create_module(&self) -> Module {
        let mut module = Module::new();

        fn test_select(ns: &str) -> Result<Option<Value>, Box<EvalAltResult>> {
            Ok(Some(json!(MxStoreService::get(ns).unwrap().get_objects())))
        }

        let path = self.0.clone();
        module.set_native_fn("select", move || test_select(&path));
        module
    }
}

#[derive(Clone, CustomType)]
pub struct RhaiStoreObject {
    uri: String,
}

impl RhaiStoreObject {
    pub fn new(uri: &str) -> Self {
        Self {
            uri: uri.to_owned(),
        }
    }

    #[allow(dead_code)]
    pub fn get_uri(&self) -> String {
        self.uri.clone()
    }

    pub fn aes_encrypt(col: &mut Self, text: &str) -> Result<String, Box<EvalAltResult>> {
        let call_uri = format!("{}#find_one", col.uri);
        if let Ok(invoke_uri) = InvokeUri::parse(&call_uri) {
            if let Some(mxs) = MxStoreService::get(&invoke_uri.namespace) {
                Ok(mxs.aes_encode_text(text))
            } else {
                Err(Box::new(EvalAltResult::ErrorRuntime(
                    Dynamic::from(format!(
                        "namespace: {} was not be found.",
                        &invoke_uri.namespace
                    )),
                    Position::new(1, 1),
                )))
            }
        } else {
            Err(Box::new(EvalAltResult::ErrorRuntime(
                Dynamic::from(format!(
                    "special {} could not be parsed as InvokeURI.",
                    &call_uri
                )),
                Position::new(1, 1),
            )))
        }
    }

    pub fn aes_decrypt(col: &mut Self, text: &str) -> Result<String, Box<EvalAltResult>> {
        let call_uri = format!("{}#find_one", col.uri);
        if let Ok(invoke_uri) = InvokeUri::parse(&call_uri) {
            if let Some(mxs) = MxStoreService::get(&invoke_uri.namespace) {
                Ok(mxs.aes_decode_text(text))
            } else {
                Err(Box::new(EvalAltResult::ErrorRuntime(
                    Dynamic::from(format!(
                        "namespace: {} was not be found.",
                        &invoke_uri.namespace
                    )),
                    Position::new(1, 1),
                )))
            }
        } else {
            Err(Box::new(EvalAltResult::ErrorRuntime(
                Dynamic::from(format!(
                    "special {} could not be parsed as InvokeURI.",
                    &call_uri
                )),
                Position::new(1, 1),
            )))
        }
    }

    pub fn sha256_with_rsa_sign(col: &mut Self, text: &str) -> Result<String, Box<EvalAltResult>> {
        let call_uri = format!("{}#find_one", col.uri);
        if let Ok(invoke_uri) = InvokeUri::parse(&call_uri) {
            if let Some(mxs) = MxStoreService::get(&invoke_uri.namespace) {
                Ok(mxs.sha256_with_rsa_sign(text))
            } else {
                Err(Box::new(EvalAltResult::ErrorRuntime(
                    Dynamic::from(format!(
                        "namespace: {} was not be found.",
                        &invoke_uri.namespace
                    )),
                    Position::new(1, 1),
                )))
            }
        } else {
            Err(Box::new(EvalAltResult::ErrorRuntime(
                Dynamic::from(format!(
                    "special {} could not be parsed as InvokeURI.",
                    &call_uri
                )),
                Position::new(1, 1),
            )))
        }
    }

    pub fn rsa_decrypt(col: &mut Self, text: &str) -> Result<String, Box<EvalAltResult>> {
        let call_uri = format!("{}#find_one", col.uri);
        if let Ok(invoke_uri) = InvokeUri::parse(&call_uri) {
            if let Some(mxs) = MxStoreService::get(&invoke_uri.namespace) {
                Ok(mxs.rsa_decrypt_text(text))
            } else {
                Err(Box::new(EvalAltResult::ErrorRuntime(
                    Dynamic::from(format!(
                        "namespace: {} was not be found.",
                        &invoke_uri.namespace
                    )),
                    Position::new(1, 1),
                )))
            }
        } else {
            Err(Box::new(EvalAltResult::ErrorRuntime(
                Dynamic::from(format!(
                    "special {} could not be parsed as InvokeURI.",
                    &call_uri
                )),
                Position::new(1, 1),
            )))
        }
    }

    pub fn rsa_encrypt(col: &mut Self, text: &str) -> Result<String, Box<EvalAltResult>> {
        let call_uri = format!("{}#find_one", col.uri);
        if let Ok(invoke_uri) = InvokeUri::parse(&call_uri) {
            if let Some(mxs) = MxStoreService::get(&invoke_uri.namespace) {
                Ok(mxs.rsa_encrypt_text(text))
            } else {
                Err(Box::new(EvalAltResult::ErrorRuntime(
                    Dynamic::from(format!(
                        "namespace: {} was not be found.",
                        &invoke_uri.namespace
                    )),
                    Position::new(1, 1),
                )))
            }
        } else {
            Err(Box::new(EvalAltResult::ErrorRuntime(
                Dynamic::from(format!(
                    "special {} could not be parsed as InvokeURI.",
                    &call_uri
                )),
                Position::new(1, 1),
            )))
        }
    }

    pub(crate) fn invoke(
        col: &mut Self,
        name: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Value,
    ) -> Result<Option<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#{}", col.uri, name);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_one(call_uri.clone(), ctx, vec![args]).await {
                Ok(t) => Ok(t),
                Err(err) => Err(Box::new(EvalAltResult::ErrorRuntime(
                    Dynamic::from(err.to_string()),
                    Position::new(1, 1),
                ))),
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub(crate) fn invoke_args(
        col: &mut Self,
        name: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Option<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#{}", col.uri, name);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_one(call_uri.clone(), ctx, args).await {
                Ok(t) => Ok(t),
                Err(err) => Err(Box::new(EvalAltResult::ErrorRuntime(
                    Dynamic::from(err.to_string()),
                    Position::new(1, 1),
                ))),
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub fn select(
        col: &mut Self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Value,
    ) -> Result<Option<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#select", col.uri);
        let call_uri2 = call_uri.clone();

        pin_blockon_async_v2!(async move {
            log::info!("invoker select return one {call_uri}");

            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_one(call_uri.clone(), ctx, vec![args]).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub fn find_one(
        col: &mut Self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Value,
    ) -> Result<Option<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#find_one", col.uri);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_one(call_uri.clone(), ctx, vec![args]).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub fn query(
        &mut self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#query", self.uri);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_vec(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub fn paged_query(
        &mut self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Page<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#paged_query", self.uri);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_page(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }
}

#[derive(Clone, CustomType)]
pub struct RhaiStoreQuery {
    uri: String,
}

impl RhaiStoreQuery {
    pub fn new(uri: &str) -> Self {
        Self {
            uri: uri.to_owned(),
        }
    }

    pub fn get_uri(&self) -> String {
        self.uri.clone()
    }

    pub fn find_one(
        &mut self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Option<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#find_one", self.uri);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // let wret = ret.map(|f| f.map(|mut t| ValueGetterSetter::to_rhai_object(&mut t)));
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_one(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub fn direct_query(
        &mut self,
        ctx: Arc<Mutex<InvocationContext>>,
        sql: String,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#find_one", self.uri);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // let wret = ret.map(|f| f.map(|mut t| ValueGetterSetter::to_rhai_object(&mut t)));
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_direct_query(call_uri, ctx, sql, args).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub fn direct_query_v2(
        &mut self,
        ctx: Arc<Mutex<InvocationContext>>,
        query: Value,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#find_one", self.uri);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // let wret = ret.map(|f| f.map(|mut t| ValueGetterSetter::to_rhai_object(&mut t)));
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_direct_query_v2(call_uri, ctx, query, args).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub fn execute(
        &mut self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Option<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#execute", self.uri);
        let call_uri2 = call_uri.clone();
        // pin_blockon_async_v2!(async move {
        //     let ret = match MxStoreService::invoke_return_one(call_uri.clone(), ctx, args).await {
        //         Ok(t) => {
        //             log::info!("OK: {:?}", t);
        //             Ok(t)
        //         }
        //         Err(err) => {
        //             log::info!("Err: {:?}", err);
        //             Err(Box::new(EvalAltResult::ErrorRuntime(
        //                 Dynamic::from(err.to_string()),
        //                 Position::new(1, 1),
        //             )))
        //         }
        //     };
        //     // let wret = ret.map(|f| f.map(|mut t| ValueGetterSetter::to_rhai_object(&mut t)));
        //     Box::new(ret) as Box<dyn Any + Send + Sync>
        // })
        // .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
        //     Dynamic::from(call_uri2),
        //     Position::new(1, 1),
        // ))))
        pin_blockon_async_v2!(async move {
            // let wret = ret.map(|f| f.map(|mut t| ValueGetterSetter::to_rhai_object(&mut t)));
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_one(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub fn search(
        &mut self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#search", self.uri);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_vec(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub fn paged_search(
        &mut self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Page<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#paged_search", self.uri);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_page(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }
}

#[derive(Clone, CustomType)]
pub struct RhaiStorePlugin {
    uri: String,
}

impl RhaiStorePlugin {
    pub fn new(uri: &str) -> Self {
        Self {
            uri: uri.to_owned(),
        }
    }

    pub fn get_uri(&self) -> String {
        self.uri.clone()
    }

    pub fn register(engine: &mut Engine) {
        engine
            .register_type_with_name::<RhaiStorePlugin>("PluginObject")
            .register_fn("new_plugin_object", RhaiStorePlugin::new)
            .register_fn(
                "invoke_return_option",
                RhaiStorePlugin::invoke_return_option,
            )
            .register_fn(
                "invoke_return_option",
                RhaiStorePlugin::invoke_return_option_json,
            )
            .register_fn("invoke_return_vec", RhaiStorePlugin::invoke_return_vec)
            .register_fn("invoke_return_vec", RhaiStorePlugin::invoke_return_vec_json)
            .register_fn("invoke_return_page", RhaiStorePlugin::invoke_return_page)
            .register_fn(
                "invoke_return_page",
                RhaiStorePlugin::invoke_return_page_json,
            );

        MxStoreService::get_namespaces().into_iter().for_each(|ns| {
            // log::info!("register {ns}");
            if let Some(nss) = MxStoreService::get(&ns) {
                nss.get_plugins().into_iter().for_each(|proto| {
                    let ns_protocol = format!("{}://{ns}/{}", proto.protocol, proto.name);
                    if let Some(pls) = MxStoreService::get_plugin_service(&ns_protocol) {
                        pls.get_metadata().into_iter().for_each(|m| {
                            // log::info!("register method {} for {ns_protocol}", m.name);
                            if m.return_page {
                                if m.params_vec {
                                    engine.register_fn(
                                        &m.name,
                                        |callctx: NativeCallContext,
                                         obj: &mut RhaiStorePlugin,
                                         ctx: Arc<Mutex<InvocationContext>>,
                                         args: Array| {
                                            obj.invoke_return_page(callctx.fn_name(), ctx, args)
                                        },
                                    );
                                    engine.register_fn(
                                        &m.name,
                                        |callctx: NativeCallContext,
                                         obj: &mut RhaiStorePlugin,
                                         ctx: Arc<Mutex<InvocationContext>>,
                                         args: Vec<Value>| {
                                            obj.invoke_return_page_json(
                                                callctx.fn_name(),
                                                ctx,
                                                args,
                                            )
                                        },
                                    );
                                } else {
                                    engine.register_fn(
                                        &m.name,
                                        |callctx: NativeCallContext,
                                         obj: &mut RhaiStorePlugin,
                                         ctx: Arc<Mutex<InvocationContext>>,
                                         args: Value| {
                                            obj.invoke_return_page(
                                                callctx.fn_name(),
                                                ctx,
                                                vec![Dynamic::from(args)],
                                            )
                                        },
                                    );
                                    engine.register_fn(
                                        &m.name,
                                        |callctx: NativeCallContext,
                                         obj: &mut RhaiStorePlugin,
                                         ctx: Arc<Mutex<InvocationContext>>,
                                         args: Dynamic| {
                                            obj.invoke_return_page(
                                                callctx.fn_name(),
                                                ctx,
                                                vec![args],
                                            )
                                        },
                                    );
                                }
                            } else if m.return_vec {
                                if m.params_vec {
                                    engine.register_fn(
                                        &m.name,
                                        |callctx: NativeCallContext,
                                         obj: &mut RhaiStorePlugin,
                                         ctx: Arc<Mutex<InvocationContext>>,
                                         args: Array| {
                                            obj.invoke_return_vec(callctx.fn_name(), ctx, args)
                                        },
                                    );
                                    engine.register_fn(
                                        &m.name,
                                        |callctx: NativeCallContext,
                                         obj: &mut RhaiStorePlugin,
                                         ctx: Arc<Mutex<InvocationContext>>,
                                         args: Vec<Value>| {
                                            obj.invoke_return_vec_json(callctx.fn_name(), ctx, args)
                                        },
                                    );
                                } else {
                                    engine.register_fn(
                                        &m.name,
                                        |callctx: NativeCallContext,
                                         obj: &mut RhaiStorePlugin,
                                         ctx: Arc<Mutex<InvocationContext>>,
                                         args: Value| {
                                            obj.invoke_return_vec(
                                                callctx.fn_name(),
                                                ctx,
                                                vec![Dynamic::from(args)],
                                            )
                                        },
                                    );
                                    engine.register_fn(
                                        &m.name,
                                        |callctx: NativeCallContext,
                                         obj: &mut RhaiStorePlugin,
                                         ctx: Arc<Mutex<InvocationContext>>,
                                         args: Dynamic| {
                                            obj.invoke_return_vec(
                                                callctx.fn_name(),
                                                ctx,
                                                vec![args],
                                            )
                                        },
                                    );
                                }
                            } else if m.params_vec {
                                engine.register_fn(
                                    &m.name,
                                    |callctx: NativeCallContext,
                                     obj: &mut RhaiStorePlugin,
                                     ctx: Arc<Mutex<InvocationContext>>,
                                     args: Array| {
                                        obj.invoke_return_option(callctx.fn_name(), ctx, args)
                                    },
                                );
                                engine.register_fn(
                                    &m.name,
                                    |callctx: NativeCallContext,
                                     obj: &mut RhaiStorePlugin,
                                     ctx: Arc<Mutex<InvocationContext>>,
                                     args: Vec<Value>| {
                                        obj.invoke_return_option_json(callctx.fn_name(), ctx, args)
                                    },
                                );
                            } else {
                                engine.register_fn(
                                    &m.name,
                                    |callctx: NativeCallContext,
                                     obj: &mut RhaiStorePlugin,
                                     ctx: Arc<Mutex<InvocationContext>>,
                                     args: Value| {
                                        obj.invoke_return_option(
                                            callctx.fn_name(),
                                            ctx,
                                            vec![Dynamic::from(args)],
                                        )
                                    },
                                );
                                engine.register_fn(
                                    &m.name,
                                    |callctx: NativeCallContext,
                                     obj: &mut RhaiStorePlugin,
                                     ctx: Arc<Mutex<InvocationContext>>,
                                     args: Dynamic| {
                                        obj.invoke_return_option(callctx.fn_name(), ctx, vec![args])
                                    },
                                );
                            }
                        });
                    }
                });
            }
        });
    }

    pub fn invoke_return_option(
        &mut self,
        method: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Array,
    ) -> Result<Option<Value>, Box<EvalAltResult>> {
        let vec_args = args
            .into_iter()
            .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
            .collect();
        self.invoke_return_option_json(method, ctx, vec_args)
    }

    pub fn invoke_return_option_json(
        &mut self,
        method: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Option<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#{}", self.uri, method);
        let call_uri2 = call_uri.clone();
        // log::warn!("call uri in plugin {call_uri}");
        pin_blockon_async_v2!(async move {
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_one(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub fn invoke_return_vec(
        &mut self,
        method: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Array,
    ) -> Result<Vec<Value>, Box<EvalAltResult>> {
        let vec_args = args
            .into_iter()
            .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
            .collect();
        self.invoke_return_vec_json(method, ctx, vec_args)
    }

    pub fn invoke_return_vec_json(
        &mut self,
        method: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#{}", self.uri, method);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_vec(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }

    pub fn invoke_return_page(
        &mut self,
        method: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Array,
    ) -> Result<Page<Value>, Box<EvalAltResult>> {
        let vec_args = args
            .into_iter()
            .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
            .collect();
        self.invoke_return_page_json(method, ctx, vec_args)
    }

    pub fn invoke_return_page_json(
        &mut self,
        method: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Page<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#{}", self.uri, method);
        let call_uri2 = call_uri.clone();
        pin_blockon_async_v2!(async move {
            // Box::new(ret) as Box<dyn Any + Send + Sync>
            match MxStoreService::invoke_return_page(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {t:?}");
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {err:?}");
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            }
        })
        .unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(call_uri2),
            Position::new(1, 1),
        ))))
    }
}

/**
 * function that to be used as create the object
 */
pub fn log_info(msg: &str)  {
    log::info!("{msg}");
}

/**
 * function that to be used as create the object
 */
pub fn log_warn(msg: &str) {
    log::warn!("{msg}");
}

/**
 * function that to be used as create the object
 */
pub fn require_create(uri: &str) -> rhai::Dynamic {
    if uri.starts_with("object://") {
        rhai::Dynamic::from(RhaiStoreObject::new(uri))
    } else if uri.starts_with("query://") {
        rhai::Dynamic::from(RhaiStoreQuery::new(uri))
    } else {
        rhai::Dynamic::from(RhaiStorePlugin::new(uri))
    }
}

/**
 * a value getter/setter holder for serde_json::Value
 */
pub struct ValueGetterSetter {}

impl ValueGetterSetter {
    pub fn parse_json_object(text: &str) -> Value {
        serde_json::from_str(text).unwrap_or(Value::String(text.to_owned()))
    }

    pub fn create_json_object() -> Value {
        Value::Object(Map::new())
    }

    pub fn create_json_string(val: &str) -> Value {
        Value::String(val.to_string())
    }

    pub fn create_json_bool(val: bool) -> Value {
        Value::Bool(val)
    }

    pub fn create_json_null() -> Value {
        Value::Null
    }

    pub fn create_json_array() -> Value {
        Value::Array(vec![])
    }

    pub fn create_json_array_1(val: Value) -> Value {
        Value::Array(vec![val])
    }

    pub fn create_json_array_2(val: Value, val2: Value) -> Value {
        Value::Array(vec![val, val2])
    }

    pub fn create_json_array_3(val: Value, val2: Value, val3: Value) -> Value {
        Value::Array(vec![val, val2, val3])
    }

    #[allow(clippy::ptr_arg)]
    pub fn push_item(mp: &mut Vec<Value>, val: Value) {
        mp.push(val);
    }

    pub fn remove_item_i64(mp: &mut Vec<Value>, idx: i64) {
        mp.remove(idx as usize);
    }

    pub fn remove_item_u64(mp: &mut Vec<Value>, idx: u64) {
        mp.remove(idx as usize);
    }

    pub fn to_array(val: &mut Value) -> Vec<Value> {
        match val {
            Value::Array(tsc) => tsc.to_owned(),
            _ => vec![val.to_owned()],
        }
    }

    pub fn to_array_option(val: &mut Option<Value>) -> Vec<Value> {
        match val {
            Some(tval) => Self::to_array(tval),
            None => vec![],
        }
    }

    pub fn canonicalized_query_asc(arg: &mut Value) -> Value {
        Self::canonicalized_query(arg, false)
    }

    pub fn canonicalized_query(arg: &mut Value, desc: bool) -> Value {
        if let Value::Object(val) = arg {
            let mut sorted = val.keys().map(|f| f.to_owned()).collect::<Vec<String>>();
            if desc {
                sorted.sort_by(|a, b| b.cmp(a));
            } else {
                sorted.sort();
            }
            let mut text = vec![];
            for key in sorted {
                let val_str = val
                    .get(&key)
                    .map(|f| f.as_str().unwrap_or_default())
                    .unwrap_or_default();
                text.push(format!("{key}={val_str}"));
            }
            let ret = text.join("&");
            Value::String(ret)
        } else {
            Value::Null
        }
    }

    pub fn create_json_from_map(obj: rhai::Map) -> Value {
        match serde_json::to_value(obj) {
            Ok(t) => t,
            Err(err) => {
                log::debug!("ser error: {err}");
                Value::Null
            }
        }
    }

    pub fn create_json_from_array(arrays: rhai::Array) -> Vec<Value> {
        arrays
            .iter()
            .map(|s| serde_json::to_value(s).unwrap_or(Value::Null))
            .collect()
    }

    pub fn create_json_number(nb: Number) -> Value {
        Value::Number(nb)
    }

    pub fn create_paged_json(
        records: Vec<Value>,
        total: i64,
        page_no: i64,
        page_size: i64,
    ) -> Page<Value> {
        Page::<Value>::new_total(page_no as u64, page_size as u64, total as u64)
            .set_records(records)
    }

    pub fn create_paged_json_nototal(
        records: Vec<Value>,
        page_no: i64,
        page_size: i64,
    ) -> Page<Value> {
        Page::<Value>::new_total(page_no as u64, page_size as u64, 0).set_records(records)
    }

    pub fn create_paged_json_empty() -> Page<Value> {
        Page::<Value>::new_total(0, 0, 0)
    }

    pub fn create_paged_json_array(
        records: Array,
        total: i64,
        page_no: i64,
        page_size: i64,
    ) -> Page<Value> {
        Page::<Value>::new_total(page_no as u64, page_size as u64, total as u64)
            .set_records(records.into_iter().map(|f| f.cast::<Value>()).collect())
    }

    pub fn create_paged_json_nototal_array(
        records: Array,
        page_no: i64,
        page_size: i64,
    ) -> Page<Value> {
        Page::<Value>::new_total(page_no as u64, page_size as u64, 0)
            .set_records(records.into_iter().map(|f| f.cast::<Value>()).collect())
    }

    pub fn to_rhai_object(mp: &mut Value) -> rhai::Map {
        match serde_json::from_value::<rhai::Map>(mp.to_owned()) {
            Ok(m) => m,
            Err(err) => {
                log::debug!("Could not converto rhai_object, {err:?}");
                rhai::Map::new()
            }
        }
    }

    pub fn to_rhai_array_value(mp: &mut Value) -> rhai::Array {
        match mp {
            Value::Array(st) => Self::to_rhai_array_vec(st),
            _ => {
                vec![rhai::Dynamic::from_map(Self::to_rhai_object(mp))]
            }
        }
    }

    pub fn to_rhai_array_option(mp: &mut Option<Value>) -> rhai::Array {
        match mp {
            None => vec![],
            Some(t) => Self::to_rhai_array_value(t),
        }
    }

    #[allow(clippy::ptr_arg)]
    pub fn to_rhai_array_vec(mp: &mut Vec<Value>) -> rhai::Array {
        mp.iter_mut()
            .map(|f| rhai::Dynamic::from_map(Self::to_rhai_object(f)))
            .collect()
    }

    pub fn to_rhai_object_option(mp: &mut Option<Value>) -> rhai::Map {
        match mp {
            Some(st) => match serde_json::from_value::<rhai::Map>(st.to_owned()) {
                Ok(m) => m,
                Err(err) => {
                    log::debug!("Could not converto rhai_object, {err:?}");
                    rhai::Map::new()
                }
            },
            None => rhai::Map::new(),
        }
    }

    pub fn is_paged(_mp: &mut Value) -> bool {
        false
    }

    pub fn is_paged_option(_mp: &mut Option<Value>) -> bool {
        false
    }

    pub fn is_paged_page(_mp: &mut Page<Value>) -> bool {
        true
    }

    #[allow(clippy::ptr_arg)]
    pub fn is_paged_vec(_mp: &mut Vec<Value>) -> bool {
        false
    }

    pub fn is_list(_mp: &mut Value) -> bool {
        false
    }

    pub fn is_list_option(_mp: &mut Option<Value>) -> bool {
        false
    }

    pub fn is_list_page(_mp: &mut Page<Value>) -> bool {
        false
    }

    #[allow(clippy::ptr_arg)]
    pub fn is_list_vec(_mp: &mut Vec<Value>) -> bool {
        true
    }

    #[allow(clippy::ptr_arg)]
    pub fn get_len_vec(mp: &mut Vec<Value>) -> i64 {
        mp.len() as i64
    }

    #[allow(clippy::ptr_arg)]
    pub fn get_len_option(mp: &mut Option<Value>) -> i64 {
        match mp {
            None => 0,
            Some(Value::Array(stt)) => stt.len() as i64,
            Some(_) => 1,
        }
    }

    #[allow(clippy::ptr_arg)]
    pub fn get_len_value(mp: &mut Value) -> i64 {
        match mp {
            Value::Array(stt) => stt.len() as i64,
            Value::Null => 0,
            _ => 1,
        }
    }

    #[allow(clippy::ptr_arg)]
    pub fn is_empty_vec(mp: &mut Vec<Value>) -> bool {
        mp.is_empty()
    }

    #[allow(clippy::ptr_arg)]
    pub fn is_empty_option(mp: &mut Option<Value>) -> bool {
        mp.as_ref().map(|f| f.is_null()).unwrap_or(true)
    }

    #[allow(clippy::ptr_arg)]
    pub fn is_empty_value(mp: &mut Value) -> bool {
        match mp {
            Value::Null => true,
            Value::Array(tc) => tc.is_empty(),
            Value::Object(ct) => ct.is_empty(),
            _ => false,
        }
    }

    #[allow(clippy::ptr_arg)]
    pub fn get_item_i64(mp: &mut Vec<Value>, val: i64) -> Value {
        let s_val = val as usize;
        if mp.len() > s_val {
            mp[val as usize].clone()
        } else {
            Value::Null
        }
    }

    #[allow(clippy::ptr_arg)]
    pub fn get_item_u64(mp: &mut Vec<Value>, val: u64) -> Value {
        let s_val = val as usize;
        if mp.len() > s_val {
            mp[val as usize].clone()
        } else {
            Value::Null
        }
    }

    #[allow(clippy::ptr_arg)]
    pub fn set_item_i64(mp: &mut Vec<Value>, idx: i64, val: Value) {
        mp[idx as usize] = val
    }

    #[allow(clippy::ptr_arg)]
    pub fn set_item_u64(mp: &mut Vec<Value>, idx: u64, val: Value) {
        mp[idx as usize] = val
    }

    #[allow(clippy::ptr_arg)]
    pub fn get_item_usize(mp: &mut Vec<Value>, val: usize) -> Value {
        mp[val].clone()
    }

    pub fn set_records(mp: &mut Page<Value>, rec: Vec<Value>) -> Page<Value> {
        mp.clone().set_records(rec)
    }

    pub fn set_page_no(mp: &mut Page<Value>, arg: u64) -> Page<Value> {
        mp.clone().set_page_no(arg)
    }

    pub fn set_page_size(mp: &mut Page<Value>, arg: u64) -> Page<Value> {
        mp.clone().set_page_size(arg)
    }

    pub fn set_total(mp: &mut Page<Value>, total: u64) -> Page<Value> {
        mp.clone().set_total(total)
    }

    pub fn get_page_no(mp: &mut Page<Value>) -> u64 {
        mp.page_no()
    }

    pub fn get_page_size(mp: &mut Page<Value>) -> u64 {
        mp.page_size()
    }

    pub fn get_total(mp: &mut Page<Value>) -> u64 {
        mp.total()
    }

    pub fn get_records(mp: &mut Page<Value>) -> Vec<Value> {
        mp.records.clone()
        // mp.get_records().clone()
    }

    pub fn get(mp: &mut Value, name: &str) -> Value {
        mp.get(name).cloned().unwrap_or(Value::Null)
    }

    pub fn select(mp: &mut Value, path: &str) -> Value {
        json_path_get(mp, path).unwrap_or(Value::Null)
    }

    pub fn select_option(mp: &mut Option<Value>, path: &str) -> Value {
        if let Some(mp) = mp {
            json_path_get(mp, path).unwrap_or(Value::Null)
        } else {
            Value::Null
        }
    }

    pub fn get_option_value(mp: &mut Option<Value>, name: &str) -> Value {
        match mp {
            None => Value::Null,
            Some(np) => np.clone().get(name).cloned().unwrap_or(Value::Null),
        }
    }

    pub fn to_debug_option(mp: &mut Option<Value>) -> ImmutableString {
        match mp {
            None => "None".into(),
            Some(val) => {
                if val.is_string() {
                    val.as_str().unwrap_or_default().into()
                } else {
                    serde_json::to_string(val)
                        .unwrap_or("None".to_string())
                        .into()
                }
            }
        }
    }

    pub fn to_debug(mp: &mut Value) -> ImmutableString {
        if mp.is_string() {
            mp.as_str().unwrap_or_default().into()
        } else {
            serde_json::to_string(mp)
                .unwrap_or("None".to_string())
                .into()
        }
    }

    pub fn to_debug_vec(mp: &mut Vec<Value>) -> ImmutableString {
        serde_json::to_string(mp)
            .unwrap_or("None".to_string())
            .into()
    }

    pub fn to_debug_page(mp: &mut Page<Value>) -> ImmutableString {
        serde_json::to_string(mp)
            .unwrap_or("None".to_string())
            .into()
    }

    pub fn unwrap_option(mp: &mut Option<Value>) -> Value {
        match mp {
            Some(m) => m.to_owned(),
            None => Value::Null,
        }
    }

    pub fn unwrap_value(mp: &mut Value) -> Value {
        mp.to_owned()
    }

    pub fn set(mp: &mut Value, name: &str, val: Value) {
        mp[name] = val;
    }

    pub fn set_option(mp: &mut Option<Value>, name: &str, val: Value) {
        match mp.clone() {
            Some(mut xmp) => {
                xmp[name] = val;
                mp.replace(xmp);
            }
            None => {
                let mut mpobj = Map::new();
                mpobj.insert(name.to_owned(), val);
                mp.replace(Value::Object(mpobj));
            }
        }
    }

    pub fn set_string(mp: &mut Value, name: &str, val: &str) {
        mp[name] = Value::String(val.to_owned());
    }

    pub fn set_string_option(mp: &mut Option<Value>, name: &str, val: &str) {
        match mp.clone() {
            Some(mut xmp) => {
                xmp[name] = Value::String(val.to_owned());
                mp.replace(xmp);
            }
            None => {
                let mut mpobj = Map::new();
                mpobj.insert(name.to_owned(), Value::String(val.to_owned()));
                mp.replace(Value::Object(mpobj));
            }
        }
    }

    pub fn set_bool(mp: &mut Value, name: &str, val: bool) {
        mp[name] = Value::Bool(val);
    }

    pub fn set_bool_option(mp: &mut Option<Value>, name: &str, val: bool) {
        match mp.clone() {
            Some(mut xmp) => {
                xmp[name] = Value::Bool(val);
                mp.replace(xmp);
            }
            None => {
                let mut mpobj = Map::new();
                mpobj.insert(name.to_owned(), Value::Bool(val));
                mp.replace(Value::Object(mpobj));
            }
        }
    }

    pub fn set_i64(mp: &mut Value, name: &str, val: i64) {
        mp[name] = Value::Number(Number::from(val));
    }

    pub fn set_i64_option(mp: &mut Option<Value>, name: &str, val: i64) {
        match mp.clone() {
            Some(mut xmp) => {
                xmp[name] = Value::Number(Number::from(val));
                mp.replace(xmp);
            }
            None => {
                let mut mpobj = Map::new();
                mpobj.insert(name.to_owned(), Value::Number(Number::from(val)));
                mp.replace(Value::Object(mpobj));
            }
        }
    }

    pub fn set_u64(mp: &mut Value, name: &str, val: u64) {
        mp[name] = Value::Number(Number::from(val));
    }

    pub fn set_u64_option(mp: &mut Option<Value>, name: &str, val: u64) {
        match mp.clone() {
            Some(mut xmp) => {
                xmp[name] = Value::Number(Number::from(val));
                mp.replace(xmp);
            }
            None => {
                let mut mpobj = Map::new();
                mpobj.insert(name.to_owned(), Value::Number(Number::from(val)));
                mp.replace(Value::Object(mpobj));
            }
        }
    }

    pub fn set_f64(mp: &mut Value, name: &str, val: f64) {
        mp[name] = Value::Number(Number::from_f64(val).unwrap());
    }

    pub fn set_f64_option(mp: &mut Option<Value>, name: &str, val: f64) {
        match mp.clone() {
            Some(mut xmp) => {
                xmp[name] = Value::Number(Number::from_f64(val).unwrap());
                mp.replace(xmp);
            }
            None => {
                let mut mpobj = Map::new();
                mpobj.insert(
                    name.to_owned(),
                    Value::Number(Number::from_f64(val).unwrap()),
                );
                mp.replace(Value::Object(mpobj));
            }
        }
    }

    pub fn push(mp: &mut Value, val: Value) -> Value {
        if !mp.is_array() {
            log::warn!("unsupport function for Non-Array JSON Object");
            return mp.clone();
        }

        match mp.as_array_mut() {
            Some(list) => {
                list.push(val);
            }
            None => {
                log::warn!("unsupport function for Non-Array JSON Object");
            }
        }
        mp.clone()
    }

    pub fn create_query_condition() -> Value {
        json!(QueryCondition::default())
    }

    pub fn create_query_condition_args(and: Vec<Value>) -> Value {
        json!(QueryCondition {
            and: and
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).map(Some).unwrap_or(None))
                .collect(),
            or: vec![],
            ..Default::default()
        })
    }

    pub fn create_query_condition_args_2(and: Value, or: Value) -> Value {
        let and_ci = serde_json::from_value::<ConditionItem>(and).unwrap_or_default();
        let or_ci = serde_json::from_value::<ConditionItem>(or).unwrap_or_default();
        json!(QueryCondition {
            and: vec![and_ci],
            or: vec![or_ci],
            ..Default::default()
        })
    }

    pub fn create_query_condition_args_1(and: Value) -> Value {
        let ci = serde_json::from_value::<ConditionItem>(and).unwrap_or_default();
        json!(QueryCondition {
            and: vec![ci],
            or: vec![],
            ..Default::default()
        })
    }

    pub fn create_query_condition_args_3(and: Value, or: Value, page: Value) -> Value {
        let and_ci = serde_json::from_value::<ConditionItem>(and).unwrap_or_default();
        let or_ci = serde_json::from_value::<ConditionItem>(or).unwrap_or_default();
        let paging = serde_json::from_value::<Option<IPaging>>(page).unwrap_or(None);

        json!(QueryCondition {
            and: vec![and_ci],
            or: vec![or_ci],
            paging,
            ..Default::default()
        })
    }

    pub fn create_query_condition_args_4(and: Value, or: Value, ord: Value, page: Value) -> Value {
        let and_ci = serde_json::from_value::<ConditionItem>(and).unwrap_or_default();
        let or_ci = serde_json::from_value::<ConditionItem>(or).unwrap_or_default();
        let paging = serde_json::from_value::<Option<IPaging>>(page).unwrap_or(None);
        let ord_oi = serde_json::from_value::<OrdianlItem>(ord).unwrap_or_default();

        json!(QueryCondition {
            and: vec![and_ci],
            or: vec![or_ci],
            sorts: vec![ord_oi],
            paging,
            ..Default::default()
        })
    }

    pub fn create_query_condition_args_5(
        and: Value,
        or: Value,
        g: Value,
        ord: Value,
        page: Value,
    ) -> Value {
        let and_ci = serde_json::from_value::<ConditionItem>(and).unwrap_or_default();
        let or_ci = serde_json::from_value::<ConditionItem>(or).unwrap_or_default();
        let g_oi = serde_json::from_value::<OrdianlItem>(g).unwrap_or_default();
        let ord_oi = serde_json::from_value::<OrdianlItem>(ord).unwrap_or_default();
        let paging = serde_json::from_value::<Option<IPaging>>(page).unwrap_or(None);

        json!(QueryCondition {
            and: vec![and_ci],
            or: vec![or_ci],
            group_by: vec![g_oi],
            sorts: vec![ord_oi],
            paging,
            dont_update_exist: None,
            retrieved_detail: None
        })
    }

    pub fn create_ordianl_item() -> Value {
        json!(OrdianlItem::default())
    }

    pub fn create_ordianl_item_args(field: &str, asc: bool) -> Value {
        json!(OrdianlItem {
            field: field.to_owned(),
            sort_asc: asc
        })
    }

    pub fn create_condition() -> Value {
        json!(ConditionItem::default())
    }

    pub fn create_condition_args(field: &str, op: &str, val: Value) -> Value {
        json!(ConditionItem {
            field: field.to_owned(),
            op: op.to_owned(),
            value: val,
            value2: Value::Null,
            ..Default::default()
        })
    }
}

pub struct InvocationContextGetterSetter;

impl InvocationContextGetterSetter {
    pub fn get(ctx: &mut Arc<Mutex<InvocationContext>>, name: &str) -> rhai::Dynamic {
        if let Some(t) = ctx.lock().unwrap().get_(name) {
            if let Some(bx) = t.downcast_ref::<Option<Value>>() {
                return rhai::Dynamic::from(bx.clone());
            }
            if let Some(bx) = t.downcast_ref::<Value>() {
                return rhai::Dynamic::from(bx.clone());
            }
            if let Some(bx) = t.downcast_ref::<Vec<Value>>() {
                return rhai::Dynamic::from(bx.clone());
            }
            if let Some(bx) = t.downcast_ref::<Page<Value>>() {
                return rhai::Dynamic::from(bx.clone());
            }
        };
        rhai::Dynamic::from(Value::Null)
    }

    pub fn set_username(ctx: &mut Arc<Mutex<InvocationContext>>, username: &str, userid: &str) {
        ctx.lock().unwrap().set_current_user(username, Some(userid));
    }

    pub fn set_username_withid(ctx: &mut Arc<Mutex<InvocationContext>>, username: &str, userid: i64) {
        if userid == 0 {
            ctx.lock().unwrap().set_current_user(username, None);
        } else {
            ctx.lock().unwrap().set_current_user(username, Some(&format!("{userid}")));
        }
    }


    pub fn get_username(ctx: &mut Arc<Mutex<InvocationContext>>) -> rhai::Dynamic {
        match ctx.lock().unwrap().get_current_user() {
            Some(us) => {
                rhai::Dynamic::from_str(&us).unwrap_or(rhai::Dynamic::default())
            },
            None => {
                rhai::Dynamic::default()
            }
        }
    }

    pub fn get_userid(ctx: &mut Arc<Mutex<InvocationContext>>) -> rhai::Dynamic {
        match ctx.lock().unwrap().get_current_userid() {
            Some(us) => {
                rhai::Dynamic::from_str(&us).unwrap_or(rhai::Dynamic::default())
            },
            None => {
                rhai::Dynamic::default()
            }
        }
    }    

    /**
     * 获取当前用户的Authorization Token, JwtToken
     */
    pub fn get_authorization(ctx: &mut Arc<Mutex<InvocationContext>>) -> rhai::Dynamic {
        match ctx.lock().unwrap().get_current_authorization() {
            Some(us) => {
                log::info!("Get the current authorization: {us}");
                rhai::Dynamic::from_str(&us).unwrap_or(rhai::Dynamic::default())
            },
            None => {
                log::info!("Get ghe current authorization with None");
                rhai::Dynamic::default()
            }
        }
    }

    pub fn set_failed(ctx: &mut Arc<Mutex<InvocationContext>>) {
        ctx.lock().unwrap().set_failed();
    }

    pub fn get_return(ctx: &mut Arc<Mutex<InvocationContext>>) -> rhai::Dynamic {
        Self::get(ctx, "RETURN_VALUE")
    }

    pub fn get_status(ctx: &mut Arc<Mutex<InvocationContext>>) -> rhai::Dynamic {
        // Self::get(ctx, "RETURN_STATUS")
        let st = ctx.lock().unwrap().get_status();
        rhai::Dynamic::from(st)
    }

    pub fn get_message(ctx: &mut Arc<Mutex<InvocationContext>>) -> rhai::Dynamic {
        let st = ctx.lock().unwrap().get_message();
        rhai::Dynamic::from(st)
    }

    pub fn set_status(ctx: &mut Arc<Mutex<InvocationContext>>, val: rhai::Dynamic) {
        ctx.lock()
            .unwrap()
            .set_status(val.as_int().unwrap_or_default());
    }

    pub fn set_message(ctx: &mut Arc<Mutex<InvocationContext>>, val: rhai::Dynamic) {
        let msg = val.to_string();
        ctx.lock().unwrap().set_message(&msg);
    }

    pub fn get_return_rawdata(ctx: &mut Arc<Mutex<InvocationContext>>) -> rhai::Dynamic {
        let st = ctx.lock().unwrap().get_return_rawdata();
        rhai::Dynamic::from(st)
    }

    pub fn set_return_rawdata(ctx: &mut Arc<Mutex<InvocationContext>>, val: rhai::Dynamic) {
        ctx.lock()
            .unwrap()
            .set_return_rawdata(val.as_bool().unwrap_or_default());
    }

    pub fn get_response_xml(ctx: &mut Arc<Mutex<InvocationContext>>) -> rhai::Dynamic {
        let st = ctx.lock().unwrap().get_response_xml();
        rhai::Dynamic::from(st)
    }

    pub fn set_response_xml(ctx: &mut Arc<Mutex<InvocationContext>>, val: rhai::Dynamic) {
        ctx.lock()
            .unwrap()
            .set_response_xml(val.as_bool().unwrap_or_default());
    }

    pub fn set(ctx: &mut Arc<Mutex<InvocationContext>>, name: &str, val: rhai::Dynamic) {
        ctx.lock().unwrap().insert(name, val);
    }

    pub fn set_option(ctx: &mut Arc<Mutex<InvocationContext>>, name: &str, val: Option<Value>) {
        ctx.lock().unwrap().insert(name, val);
    }

    pub fn set_value(ctx: &mut Arc<Mutex<InvocationContext>>, name: &str, val: Value) {
        ctx.lock().unwrap().insert(name, val);
    }

    pub fn set_vec(ctx: &mut Arc<Mutex<InvocationContext>>, name: &str, val: Vec<Value>) {
        ctx.lock().unwrap().insert(name, val);
    }

    pub fn set_paged(ctx: &mut Arc<Mutex<InvocationContext>>, name: &str, val: Page<Value>) {
        ctx.lock().unwrap().insert(name, val);
    }

    pub fn set_return(ctx: &mut Arc<Mutex<InvocationContext>>, val: rhai::Dynamic) {
        ctx.lock().unwrap().insert("RETURN_VALUE", val);
    }

    pub fn set_return_option(ctx: &mut Arc<Mutex<InvocationContext>>, val: Option<Value>) {
        ctx.lock().unwrap().insert("RETURN_VALUE", val);
    }

    pub fn set_return_value(ctx: &mut Arc<Mutex<InvocationContext>>, val: Value) {
        ctx.lock().unwrap().insert("RETURN_VALUE", Some(val));
    }

    pub fn set_return_vec(ctx: &mut Arc<Mutex<InvocationContext>>, val: Vec<Value>) {
        ctx.lock()
            .unwrap()
            .insert("RETURN_VALUE", Some(Value::Array(val)));
    }

    pub fn set_return_paged(ctx: &mut Arc<Mutex<InvocationContext>>, val: Page<Value>) {
        ctx.lock().unwrap().insert("RETURN_VALUE", Some(json!(val)));
    }

    pub fn get_string(ctx: &mut Arc<Mutex<InvocationContext>>, name: &str) -> String {
        if let Ok(t) = ctx.lock().unwrap().get::<String>(name) {
            t.clone()
        } else {
            String::new()
        }
    }

    pub fn get_i64(ctx: &mut Arc<Mutex<InvocationContext>>, name: &str) -> i64 {
        if let Ok(t) = ctx.lock().unwrap().get::<i64>(name) {
            *t
        } else {
            0i64
        }
    }

    pub fn get_u64(ctx: &mut Arc<Mutex<InvocationContext>>, name: &str) -> u64 {
        if let Ok(t) = ctx.lock().unwrap().get::<u64>(name) {
            *t
        } else {
            0u64
        }
    }

    pub fn get_bool(ctx: &mut Arc<Mutex<InvocationContext>>, name: &str) -> bool {
        if let Ok(t) = ctx.lock().unwrap().get::<bool>(name) {
            *t
        } else {
            false
        }
    }

    pub fn get_hook_uri(ctx: &mut Arc<Mutex<InvocationContext>>) -> String {
        if let Ok(t) = ctx.lock().unwrap().get::<String>("HOOK_HANDLE_URI") {
            t.clone()
        } else {
            String::new()
        }
    }

    pub fn lock(
        ctx: &mut Arc<Mutex<InvocationContext>>,
        ns: &str,
        key: &str,
        expr: i64,
    ) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
        match ctx.lock().unwrap().lock(ns, key, expr) {
            Ok(t) => Ok(rhai::Dynamic::from(t)),
            Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
                "Unable lock the key".to_string().to_owned(),
                err.into_boxed_dyn_error(),
            ))),
        }
    }

    pub fn lock_ns(
        ctx: &mut Arc<Mutex<InvocationContext>>,
        ns: &str,
        key: &str,
    ) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
        Self::lock(ctx, ns, key, 30)
    }

    pub fn unlock(
        ctx: &mut Arc<Mutex<InvocationContext>>,
        ns: &str,
        key: &str,
    ) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
        match ctx.lock().unwrap().unlock(ns, key) {
            Ok(t) => Ok(rhai::Dynamic::from(t)),
            Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
                "Unable unlock the key".to_string().to_owned(),
                err.into_boxed_dyn_error(),
            ))),
        }
    }

    pub fn release_connections(ctx: &mut Arc<Mutex<InvocationContext>>) {
        let _ = pin_blockon_async_v2!(async move {
            release_all_connections(ctx).await;
            // ctx.lock().unwrap().finalize_async().await;
        });
    }
}

pub fn json_path_get(t: &Value, path: &str) -> Option<Value> {
    let jspath = if path.starts_with("$.") {
        path.to_owned()
    } else {
        format!("$.{path}")
    };

    if let Ok(inst) = jsonpath_rust::JsonPathInst::from_str(&jspath) {
        let slice = inst.find_slice(t);
        if slice.is_empty() {
            None
        } else if slice.len() == 1 {
            let ret = &slice[0].clone();
            Some(ret.to_owned())
        } else {
            let ret = Value::Array(slice.into_iter().map(|f| f.to_owned()).collect());
            Some(ret)
        }
    } else {
        None
    }
}
