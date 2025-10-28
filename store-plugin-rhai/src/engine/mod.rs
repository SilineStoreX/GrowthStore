use ::reqwest::Method;
use anyhow::anyhow;
use date::RhaiDateTime;
use rbatis::Page;
use reqwest::RhaiHttpClient;
use resolver::{
    require_create, InvocationContextGetterSetter, RhaiStoreObject, RhaiStorePlugin,
    RhaiStoreQuery, RhaiStoreServiceResolver, ValueGetterSetter,
};
use rhai::{Array, Dynamic, EvalAltResult, Expr, ImmutableString, Position, Stmt};
// use rhai::packages::Package;
// use rhai_fs::FilesystemPackage;

use std::{
    cmp::Ordering,
    fs::File,
    io::{BufReader, Read},
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex, OnceLock},
};

use chimes_store_core::service::{invoker::InvocationContext, sdk::InvokeUri};
use serde_json::{Number, Value};

use crate::engine::resolver::{log_info, log_warn};

mod common;
mod date;
mod redis;
mod reqwest;
pub mod resolver;

#[derive(Clone)]
pub struct EvalEngine {
    engine: Arc<rhai::Engine>,
}

impl EvalEngine {
    fn create() -> Self {
        let mut engin = rhai::Engine::new();
        // let fs = FilesystemPackage::new();

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

        // let arith = rhai::packages::ArithmeticPackage::new();
        // arith.register_into_engine(&mut engin);

        // fs.register_into_engine(&mut engin);
        engin.register_fn("info", log_info);
        engin.register_fn("warn", log_warn);
        engin.register_fn("required", require_create);
        engin.register_fn("rand_string", common::rand_text);
        engin.register_fn("random_string", common::rand_text);
        engin.register_fn("urlencode", common::url_encode);
        engin.register_fn("urldecode", common::url_decode);
        engin.register_fn("sha1_text", common::sha1_text);
        engin.register_fn("sha2_text", common::sha2_text);
        engin.register_fn("hmac_sha1", common::hmac_sha1_rhai);
        engin.register_fn("hmac_sha256", common::hmac_sha256_rhai);
        engin.register_fn("hmac_sha512", common::hmac_sha512_rhai);
        engin.register_fn("md5string", common::text_md5);
        engin.register_fn("data_to_hex", common::blob_to_hex);
        engin.register_fn("read_file_blob", common::read_file_blob);
        engin.register_fn("read_file_string", common::read_file_string);
        engin.register_fn("write_file_blob", common::write_file_blob);
        engin.register_fn("write_file_string", common::write_file_string);
        engin.register_fn("base32encode", common::text_base32_encode);
        engin.register_fn("base32decode", common::text_base32_decode);
        engin.register_fn("base64encode", common::text_base64_encode);
        engin.register_fn("base64decode", common::text_base64_decode);
        engin.register_fn("base64encode", common::blob_base64_encode);
        engin.register_fn("base64decode", common::blob_base64_decode);
        engin.register_fn("base64encode", common::text_base64_encode_urlsafe);
        engin.register_fn("base64decode", common::text_base64_decode_urlsafe);
        engin.register_fn("file_base64encode", common::file_base64_encode_urlsafe);
        engin.register_fn("file_base64decode", common::file_base64_decode_urlsafe);
        engin.register_fn("base64encode", common::blob_base64_encode_urlsafe);
        engin.register_fn("base64decode", common::blob_base64_decode_urlsafe);
        engin.register_fn("snowflake_id", common::rhai_snowflake_id);
        engin.register_fn("snowflake_id", common::rhai_snowflake_id_custom);
        engin.register_fn("parse_xml", common::rhai_parse_xml);
        engin.register_fn("redis_get", redis::rhai_redis_get);
        engin.register_fn("redis_set", redis::rhai_redis_set);
        engin.register_fn("redis_set", redis::rhai_redis_set_expire);
        engin.register_fn("redis_set", redis::rhai_redis_set_expire_nx);
        engin.register_fn("redis_del", redis::rhai_redis_del);
        engin.register_fn("redis_keys", redis::rhai_redis_keys);
        engin.register_fn("redis_flushall", redis::rhai_redis_flushall);
        engin.register_fn("redis_lock", redis::rhai_redis_lock);
        engin.register_fn("redis_unlock", redis::rhai_redis_unlock);
        engin.register_fn("get_namesapce_config", common::rhai_get_namesapce_config);
        engin.register_fn("get_plugin_config", common::rhai_get_plugin_config);
        engin.register_fn("uuid", common::rhai_uuid);
        engin.register_fn("queue_addtask", common::queue_addtask);
        engin.register_fn("schedule_add_oneshot", common::schedule_add_oneshot_task);
        engin.register_fn("schedule_oneshot_query", common::schedule_oneshot_query);
        engin.register_fn("http_request", RhaiHttpClient::sync_http_request);
        engin.register_fn(
            "http_request",
            |url: &str, method: &str, data: Dynamic, opt: Dynamic| {
                let data_val = serde_json::to_value(data).unwrap_or(Value::Null);
                let opt_val = serde_json::to_value(opt).map(Some).unwrap_or(None);
                match Method::from_str(method) {
                    Ok(mth) => RhaiHttpClient::sync_http_request(url, mth, data_val, opt_val),
                    Err(err) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        Dynamic::from_str(&err.to_string()).unwrap(),
                        Position::new(1, 1),
                    ))),
                }
            },
        );
        engin.register_fn("http_get", |url: &str, data: Dynamic, opt: Dynamic| {
            let data_val = serde_json::to_value(data).unwrap_or(Value::Null);
            let opt_val = serde_json::to_value(opt).map(Some).unwrap_or(None);
            RhaiHttpClient::sync_http_request(url, Method::GET, data_val, opt_val)
        });
        engin.register_fn("http_get", |url: &str, data: Dynamic| {
            let data_val = serde_json::to_value(data).unwrap_or(Value::Null);
            RhaiHttpClient::sync_http_request(url, Method::GET, data_val, None)
        });
        engin.register_fn("http_get", |url: &str, data: Value, opt: Value| {
            RhaiHttpClient::sync_http_request(url, Method::GET, data, Some(opt))
        });
        engin.register_fn("http_get", |url: &str, data: Value| {
            RhaiHttpClient::sync_http_request(url, Method::GET, data, None)
        });
        engin.register_fn("http_get", |url: &str| {
            RhaiHttpClient::sync_http_request(url, Method::GET, Value::Null, None)
        });
        engin.register_fn("http_post", |url: &str, data: Dynamic, opt: Dynamic| {
            let data_val = serde_json::to_value(data).unwrap_or(Value::Null);
            let opt_val = serde_json::to_value(opt).map(Some).unwrap_or(None);
            RhaiHttpClient::sync_http_request(url, Method::POST, data_val, opt_val)
        });
        engin.register_fn("http_post", |url: &str, data: Dynamic| {
            let data_val = serde_json::to_value(data).unwrap_or(Value::Null);
            RhaiHttpClient::sync_http_request(url, Method::POST, data_val, None)
        });
        engin.register_fn("http_post", |url: &str, data: Value, opt: Value| {
            RhaiHttpClient::sync_http_request(url, Method::POST, data, Some(opt))
        });
        engin.register_fn("http_post", |url: &str, data: Value| {
            RhaiHttpClient::sync_http_request(url, Method::POST, data, None)
        });
        engin.register_fn("http_put", |url: &str, data: Dynamic, opt: Dynamic| {
            let data_val = serde_json::to_value(data).unwrap_or(Value::Null);
            let opt_val = serde_json::to_value(opt).map(Some).unwrap_or(None);
            RhaiHttpClient::sync_http_request(url, Method::PUT, data_val, opt_val)
        });
        engin.register_fn("http_put", |url: &str, data: Dynamic| {
            let data_val = serde_json::to_value(data).unwrap_or(Value::Null);
            RhaiHttpClient::sync_http_request(url, Method::PUT, data_val, None)
        });
        engin.register_fn("http_put", |url: &str, data: Value, opt: Value| {
            RhaiHttpClient::sync_http_request(url, Method::PUT, data, Some(opt))
        });
        engin.register_fn("http_put", |url: &str, data: Value| {
            RhaiHttpClient::sync_http_request(url, Method::PUT, data, None)
        });
        engin.register_fn("http_delete", |url: &str, data: Dynamic, opt: Dynamic| {
            let data_val = serde_json::to_value(data).unwrap_or(Value::Null);
            let opt_val = serde_json::to_value(opt).map(Some).unwrap_or(None);
            RhaiHttpClient::sync_http_request(url, Method::DELETE, data_val, opt_val)
        });
        engin.register_fn("http_delete", |url: &str, data: Dynamic| {
            let data_val = serde_json::to_value(data).unwrap_or(Value::Null);
            RhaiHttpClient::sync_http_request(url, Method::DELETE, data_val, None)
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
            .register_fn("get_message", InvocationContextGetterSetter::get_message)
            .register_fn("get_status", InvocationContextGetterSetter::get_status)
            .register_fn("set_message", InvocationContextGetterSetter::set_message)
            .register_fn("set_status", InvocationContextGetterSetter::set_status)
            .register_fn("set_user", InvocationContextGetterSetter::set_username)
            .register_fn("set_user", InvocationContextGetterSetter::set_username_withid)
            .register_fn("get_username", InvocationContextGetterSetter::get_username)
            .register_fn("get_userid", InvocationContextGetterSetter::get_userid)
            .register_fn("get_authorization", InvocationContextGetterSetter::get_authorization)
            .register_fn("set_status", InvocationContextGetterSetter::set_status)
            .register_fn(
                "get_response_xml",
                InvocationContextGetterSetter::get_response_xml,
            )
            .register_fn(
                "set_response_xml",
                InvocationContextGetterSetter::set_response_xml,
            )
            .register_fn(
                "get_return_rawdata",
                InvocationContextGetterSetter::get_return_rawdata,
            )
            .register_fn(
                "set_return_rawdata",
                InvocationContextGetterSetter::set_return_rawdata,
            )
            .register_fn("get_hook_uri", InvocationContextGetterSetter::get_hook_uri)
            .register_fn("set", InvocationContextGetterSetter::set)
            .register_fn("set", InvocationContextGetterSetter::set_option)
            .register_fn("set", InvocationContextGetterSetter::set_value)
            .register_fn("set", InvocationContextGetterSetter::set_vec)
            .register_fn("set", InvocationContextGetterSetter::set_paged)
            .register_fn("set_return", InvocationContextGetterSetter::set_return)
            .register_fn(
                "set_return",
                InvocationContextGetterSetter::set_return_option,
            )
            .register_fn(
                "set_return",
                InvocationContextGetterSetter::set_return_value,
            )
            .register_fn("set_return", InvocationContextGetterSetter::set_return_vec)
            .register_fn(
                "set_return",
                InvocationContextGetterSetter::set_return_paged,
            )
            .register_fn("set_failed", InvocationContextGetterSetter::set_failed)
            .register_fn("get_return", InvocationContextGetterSetter::get_return)
            .register_fn("lock", InvocationContextGetterSetter::lock_ns)
            .register_fn("lock", InvocationContextGetterSetter::lock)
            .register_fn("unlock", InvocationContextGetterSetter::unlock)
            .register_fn(
                "release",
                InvocationContextGetterSetter::release_connections,
            );
        // release method of ctx called to commit when all operations was success and call rollbak if anyone operation was failed.
        // this method is gated to release connections when a long time script calling.

        engin
            .register_type_with_name::<RhaiStoreObject>("StoreObject")
            .register_fn("new_store_object", RhaiStoreObject::new)
            .register_fn("aes_encrypt", RhaiStoreObject::aes_encrypt)
            .register_fn("aes_decrypt", RhaiStoreObject::aes_decrypt)
            .register_fn("rsa_encrypt", RhaiStoreObject::rsa_encrypt)
            .register_fn("rsa_decrypt", RhaiStoreObject::rsa_decrypt)
            .register_fn(
                "sha256_with_rsa_sign",
                RhaiStoreObject::sha256_with_rsa_sign,
            )
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
                "invoke",
                |caller: &mut RhaiStoreObject,
                 name: &str,
                 ctx: Arc<Mutex<InvocationContext>>,
                 args: Vec<Value>| {
                    RhaiStoreObject::invoke_args(caller, name, ctx, args)
                },
            )
            .register_fn(
                "invoke",
                |caller: &mut RhaiStoreObject,
                 name: &str,
                 ctx: Arc<Mutex<InvocationContext>>,
                 arg: Value| { RhaiStoreObject::invoke(caller, name, ctx, arg) },
            )
            .register_fn(
                "insert",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::invoke(caller, "insert", ctx, arg)
                },
            )
            .register_fn(
                "insert",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreObject::invoke(caller, "insert", ctx, obj_args)
                },
            )
            .register_fn(
                "insert",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args: Vec<Value> = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    let len = vec_args.len();

                    match len.cmp(&1) {
                        Ordering::Greater => {
                            RhaiStoreObject::invoke(caller, "insert", ctx, vec_args[1].clone())
                        }
                        Ordering::Equal => {
                            RhaiStoreObject::invoke(caller, "insert", ctx, vec_args[0].clone())
                        }
                        Ordering::Less => {
                            RhaiStoreObject::invoke(caller, "insert", ctx, Value::Null)
                        }
                    }
                },
            )
            .register_fn(
                "update",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::invoke(caller, "update", ctx, arg)
                },
            )
            .register_fn(
                "update",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreObject::invoke(caller, "update", ctx, obj_args)
                },
            )
            .register_fn(
                "update",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args: Vec<Value> = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    let len = vec_args.len();
                    match len.cmp(&1) {
                        Ordering::Greater => {
                            RhaiStoreObject::invoke(caller, "update", ctx, vec_args[1].clone())
                        }
                        Ordering::Equal => {
                            RhaiStoreObject::invoke(caller, "update", ctx, vec_args[0].clone())
                        }
                        Ordering::Less => {
                            RhaiStoreObject::invoke(caller, "update", ctx, Value::Null)
                        }
                    }
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
                "upsert",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreObject::invoke_args(caller, "upsert", ctx, vec![obj_args])
                },
            )
            .register_fn(
                "upsert",
                |caller: &mut RhaiStoreObject,
                 ctx: Arc<Mutex<InvocationContext>>,
                 arg: Dynamic,
                 cond: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    let obj_cond = serde_json::to_value(cond).unwrap_or(Value::Null);
                    RhaiStoreObject::invoke_args(caller, "upsert", ctx, vec![obj_args, obj_cond])
                },
            )
            .register_fn(
                "upsert",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args: Vec<Value> = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    RhaiStoreObject::invoke_args(caller, "upsert", ctx, vec_args)
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
                "save_batch",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args: Vec<Value> = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    RhaiStoreObject::invoke_args(caller, "save_batch", ctx, vec_args)
                },
            )
            .register_fn(
                "update_by",
                |caller: &mut RhaiStoreObject,
                 ctx: Arc<Mutex<InvocationContext>>,
                 arg: Value,
                 cond: Value| {
                    RhaiStoreObject::invoke_args(caller, "update_by", ctx, vec![arg, cond])
                },
            )
            .register_fn(
                "update_by",
                |caller: &mut RhaiStoreObject,
                 ctx: Arc<Mutex<InvocationContext>>,
                 args: Vec<Value>| {
                    RhaiStoreObject::invoke_args(caller, "update_by", ctx, args)
                },
            )
            .register_fn(
                "update_by",
                |caller: &mut RhaiStoreObject,
                 ctx: Arc<Mutex<InvocationContext>>,
                 arg: Dynamic,
                 cond: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    let obj_cond = serde_json::to_value(cond).unwrap_or(Value::Null);
                    RhaiStoreObject::invoke_args(caller, "update_by", ctx, vec![obj_args, obj_cond])
                },
            )
            .register_fn(
                "update_by",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args: Vec<Value> = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    RhaiStoreObject::invoke_args(caller, "update_by", ctx, vec_args)
                },
            )
            .register_fn(
                "delete",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::invoke(caller, "delete", ctx, arg)
                },
            )
            .register_fn(
                "delete",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreObject::invoke(caller, "delete", ctx, obj_args)
                },
            )
            .register_fn(
                "delete",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args: Vec<Value> = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    let len = vec_args.len();
                    match len.cmp(&1) {
                        Ordering::Greater => {
                            RhaiStoreObject::invoke(caller, "delete", ctx, vec_args[1].clone())
                        }
                        Ordering::Equal => {
                            RhaiStoreObject::invoke(caller, "delete", ctx, vec_args[0].clone())
                        }
                        Ordering::Less => {
                            RhaiStoreObject::invoke(caller, "delete", ctx, Value::Null)
                        }
                    }
                },
            )
            .register_fn(
                "delete_by",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    match arg {
                        Value::Array(stargs) => {
                            RhaiStoreObject::invoke_args(caller, "delete_by", ctx, stargs)
                        }
                        _ => RhaiStoreObject::invoke_args(
                            caller,
                            "delete_by",
                            ctx,
                            vec![Value::Null, arg],
                        ),
                    }
                },
            )
            .register_fn(
                "delete_by",
                |caller: &mut RhaiStoreObject,
                 ctx: Arc<Mutex<InvocationContext>>,
                 args: Vec<Value>| {
                    RhaiStoreObject::invoke_args(caller, "delete_by", ctx, args)
                },
            )
            .register_fn(
                "delete_by",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    match obj_args {
                        Value::Array(stargs) => {
                            RhaiStoreObject::invoke_args(caller, "delete_by", ctx, stargs)
                        }
                        _ => RhaiStoreObject::invoke_args(
                            caller,
                            "delete_by",
                            ctx,
                            vec![Value::Null, obj_args],
                        ),
                    }
                },
            )
            .register_fn(
                "delete_by",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args: Vec<Value> = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    let len = vec_args.len();
                    if len == 1 {
                        RhaiStoreObject::invoke_args(
                            caller,
                            "delete_by",
                            ctx,
                            vec![Value::Null, vec_args[0].clone()],
                        )
                    } else {
                        RhaiStoreObject::invoke_args(caller, "delete_by", ctx, vec_args)
                    }
                },
            )
            .register_fn("find_one", RhaiStoreObject::find_one)
            .register_fn(
                "find_one",
                |caller: &mut RhaiStoreObject,
                 ctx: Arc<Mutex<InvocationContext>>,
                 arg: Vec<Value>| {
                    RhaiStoreObject::find_one(caller, ctx, arg[0].clone())
                },
            )
            .register_fn(
                "find_one",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreObject::find_one(caller, ctx, obj_args)
                },
            )
            .register_fn(
                "find_one",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args: Vec<Value> = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    let len = vec_args.len();
                    match len.cmp(&1) {
                        std::cmp::Ordering::Less => {
                            RhaiStoreObject::find_one(caller, ctx, Value::Null)
                        }
                        std::cmp::Ordering::Equal => {
                            RhaiStoreObject::find_one(caller, ctx, vec_args[0].clone())
                        }
                        std::cmp::Ordering::Greater => {
                            RhaiStoreObject::find_one(caller, ctx, vec_args[1].clone())
                        }
                    }
                },
            )
            .register_fn("query", RhaiStoreObject::query)
            .register_fn(
                "query",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::query(caller, ctx, vec![arg])
                },
            )
            .register_fn(
                "query",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreObject::query(caller, ctx, vec![obj_args])
                },
            )
            .register_fn(
                "query",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    RhaiStoreObject::query(caller, ctx, vec_args)
                },
            )
            .register_fn("paged_query", RhaiStoreObject::paged_query)
            .register_fn(
                "paged_query",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreObject::paged_query(caller, ctx, vec![arg])
                },
            )
            .register_fn(
                "paged_query",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreObject::paged_query(caller, ctx, vec![obj_args])
                },
            )
            .register_fn(
                "paged_query",
                |caller: &mut RhaiStoreObject, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    RhaiStoreObject::paged_query(caller, ctx, vec_args)
                },
            );

        engin
            .register_type_with_name::<RhaiStoreQuery>("QueryObject")
            .register_fn("new_query", RhaiStoreQuery::new)
            .register_fn("find_one", RhaiStoreQuery::find_one)
            .register_fn(
                "find_one",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreQuery::find_one(caller, ctx, vec![arg])
                },
            )
            .register_fn(
                "find_one",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreQuery::find_one(caller, ctx, vec![obj_args])
                },
            )
            .register_fn(
                "find_one",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    RhaiStoreQuery::find_one(caller, ctx, vec_args)
                },
            )
            .register_fn("direct_query", RhaiStoreQuery::direct_query)
            .register_fn(
                "direct_query",
                |caller: &mut RhaiStoreQuery,
                 ctx: Arc<Mutex<InvocationContext>>,
                 sql: ImmutableString,
                 arg: Value| {
                    RhaiStoreQuery::direct_query(caller, ctx, sql.to_string(), vec![arg])
                },
            )
            .register_fn(
                "direct_query",
                |caller: &mut RhaiStoreQuery,
                 ctx: Arc<Mutex<InvocationContext>>,
                 sql: ImmutableString,
                 arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreQuery::direct_query(caller, ctx, sql.to_string(), vec![obj_args])
                },
            )
            .register_fn(
                "direct_query",
                |caller: &mut RhaiStoreQuery,
                 ctx: Arc<Mutex<InvocationContext>>,
                 sql: ImmutableString,
                 args: Array| {
                    let vec_args = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    RhaiStoreQuery::direct_query(caller, ctx, sql.to_string(), vec_args)
                },
            )
            .register_fn("direct_query_v2", RhaiStoreQuery::direct_query_v2)
            .register_fn(
                "direct_query_v2",
                |caller: &mut RhaiStoreQuery,
                 ctx: Arc<Mutex<InvocationContext>>,
                 query: Value,
                 arg: Value| {
                    RhaiStoreQuery::direct_query_v2(caller, ctx, query, vec![arg])
                },
            )
            .register_fn(
                "direct_query_v2",
                |caller: &mut RhaiStoreQuery,
                 ctx: Arc<Mutex<InvocationContext>>,
                 query: Dynamic,
                 arg: Dynamic| {
                    let json_query = serde_json::to_value(query).unwrap_or(Value::Null);
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreQuery::direct_query_v2(caller, ctx, json_query, vec![obj_args])
                },
            )
            .register_fn(
                "direct_query_v2",
                |caller: &mut RhaiStoreQuery,
                 ctx: Arc<Mutex<InvocationContext>>,
                 query: Dynamic,
                 args: Array| {
                    let json_query = serde_json::to_value(query).unwrap_or(Value::Null);
                    let vec_args = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    RhaiStoreQuery::direct_query_v2(caller, ctx, json_query, vec_args)
                },
            )
            .register_fn("execute", RhaiStoreQuery::execute)
            .register_fn(
                "execute",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreQuery::execute(caller, ctx, vec![arg])
                },
            )
            .register_fn(
                "execute",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreQuery::execute(caller, ctx, vec![obj_args])
                },
            )
            .register_fn(
                "execute",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    RhaiStoreQuery::execute(caller, ctx, vec_args)
                },
            )
            .register_fn("search", RhaiStoreQuery::search)
            .register_fn(
                "search",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreQuery::search(caller, ctx, vec![arg])
                },
            )
            .register_fn(
                "search",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreQuery::search(caller, ctx, vec![obj_args])
                },
            )
            .register_fn(
                "search",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    RhaiStoreQuery::search(caller, ctx, vec_args)
                },
            )
            .register_fn("paged_search", RhaiStoreQuery::paged_search)
            .register_fn(
                "paged_search",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, arg: Value| {
                    RhaiStoreQuery::paged_search(caller, ctx, vec![arg])
                },
            )
            .register_fn(
                "paged_search",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, arg: Dynamic| {
                    let obj_args = serde_json::to_value(arg).unwrap_or(Value::Null);
                    RhaiStoreQuery::paged_search(caller, ctx, vec![obj_args])
                },
            )
            .register_fn(
                "paged_search",
                |caller: &mut RhaiStoreQuery, ctx: Arc<Mutex<InvocationContext>>, args: Array| {
                    let vec_args = args
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
                        .collect();
                    RhaiStoreQuery::paged_search(caller, ctx, vec_args)
                },
            );

        engin
            .register_type_with_name::<RhaiDateTime>("DateTime")
            .register_fn("from_1970", RhaiDateTime::from_1970)
            .register_fn("now", RhaiDateTime::now)
            .register_fn("now_utc", RhaiDateTime::now_utc)
            .register_fn("datetime", RhaiDateTime::now)
            .register_fn("datetime_utc", RhaiDateTime::now_utc)
            .register_fn("to_string", RhaiDateTime::to_string)
            .register_fn("to_global", RhaiDateTime::to_global)
            .register_fn("to_locale", RhaiDateTime::to_locale)
            .register_fn("to_locale", RhaiDateTime::to_locale_fmt)
            .register_fn("to_rfc2822", RhaiDateTime::to_rfc2822)
            .register_fn("format", RhaiDateTime::format)
            .register_fn("parse_date", RhaiDateTime::parse_date)
            .register_fn("parse_date", RhaiDateTime::parse_date_default)
            .register_fn("from_timestamp_micros", RhaiDateTime::from_timestamp_micros)
            .register_fn("from_timestamp_second", RhaiDateTime::from_timestamp_second)
            .register_fn("from_timestamp_millis", RhaiDateTime::from_timestamp_millis)
            .register_fn("set_offset", RhaiDateTime::set_offset_with)
            .register_fn("with_offset", RhaiDateTime::with_offset)
            .register_fn("year", RhaiDateTime::year)
            .register_fn("month", RhaiDateTime::month)
            .register_fn("day", RhaiDateTime::day)
            .register_fn("hour", RhaiDateTime::hour)
            .register_fn("minute", RhaiDateTime::minute)
            .register_fn("second", RhaiDateTime::second)
            .register_fn("timestamp_second", RhaiDateTime::timestamp_second)
            .register_fn("timestamp_micro", RhaiDateTime::timestamp_micro)
            .register_fn("timestamp_millis", RhaiDateTime::timestamp_millis);

        RhaiStorePlugin::register(&mut engin);

        engin
            .register_type_with_name::<Value>("JSON")
            .register_fn("parse_json", ValueGetterSetter::parse_json_object)
            .register_fn("new_json_object", ValueGetterSetter::create_json_object)
            .register_fn("new_json_null", ValueGetterSetter::create_json_null)
            .register_fn("new_json_array", ValueGetterSetter::create_json_array)
            .register_fn("new_json_array", ValueGetterSetter::create_json_array)
            .register_fn("new_json_array", ValueGetterSetter::create_json_array_1)
            .register_fn("new_json_array", ValueGetterSetter::create_json_array_2)
            .register_fn("new_json_array", ValueGetterSetter::create_json_array_3)
            .register_fn("new_json_string", ValueGetterSetter::create_json_string)
            .register_fn("new_json_bool", ValueGetterSetter::create_json_bool)
            .register_fn("new_json_number", ValueGetterSetter::create_json_number)
            .register_fn("new_json_paged", ValueGetterSetter::create_paged_json)
            .register_fn("new_array_from", ValueGetterSetter::create_json_from_array)
            .register_fn(
                "new_json_paged",
                ValueGetterSetter::create_paged_json_nototal,
            )
            .register_fn("new_json_paged", ValueGetterSetter::create_paged_json_empty)
            .register_fn("new_json_paged", ValueGetterSetter::create_paged_json_array)
            .register_fn("push", ValueGetterSetter::push_item)
            .register_fn("remove", ValueGetterSetter::remove_item_i64)
            .register_fn("remove", ValueGetterSetter::remove_item_u64)
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
            .register_fn(
                "canonicalized_query",
                ValueGetterSetter::canonicalized_query,
            )
            .register_fn(
                "canonicalized_query",
                ValueGetterSetter::canonicalized_query_asc,
            )
            .register_fn("to_rhai_object", ValueGetterSetter::to_rhai_object)
            .register_fn("to_rhai_object", ValueGetterSetter::to_rhai_object_option)
            .register_fn("to_rhai_array", ValueGetterSetter::to_rhai_array_value)
            .register_fn("to_rhai_array", ValueGetterSetter::to_rhai_array_vec)
            .register_fn("to_rhai_array", ValueGetterSetter::to_rhai_array_option)
            .register_fn("to_array", ValueGetterSetter::to_array)
            .register_fn("to_array", ValueGetterSetter::to_array_option)
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
            .register_fn("is_empty", ValueGetterSetter::is_empty_vec)
            .register_fn("is_empty", ValueGetterSetter::is_empty_option)
            .register_fn("is_empty", ValueGetterSetter::is_empty_value)
            .register_fn("len", ValueGetterSetter::get_len_vec)
            .register_fn("len", ValueGetterSetter::get_len_option)
            .register_fn("len", ValueGetterSetter::get_len_value)
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

    // pub fn get_mut_() -> &'static mut EvalEngine {
    //     // 使用MaybeUninit延迟初始化
    //     static mut LANG_EXTENSION_RHAI_MAP: MaybeUninit<EvalEngine> = MaybeUninit::uninit();
    //     // Once带锁保证只进行一次初始化
    //     static RHAI_ONCE: Once = Once::new();

    //     RHAI_ONCE.call_once(|| unsafe {
    //         LANG_EXTENSION_RHAI_MAP
    //             .as_mut_ptr()
    //             .write(EvalEngine::create());
    //     });

    //     unsafe { &mut (*LANG_EXTENSION_RHAI_MAP.as_mut_ptr()) }
    // }

    pub fn get() -> &'static Arc<EvalEngine> {
        static LANG_EXTENSION_RHAI_MAP_: OnceLock<Arc<EvalEngine>> = OnceLock::new();
        LANG_EXTENSION_RHAI_MAP_.get_or_init(|| Arc::new(EvalEngine::create()))
    }
}

pub(crate) fn init_engin() {
    // let _ = EvalEngine::get_mut();
    let _ = EvalEngine::get();
}

fn walk_statements(statements: &[Stmt], nss: &mut Vec<String>) {
    for stmt in statements {
        match stmt {
            Stmt::Var(v, _, _) => match &v.1 {
                Expr::FnCall(fx, _) => {
                    if fx.name == "required" {
                        nss.push(fx.name.to_string());
                    }
                }
                Expr::DynamicConstant(dt, _) => {
                    let mdt = dt.clone();
                    let dt_type = dt.to_string();
                    if dt_type.ends_with("::RhaiStoreObject") {
                        if let Some(dtsto) = mdt.try_cast::<RhaiStoreObject>() {
                            if let Ok(iuri) = InvokeUri::parse(&dtsto.get_uri()) {
                                nss.push(iuri.namespace);
                            }
                        }
                    } else if dt_type.ends_with("::RhaiStoreQuery") {
                        if let Some(dtsto) = mdt.try_cast::<RhaiStoreQuery>() {
                            if let Ok(iuri) = InvokeUri::parse(&dtsto.get_uri()) {
                                nss.push(iuri.namespace);
                            }
                        }
                    } else if dt_type.ends_with("::RhaiStorePlugin") {
                        if let Some(dtsto) = mdt.try_cast::<RhaiStorePlugin>() {
                            if let Ok(iuri) = InvokeUri::parse(&dtsto.get_uri()) {
                                nss.push(iuri.namespace);
                            }
                        }
                    }
                }
                _ => {}
            },
            Stmt::Block(b) => {
                walk_statements(b.statements(), nss);
            }
            Stmt::If(fc, _) => {
                walk_statements(fc.body.statements(), nss);
                walk_statements(fc.branch.statements(), nss);
            }
            Stmt::For(fc, _) => {
                walk_statements(fc.2.body.statements(), nss);
                walk_statements(fc.2.branch.statements(), nss);
            }
            Stmt::While(wc, _) => {
                walk_statements(wc.body.statements(), nss);
                walk_statements(wc.branch.statements(), nss);
            }
            Stmt::Do(dc, _, _) => {
                walk_statements(dc.body.statements(), nss);
                walk_statements(dc.branch.statements(), nss);
            }
            Stmt::TryCatch(tc, _) => {
                walk_statements(tc.body.statements(), nss);
                walk_statements(tc.branch.statements(), nss);
            }
            _ => {}
        }
    }
}

pub(crate) fn eval_script_namespaces(script: &str) -> Result<Vec<String>, anyhow::Error> {
    let script = script.to_owned();
    let args: Vec<Value> = Vec::new();
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("args", args.into());
    scope.push("ctx", Arc::new(Mutex::new(InvocationContext::new())));
    match EvalEngine::get().engine.compile_with_scope(&scope, &script) {
        Ok(ast) => {
            let mut nss = vec![];
            walk_statements(ast.statements(), &mut nss);
            nss.sort();
            nss.dedup();
            Ok(nss)
        }
        Err(err) => {
            log::warn!("error to compile the script {err}");
            Err(anyhow!("error to comiple the script {err}"))
        }
    }
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
    scope.push("ctx", ctx.clone());

    match EvalEngine::get()
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
                match t.clone().try_cast::<Option<Value>>() {
                    Some(rt) => Ok(rt),
                    None => match t.clone().try_cast::<Vec<Value>>() {
                        Some(mt) => {
                            if mt.is_empty() {
                                Ok(None)
                            } else {
                                Ok(Some(mt[0].clone()))
                            }
                        }
                        None => match t.try_cast::<Value>() {
                            Some(st) => Ok(Some(st)),
                            None => Ok(None),
                        },
                    },
                }
            }
        }
        Err(alt) => {
            ctx.lock().unwrap().set_failed();
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
    scope.push("ctx", ctx.clone());

    match EvalEngine::get()
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
                match t.clone().try_cast::<Vec<Value>>() {
                    Some(ret) => Ok(ret),
                    None => match t.clone().try_cast::<Option<Value>>() {
                        Some(ret) => Ok(ret.map(|f| vec![f]).unwrap_or(vec![])),
                        None => match t.try_cast::<Value>() {
                            Some(ret) => Ok(vec![ret]),
                            None => Ok(vec![]),
                        },
                    },
                }
            }
        }
        Err(alt) => {
            ctx.lock().unwrap().set_failed();
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
    scope.push("ctx", ctx.clone());
    log::debug!("eval rhai script: {script}");
    match EvalEngine::get()
        .engine
        .eval_with_scope::<rhai::Dynamic>(&mut scope, &script)
    {
        Ok(t) => match t.clone().try_cast::<Page<Value>>() {
            Some(tt) => Ok(tt),
            None => {
                ctx.lock().unwrap().set_failed();
                let err = anyhow!("Could not cast to Page<Value>.");
                Err(err)
            }
        },
        Err(alt) => {
            ctx.lock().unwrap().set_failed();
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
