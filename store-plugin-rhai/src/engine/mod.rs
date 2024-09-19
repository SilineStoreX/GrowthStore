use ::reqwest::Method;
use anyhow::anyhow;
use rbatis::Page;
use reqwest::RhaiHttpClient;
use resolver::{
    require_create, InvocationContextGetterSetter, RhaiStoreObject, RhaiStorePlugin,
    RhaiStoreQuery, RhaiStoreServiceResolver, ValueGetterSetter,
};
use rhai::Dynamic;
use std::{
    fs::File,
    io::{BufReader, Read},
    mem::MaybeUninit,
    path::Path,
    sync::{Arc, Mutex, Once},
};

use chimes_store_core::service::invoker::InvocationContext;
use serde_json::{Number, Value};

mod reqwest;
pub mod resolver;
pub mod common;

#[derive(Clone)]
pub struct EvalEngine {
    engine: Arc<rhai::Engine>,
}

impl EvalEngine {
    fn create() -> Self {
        let mut engin = rhai::Engine::new();
        engin.set_allow_anonymous_fn(true);
        engin.set_allow_if_expression(true);
        engin.set_allow_loop_expressions(true);
        engin.set_allow_looping(true);
        engin.set_allow_shadowing(true);
        engin.set_allow_switch_expression(true);
        engin.set_allow_statement_expression(true);
        // engin.set_default_tag(value); // should custom this
        engin.set_fail_on_invalid_map_property(true);
        engin.set_module_resolver(RhaiStoreServiceResolver::new()); // should custom this
        engin.set_max_array_size(0);
        engin.set_max_call_levels(128);
        engin.set_max_expr_depths(0, 0);
        engin.set_fast_operators(true);
        engin.set_max_string_size(0);
        engin.set_max_functions(1024);
        engin.set_max_modules(1024);
        engin.set_optimization_level(rhai::OptimizationLevel::Full);
        engin.set_max_variables(1024);
        engin.set_strict_variables(true);
        engin.register_fn("required", require_create);
        engin.register_fn("sha1_text", common::sha1_text);
        engin.register_fn("sha2_text", common::sha2_text);
        engin.register_fn("hmac_sha1", common::hmac_sha1_rhai);
        engin.register_fn("hmac_sha256", common::hmac_sha256_rhai);
        engin.register_fn("hmac_sha512", common::hmac_sha512_rhai);        
        engin.register_fn("md5string", common::text_md5);
        engin.register_fn("base64encode", common::text_base64_encode);
        engin.register_fn("base64decode", common::text_base64_decode);        
        engin.register_fn("http_request", RhaiHttpClient::sync_http_request);
        engin.register_fn("http_get", |url: &str, data: Value, opt: Value| {
            RhaiHttpClient::sync_http_request(url, Method::GET, data, Some(opt))
        });
        engin.register_fn("http_get", |url: &str, data: Value| {
            RhaiHttpClient::sync_http_request(url, Method::GET, data, None)
        });
        engin.register_fn("http_get", |url: &str| {
            RhaiHttpClient::sync_http_request(url, Method::GET, Value::Null, None)
        });
        engin.register_fn("http_post", |url: &str, data: Value, opt: Value| {
            RhaiHttpClient::sync_http_request(url, Method::POST, data, Some(opt))
        });
        engin.register_fn("http_post", |url: &str, data: Value| {
            RhaiHttpClient::sync_http_request(url, Method::POST, data, None)
        });
        engin.register_fn("http_put", |url: &str, data: Value, opt: Value| {
            RhaiHttpClient::sync_http_request(url, Method::PUT, data, Some(opt))
        });
        engin.register_fn("http_put", |url: &str, data: Value| {
            RhaiHttpClient::sync_http_request(url, Method::PUT, data, None)
        });
        engin.register_fn("http_delete", |url: &str, data: Value, opt: Value| {
            RhaiHttpClient::sync_http_request(url, Method::DELETE, data, Some(opt))
        });
        engin.register_fn("http_delete", |url: &str, data: Value| {
            RhaiHttpClient::sync_http_request(url, Method::DELETE, data, None)
        });
        engin.register_fn("http_delete", |url: &str| {
            RhaiHttpClient::sync_http_request(url, Method::DELETE, Value::Null, None)
        });
        engin.register_fn("http_option", |url: &str, opt: Value| {
            RhaiHttpClient::sync_http_request(url, Method::OPTIONS, Value::Null, Some(opt))
        });
        engin.register_fn("http_patch", |url: &str, opt: Value| {
            RhaiHttpClient::sync_http_request(url, Method::PATCH, Value::Null, Some(opt))
        });
        engin.register_fn("http_trace", |url: &str, opt: Value| {
            RhaiHttpClient::sync_http_request(url, Method::TRACE, Value::Null, Some(opt))
        });
        engin.register_fn("http_head", |url: &str, opt: Value| {
            RhaiHttpClient::sync_http_request(url, Method::HEAD, Value::Null, Some(opt))
        });
        engin.register_fn("http_connect", |url: &str| {
            RhaiHttpClient::sync_http_request(url, Method::CONNECT, Value::Null, None)
        });
        engin.register_fn("http_option", |url: &str| {
            RhaiHttpClient::sync_http_request(url, Method::OPTIONS, Value::Null, None)
        });
        engin.register_fn("http_patch", |url: &str| {
            RhaiHttpClient::sync_http_request(url, Method::PATCH, Value::Null, None)
        });
        engin.register_fn("http_trace", |url: &str| {
            RhaiHttpClient::sync_http_request(url, Method::TRACE, Value::Null, None)
        });
        engin.register_fn("http_head", |url: &str| {
            RhaiHttpClient::sync_http_request(url, Method::HEAD, Value::Null, None)
        });
        engin.register_fn("http_connect", |url: &str| {
            RhaiHttpClient::sync_http_request(url, Method::CONNECT, Value::Null, None)
        });

        engin
            .register_type_with_name::<Arc<Mutex<InvocationContext>>>("InvocationContext")
            .register_fn("get", InvocationContextGetterSetter::get)
            .register_fn("get_bool", InvocationContextGetterSetter::get_bool)
            .register_fn("get_i64", InvocationContextGetterSetter::get_i64)
            .register_fn("get_u64", InvocationContextGetterSetter::get_u64)
            .register_fn("get_string", InvocationContextGetterSetter::get_string)
            .register_fn("get_hook_uri", InvocationContextGetterSetter::get_hook_uri)
            .register_fn("set", InvocationContextGetterSetter::set)
            .register_fn("set", InvocationContextGetterSetter::set_option)
            .register_fn("set", InvocationContextGetterSetter::set_value)
            .register_fn("set", InvocationContextGetterSetter::set_vec)
            .register_fn("set", InvocationContextGetterSetter::set_paged)
            .register_fn("set_return", InvocationContextGetterSetter::set_return)
            .register_fn("set_return", InvocationContextGetterSetter::set_return_option)
            .register_fn("set_return", InvocationContextGetterSetter::set_return_value)
            .register_fn("set_return", InvocationContextGetterSetter::set_return_vec)
            .register_fn("set_return", InvocationContextGetterSetter::set_return_paged)
            .register_fn("get_return", InvocationContextGetterSetter::get_return);

        engin
            .register_type_with_name::<RhaiStoreObject>("StoreObject")
            .register_fn("new_store_object", RhaiStoreObject::new)
            .register_fn("aes_encrypt", RhaiStoreObject::aes_encrypt)
            .register_fn("aes_decrypt", RhaiStoreObject::aes_decrypt)
            .register_fn("rsa_encrypt", RhaiStoreObject::rsa_encrypt)
            .register_fn("rsa_decrypt", RhaiStoreObject::rsa_decrypt)            
            .register_fn("select", RhaiStoreObject::select)
            .register_fn(
                "select",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: i64| {
                    RhaiStoreObject::select(caller, ctx, Value::Number(Number::from(arg)))
                },
            )
            .register_fn(
                "select",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: String| {
                    RhaiStoreObject::select(caller, ctx, Value::String(arg))
                },
            )
            .register_fn("find_one", RhaiStoreObject::find_one)
            .register_fn(
                "insert",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::invoke(caller, "insert", ctx, arg)
                },
            )
            .register_fn(
                "update",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::invoke(caller, "update", ctx, arg)
                },
            )
            .register_fn(
                "upsert",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::invoke_args(caller, "upsert", ctx, vec![arg])
                },
            )
            .register_fn(
                "upsert",
                |caller: &mut RhaiStoreObject,
                 ctx: Arc<Mutex<InvocationContext>>,
                 arg: Value,
                 cond: Value| {
                    RhaiStoreObject::invoke_args(caller, "upsert", ctx, vec![arg, cond])
                },
            )
            .register_fn(
                "save_batch",
                |caller: &mut RhaiStoreObject,
                 ctx: Arc<Mutex<InvocationContext>>,
                 arg: Vec<Value>| {
                    RhaiStoreObject::invoke_args(caller, "save_batch", ctx, arg)
                },
            )            
            .register_fn(
                "update_by",
                |caller: &mut RhaiStoreObject,
                 ctx: Arc<Mutex<InvocationContext>>,
                 arg: Value,
                 cond: Value| {
                    RhaiStoreObject::invoke_args(caller, "upsert", ctx, vec![arg, cond])
                },
            )
            .register_fn(
                "delete",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::invoke(caller, "delete", ctx, arg)
                },
            )
            .register_fn(
                "delete_by",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::invoke(caller, "delete", ctx, arg)
                },
            )
            .register_fn("query", RhaiStoreObject::query)
            .register_fn(
                "query",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::query(caller, ctx, vec![arg])
                },
            )
            .register_fn("paged_query", RhaiStoreObject::paged_query)
            .register_fn(
                "paged_query",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::paged_query(caller, ctx, vec![arg])
                },
            );

        engin
            .register_type_with_name::<RhaiStoreQuery>("QueryObject")
            .register_fn("new_query", RhaiStoreQuery::new)
            .register_fn("search", RhaiStoreQuery::search)
            .register_fn("paged_search", RhaiStoreQuery::paged_search);

        RhaiStorePlugin::register(&mut engin);

        engin
            .register_type_with_name::<Value>("JSON")
            .register_fn("new_json_object", ValueGetterSetter::create_json_object)
            .register_fn("new_json_null", ValueGetterSetter::create_json_null)
            .register_fn("new_json_array", ValueGetterSetter::create_json_array)
            .register_fn("new_json_string", ValueGetterSetter::create_json_string)
            .register_fn("new_json_bool", ValueGetterSetter::create_json_bool)
            .register_fn("new_json_number", ValueGetterSetter::create_json_number)
            .register_fn("new_json_paged", ValueGetterSetter::create_paged_json)
            .register_fn(
                "new_json_paged",
                ValueGetterSetter::create_paged_json_nototal,
            )
            .register_fn("new_json_paged", ValueGetterSetter::create_paged_json_empty)
            .register_fn("new_json_paged", ValueGetterSetter::create_paged_json_array)
            .register_fn(
                "new_json_paged",
                ValueGetterSetter::create_paged_json_nototal_array,
            )
            .register_fn("new_json_object", ValueGetterSetter::create_json_from_map)
            .register_fn(
                "new_query_condition",
                ValueGetterSetter::create_query_condition,
            )
            .register_fn(
                "new_query_condition",
                ValueGetterSetter::create_query_condition_args,
            )
            .register_fn(
                "new_query_condition",
                ValueGetterSetter::create_query_condition_args_1,
            )
            .register_fn(
                "new_query_condition",
                ValueGetterSetter::create_query_condition_args_2,
            )
            .register_fn(
                "new_query_condition",
                ValueGetterSetter::create_query_condition_args_3,
            )
            .register_fn(
                "new_query_condition",
                ValueGetterSetter::create_query_condition_args_4,
            )
            .register_fn(
                "new_query_condition",
                ValueGetterSetter::create_query_condition_args_5,
            )
            .register_fn("new_ordianl_item", ValueGetterSetter::create_ordianl_item)
            .register_fn(
                "new_ordianl_item",
                ValueGetterSetter::create_ordianl_item_args,
            )
            .register_fn("new_condition_item", ValueGetterSetter::create_condition)
            .register_fn(
                "new_condition_item",
                ValueGetterSetter::create_condition_args,
            )
            .register_fn("unwrap", ValueGetterSetter::unwrap_option)
            .register_fn("unwrap", ValueGetterSetter::unwrap_value)
            .register_fn("canonicalized_query", ValueGetterSetter::canonicalized_query)
            .register_fn("canonicalized_query", ValueGetterSetter::canonicalized_query_asc)            
            .register_fn("to_rhai_object", ValueGetterSetter::to_rhai_object)                        
            .register_fn("to_rhai_object", ValueGetterSetter::to_rhai_object_option)
            .register_fn("to_array", ValueGetterSetter::to_array)
            .register_indexer_get::<Value, &str, false, Value, false>(ValueGetterSetter::get)
            .register_indexer_get::<Option<Value>, &str, false, Value, false>(
                ValueGetterSetter::get_option_value,
            )
            .register_indexer_get::<Vec<Value>, i64, false, Value, false>(
                ValueGetterSetter::get_item_i64,
            )
            .register_indexer_get::<Vec<Value>, u64, false, Value, false>(
                ValueGetterSetter::get_item_u64,
            )
            .register_indexer_get::<Vec<Value>, usize, false, Value, false>(
                ValueGetterSetter::get_item_usize,
            )
            .register_fn("to_string", ValueGetterSetter::to_debug)
            .register_fn("to_string", ValueGetterSetter::to_debug_option)
            .register_fn("to_debug", ValueGetterSetter::to_debug)
            .register_fn("to_debug", ValueGetterSetter::to_debug_page)
            .register_fn("to_debug", ValueGetterSetter::to_debug_vec)
            .register_fn("to_string", ValueGetterSetter::to_debug_page)
            .register_fn("to_string", ValueGetterSetter::to_debug_vec)
            .register_fn("is_paged", ValueGetterSetter::is_paged)
            .register_fn("is_paged", ValueGetterSetter::is_paged_option)
            .register_fn("is_paged", ValueGetterSetter::is_paged_page)
            .register_fn("is_paged", ValueGetterSetter::is_paged_vec)
            .register_fn("is_list", ValueGetterSetter::is_list)
            .register_fn("is_list", ValueGetterSetter::is_list_option)
            .register_fn("is_list", ValueGetterSetter::is_list_page)
            .register_fn("is_list", ValueGetterSetter::is_list_vec)
            .register_fn("push", ValueGetterSetter::push)
            .register_fn("select", ValueGetterSetter::select)
            .register_fn("select", ValueGetterSetter::select_option)
            .register_fn("get_records", ValueGetterSetter::get_records)
            .register_fn("set_records", ValueGetterSetter::set_records)
            .register_fn("get_page_no", ValueGetterSetter::get_page_no)
            .register_fn("set_page_no", ValueGetterSetter::set_page_no)
            .register_fn("get_total", ValueGetterSetter::get_total)
            .register_fn("set_total", ValueGetterSetter::set_total)
            .register_fn("get_page_size", ValueGetterSetter::get_page_size)
            .register_fn("set_page_size", ValueGetterSetter::set_page_size)
            .register_indexer_set::<Value, &str, false, Value, false>(ValueGetterSetter::set)
            .register_indexer_set::<Option<Value>, &str, false, Value, false>(
                ValueGetterSetter::set_option,
            )
            .register_indexer_set::<Value, &str, false, bool, false>(ValueGetterSetter::set_bool)
            .register_indexer_set::<Option<Value>, &str, false, bool, false>(
                ValueGetterSetter::set_bool_option,
            )
            .register_indexer_set::<Value, &str, false, &str, false>(ValueGetterSetter::set_string)
            .register_indexer_set::<Option<Value>, &str, false, &str, false>(
                ValueGetterSetter::set_string_option,
            )
            .register_indexer_set::<Value, &str, false, i64, false>(ValueGetterSetter::set_i64)
            .register_indexer_set::<Option<Value>, &str, false, i64, false>(
                ValueGetterSetter::set_i64_option,
            )
            .register_indexer_set::<Value, &str, false, u64, false>(ValueGetterSetter::set_u64)
            .register_indexer_set::<Option<Value>, &str, false, u64, false>(
                ValueGetterSetter::set_u64_option,
            )
            .register_indexer_set::<Value, &str, false, f64, false>(ValueGetterSetter::set_f64)
            .register_indexer_set::<Option<Value>, &str, false, f64, false>(
                ValueGetterSetter::set_f64_option,
            )
            .register_indexer_set::<Vec<Value>, i64, false, Value, false>(
                ValueGetterSetter::set_item_i64,
            )
            .register_indexer_set::<Vec<Value>, u64, false, Value, false>(
                ValueGetterSetter::set_item_u64,
            );
        Self {
            engine: Arc::new(engin),
        }
    }

    pub fn get_mut() -> &'static mut EvalEngine {
        // 使用MaybeUninit延迟初始化
        static mut LANG_EXTENSION_RHAI_MAP: MaybeUninit<EvalEngine> = MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static RHAI_ONCE: Once = Once::new();

        RHAI_ONCE.call_once(|| unsafe {
            LANG_EXTENSION_RHAI_MAP
                .as_mut_ptr()
                .write(EvalEngine::create());
        });

        unsafe { &mut (*LANG_EXTENSION_RHAI_MAP.as_mut_ptr()) }
    }
}

pub(crate) fn init_engin() {
    let _ = EvalEngine::get_mut();
}

/**
 * 如果，返回值是一个数组，则只返回该数据中第一个元素
 */
pub(crate) fn eval_script_return_one(
    script: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Option<Value>, anyhow::Error> {
    let script = script.to_owned();
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("args", args.into());
    scope.push("ctx", ctx);
    log::debug!("eval rhai script: {}", script);
    match EvalEngine::get_mut()
        .engine
        .eval_with_scope::<rhai::Dynamic>(&mut scope, &script)
    {
        Ok(t) => {
            if t.is_array() {
                let ret = t
                    .cast::<Vec<Dynamic>>()
                    .into_iter()
                    .map(|f| f.cast::<Value>())
                    .collect::<Vec<Value>>();
                Ok(Some(ret[0].clone()))
            } else {
                let ret = t.clone().try_cast::<Option<Value>>();
                if let Some(rt) = ret {
                    Ok(rt)
                } else {
                    let ret = t.try_cast::<Value>();
                    Ok(ret)
                }
            }
        }
        Err(alt) => {
            let err = anyhow!("script error: {:?}", alt);
            Err(err)
        }
    }
}

pub(crate) fn eval_script_return_vec(
    script: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Vec<Value>, anyhow::Error> {
    let script = script.to_owned();
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("args", args.into());
    scope.push("ctx", ctx);
    log::debug!("eval rhai script: {}", script);
    match EvalEngine::get_mut()
        .engine
        .eval_with_scope::<rhai::Dynamic>(&mut scope, &script)
    {
        Ok(t) => {
            if t.is_array() {
                let ret = t
                    .cast::<Vec<Dynamic>>()
                    .into_iter()
                    .map(|f| f.cast::<Value>())
                    .collect::<Vec<Value>>();
                Ok(ret)
            } else {
                match t.clone().try_cast::<Value>() {
                    Some(ret) => Ok(vec![ret]),
                    None => match t.try_cast::<Option<Value>>() {
                        Some(ret) => Ok(ret.map(|f| vec![f]).unwrap_or(vec![])),
                        None => Ok(vec![]),
                    },
                }
            }
        }
        Err(alt) => {
            let err = anyhow!("script error: {:?}", alt);
            Err(err)
        }
    }
}

pub(crate) fn eval_script_return_page(
    script: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Page<Value>, anyhow::Error> {
    let script = script.to_owned();
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("args", args.into());
    scope.push("ctx", ctx);
    log::debug!("eval rhai script: {}", script);
    match EvalEngine::get_mut()
        .engine
        .eval_with_scope::<rhai::Dynamic>(&mut scope, &script)
    {
        Ok(t) => match t.clone().try_cast::<Page<Value>>() {
            Some(tt) => Ok(tt),
            None => {
                let err = anyhow!("Could not cast to Page<Value>.");
                Err(err)
            }
        },
        Err(alt) => {
            let err = anyhow!("script error: {:?}", alt);
            Err(err)
        }
    }
}

fn load_script(filename: impl AsRef<Path>) -> Result<String, anyhow::Error> {
    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    Ok(contents)
}

pub(crate) fn eval_file_return_one(
    filename: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Option<Value>, anyhow::Error> {
    let script = load_script(filename)?;
    eval_script_return_one(&script, ctx, args)
}

pub(crate) fn eval_file_return_vec(
    filename: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Vec<Value>, anyhow::Error> {
    let script = load_script(filename)?;
    eval_script_return_vec(&script, ctx, args)
}

pub(crate) fn eval_file_return_page(
    filename: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Page<Value>, anyhow::Error> {
    let script = load_script(filename)?;
    eval_script_return_page(&script, ctx, args)
}
