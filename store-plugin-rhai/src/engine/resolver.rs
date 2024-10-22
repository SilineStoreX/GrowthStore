use chimes_store_core::config::{ConditionItem, IPaging, OrdianlItem, QueryCondition};
use chimes_store_core::pin_blockon_async;
use chimes_store_core::service::sdk::InvokeUri;
use chimes_store_core::service::{invoker::InvocationContext, starter::MxStoreService};
use rbatis::{IPage, IPageRequest, Page};
use rhai::{
    Array, CustomType, Dynamic, Engine, EvalAltResult, ImmutableString, Module, ModuleResolver, NativeCallContext, Position, TypeBuilder
};
use serde_json::{json, Map, Number, Value};
use std::any::Any;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::{collections::HashMap, mem::MaybeUninit, sync::Once};
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
        log::info!("Source: {:?} of path: {}", source, path);
        println!("Source: {:?} of path: {}", source, path);
        Ok(rhai::Shared::new(
            RhaiStoreModule::get_store_module(path).create_module(),
        ))
    }
}

pub struct RhaiStoreModule(pub(crate) String);

impl RhaiStoreModule {
    pub fn get_mut() -> &'static mut HashMap<String, RhaiStoreModule> {
        // 使用MaybeUninit延迟初始化
        static mut RHAI_STORED_MODULE_MAP: MaybeUninit<HashMap<String, RhaiStoreModule>> =
            MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static RHAI_STORE_MODULE_ONCE: Once = Once::new();

        RHAI_STORE_MODULE_ONCE.call_once(|| unsafe {
            RHAI_STORED_MODULE_MAP.as_mut_ptr().write(HashMap::new());
        });

        unsafe { &mut (*RHAI_STORED_MODULE_MAP.as_mut_ptr()) }
    }

    pub fn get_store_module(path: &str) -> &'static RhaiStoreModule {
        if Self::get_mut().contains_key(path) {
            Self::get_mut().get(path).unwrap()
        } else {
            Self::get_mut().insert(path.to_owned(), RhaiStoreModule(path.to_owned()));
            Self::get_mut().get(path).unwrap()
        }
    }

    fn create_module(&'static self) -> Module {
        let mut module = Module::new();

        fn test_select(ns: &str) -> Result<Option<Value>, Box<EvalAltResult>> {
            Ok(Some(json!(MxStoreService::get(ns).unwrap().get_objects())))
        }

        module.set_native_fn("select", || test_select(&self.0));
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


    pub fn aes_encrypt(col: &mut Self, text: &str) -> Result<String, Box<EvalAltResult>> {
        let call_uri = format!("{}#find_one", col.uri);
        if let Ok(invoke_uri) = InvokeUri::parse(&call_uri) {
            if let Some(mxs) = MxStoreService::get(&invoke_uri.namespace) {
                Ok(mxs.aes_encode_text(text))
            } else {
                Err(Box::new(EvalAltResult::ErrorRuntime(
                    Dynamic::from(format!("namespace: {} was not be found.", &invoke_uri.namespace)),
                    Position::new(1, 1),
                )))
            }
        } else {
            Err(Box::new(EvalAltResult::ErrorRuntime(
                Dynamic::from(format!("special {} could not be parsed as InvokeURI.", &call_uri)),
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
                    Dynamic::from(format!("namespace: {} was not be found.", &invoke_uri.namespace)),
                    Position::new(1, 1),
                )))
            }
        } else {
            Err(Box::new(EvalAltResult::ErrorRuntime(
                Dynamic::from(format!("special {} could not be parsed as InvokeURI.", &call_uri)),
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
                    Dynamic::from(format!("namespace: {} was not be found.", &invoke_uri.namespace)),
                    Position::new(1, 1),
                )))
            }
        } else {
            Err(Box::new(EvalAltResult::ErrorRuntime(
                Dynamic::from(format!("special {} could not be parsed as InvokeURI.", &call_uri)),
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
                    Dynamic::from(format!("namespace: {} was not be found.", &invoke_uri.namespace)),
                    Position::new(1, 1),
                )))
            }
        } else {
            Err(Box::new(EvalAltResult::ErrorRuntime(
                Dynamic::from(format!("special {} could not be parsed as InvokeURI.", &call_uri)),
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
        pin_blockon_async!(async move {
            let ret = match MxStoreService::invoke_return_one(call_uri.clone(), ctx, vec![args]).await {
                Ok(t) => Ok(t),
                Err(err) => Err(Box::new(EvalAltResult::ErrorRuntime(
                    Dynamic::from(err.to_string()),
                    Position::new(1, 1),
                ))),
            };
            Box::new(ret) as Box<dyn Any + Send + Sync>
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
        pin_blockon_async!(async move {
            let ret = match MxStoreService::invoke_return_one(call_uri.clone(), ctx, args).await {
                Ok(t) => Ok(t),
                Err(err) => Err(Box::new(EvalAltResult::ErrorRuntime(
                    Dynamic::from(err.to_string()),
                    Position::new(1, 1),
                ))),
            };
            Box::new(ret) as Box<dyn Any + Send + Sync>
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
        
        pin_blockon_async!(async move {
            log::info!("invoker select return one {call_uri}");
            let ret = match MxStoreService::invoke_return_one(call_uri.clone(), ctx, vec![args]).await {
                Ok(t) => {
                    log::info!("OK: {:?}", t);
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {:?}", err);
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            };

            Box::new(ret) as Box<dyn Any + Send + Sync>
        }).unwrap_or(Err(Box::new(EvalAltResult::ErrorRuntime(
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
        pin_blockon_async!(async move {
            let ret = match MxStoreService::invoke_return_one(call_uri.clone(), ctx, vec![args]).await {
                Ok(t) => {
                    log::info!("OK: {:?}", t);
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {:?}", err);
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            };
            Box::new(ret) as Box<dyn Any + Send + Sync>
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
        pin_blockon_async!(async move {
            let ret = match MxStoreService::invoke_return_vec(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {:?}", t);
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {:?}", err);
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            };
            Box::new(ret) as Box<dyn Any + Send + Sync>
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
    ) -> Result<Vec<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#paged_query", self.uri);
        let call_uri2 = call_uri.clone();
        pin_blockon_async!(async move {
            let ret = match MxStoreService::invoke_return_vec(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {:?}", t);
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {:?}", err);
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            };
            Box::new(ret) as Box<dyn Any + Send + Sync>
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

    pub fn search(
        &mut self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, Box<EvalAltResult>> {
        let call_uri = format!("{}#search", self.uri);
        let call_uri2 = call_uri.clone();
        pin_blockon_async!(async move {
            let ret = match MxStoreService::invoke_return_vec(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {:?}", t);
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {:?}", err);
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            };
            Box::new(ret) as Box<dyn Any + Send + Sync>
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
        pin_blockon_async!(async move {
            let ret = match MxStoreService::invoke_return_page(call_uri.clone(), ctx, args).await {
                Ok(t) => {
                    log::info!("OK: {:?}", t);
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {:?}", err);
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            };
            Box::new(ret) as Box<dyn Any + Send + Sync>
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

    pub fn register(engine: &mut Engine) {
        engine
            .register_type_with_name::<RhaiStorePlugin>("PluginObject")
            .register_fn("new_plugin_object", RhaiStorePlugin::new)
            .register_fn(
                "invoke_return_option",
                RhaiStorePlugin::invoke_return_option,
            )
            .register_fn("invoke_return_vec", RhaiStorePlugin::invoke_return_vec)
            .register_fn("invoke_return_page", RhaiStorePlugin::invoke_return_page);

        MxStoreService::get_namespaces().into_iter().for_each(|ns| {
            log::info!("register {ns}");
            if let Some(nss) = MxStoreService::get(&ns) {
                nss.get_plugins().into_iter().for_each(|proto| {
                    let ns_protocol = format!("{}://{ns}/{}", proto.protocol, proto.name);
                    if let Some(pls) = MxStoreService::get_plugin_service(&ns_protocol) {
                        pls.get_metadata().into_iter().for_each(|m| {
                            log::info!("register method {} for {ns_protocol}", m.name);
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
        let call_uri = format!("{}#{}", self.uri, method);
        let call_uri2 = call_uri.clone();
        pin_blockon_async!(async move {
            let vec_args = args.into_iter().map(|d| d.cast::<Value>()).collect();
            let ret = match MxStoreService::invoke_return_one(call_uri.clone(), ctx, vec_args).await {
                Ok(t) => {
                    log::info!("OK: {:?}", t);
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {:?}", err);
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            };
            Box::new(ret) as Box<dyn Any + Send + Sync>
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
        let call_uri = format!("{}#{}", self.uri, method);
        let call_uri2 = call_uri.clone();
        pin_blockon_async!(async move {
            let vec_args = args.into_iter().map(|d| d.cast::<Value>()).collect();
            let ret = match MxStoreService::invoke_return_vec(call_uri.clone(), ctx, vec_args).await {
                Ok(t) => {
                    log::info!("OK: {:?}", t);
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {:?}", err);
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            };
            Box::new(ret) as Box<dyn Any + Send + Sync>
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
        let call_uri = format!("{}#{}", self.uri, method);
        let call_uri2 = call_uri.clone();
        pin_blockon_async!(async move {
            let vec_args = args.into_iter().map(|d| d.cast::<Value>()).collect();
            let ret = match MxStoreService::invoke_return_page(call_uri.clone(), ctx, vec_args).await {
                Ok(t) => {
                    log::info!("OK: {:?}", t);
                    Ok(t)
                }
                Err(err) => {
                    log::info!("Err: {:?}", err);
                    Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from(err.to_string()),
                        Position::new(1, 1),
                    )))
                }
            };
            Box::new(ret) as Box<dyn Any + Send + Sync>
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
    pub fn create_json_object() -> Value {
        Value::Object(Map::new())
    }

    pub fn create_json_null() -> Value {
        Value::Null
    }

    pub fn create_json_string(val: &str) -> Value {
        Value::String(val.to_string())
    }

    pub fn create_json_bool(val: bool) -> Value {
        Value::Bool(val)
    }

    pub fn create_json_array() -> Value {
        Value::Array(vec![])
    }

    pub fn to_array(val: &mut Value) -> Vec<Value> {
        match val {
            Value::Array(tsc) => tsc.to_owned(),
            _ => vec![val.to_owned()],
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
                let val_str = val.get(&key).map(|f| f.as_str().unwrap_or_default()).unwrap_or_default();
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
                log::debug!("ser error: {}", err);
                Value::Null
            }
        }
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
        Page::<Value>::new(page_no as u64, page_size as u64).set_records(records)
    }

    pub fn create_paged_json_empty() -> Page<Value> {
        Page::<Value>::new(0, 0)
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
        Page::<Value>::new(page_no as u64, page_size as u64)
            .set_records(records.into_iter().map(|f| f.cast::<Value>()).collect())
    }

    pub fn to_rhai_object(mp: &mut Value) -> rhai::Map {
        match serde_json::from_value::<rhai::Map>(mp.to_owned()) {
            Ok(m) => m,
            Err(err) => {
                log::debug!("Could not converto rhai_object, {:?}", err);
                rhai::Map::new()
            }
        }
    }

    pub fn to_rhai_object_option(mp: &mut Option<Value>) -> rhai::Map {
        match mp {
            Some(st) => match serde_json::from_value::<rhai::Map>(st.to_owned()) {
                Ok(m) => m,
                Err(err) => {
                    log::debug!("Could not converto rhai_object, {:?}", err);
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
    pub fn get_item_i64(mp: &mut Vec<Value>, val: i64) -> Value {
        mp[val as usize].clone()
    }

    #[allow(clippy::ptr_arg)]
    pub fn get_item_u64(mp: &mut Vec<Value>, val: u64) -> Value {
        mp[val as usize].clone()
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
        mp.get_records().clone()
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
            Some(val) => serde_json::to_string(val)
                .unwrap_or("None".to_string())
                .into(),
        }
    }

    pub fn to_debug(mp: &mut Value) -> ImmutableString {
        serde_json::to_string(mp)
            .unwrap_or("None".to_string())
            .into()
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
            None => Value::Null
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

    pub fn get_return(ctx: &mut Arc<Mutex<InvocationContext>>) -> rhai::Dynamic {
        Self::get(ctx, "RETURN_VALUE")
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
        ctx.lock().unwrap().insert("RETURN_VALUE", val);
    }

    pub fn set_return_vec(ctx: &mut Arc<Mutex<InvocationContext>>, val: Vec<Value>) {
        ctx.lock().unwrap().insert("RETURN_VALUE", val);
    }

    pub fn set_return_paged(ctx: &mut Arc<Mutex<InvocationContext>>, val: Page<Value>) {
        ctx.lock().unwrap().insert("RETURN_VALUE", val);
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
}

pub fn json_path_get(t: &Value, path: &str) -> Option<Value> {
    let jspath = if path.starts_with("$.") {
        path.to_owned()
    } else {
        format!("$.{}", path)
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
