use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::anyhow;
use chimes_store_core::{
    service::{invoker::InvocationContext, script::ExtensionRegistry, starter::MxStoreService},
    utils::{create_rbatis_v2, redis::to_redis_client},
};
use chimes_store_dbs::dbs::invoker::ACQUIRE_TIMEOUT;
use chimes_store_utils::{crypto::{hex_encode_string, hmac_sha1, hmac_sha256, hmac_sha512, hmac_sm3}, template::{canonicalized_value, json_path_get, template_eval}};
use salvo::{oapi::endpoint, writing::Json, Request};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::utils::ManageApiResult;

#[endpoint]
pub async fn tool_jsonpath_test(req: &mut Request) -> Json<ManageApiResult<Value>> {
    let body = match req.parse_body::<Value>().await {
        Ok(t) => t,
        Err(err) => {
            return Json(ManageApiResult::<Value>::error(
                500,
                &format!("Could not Parse body to JSON format {err:?}"),
            ));
        }
    };

    if let Some(mp) = body.as_object() {
        let jsonpath = match mp.get("jsonpath") {
            Some(jp) => jp.as_str().map(|f| f.to_owned()).unwrap_or_default(),
            None => String::new(),
        };
        if jsonpath.is_empty() {
            return Json(ManageApiResult::<Value>::error(
                500,
                "Jsonpath was not provided",
            ));
        }

        let val = match mp.get("inputs") {
            Some(sp) => sp.to_owned(),
            None => Value::Null,
        };

        match json_path_get(&val, &jsonpath) {
            Some(mt) => Json(ManageApiResult::ok(mt)),
            None => Json(ManageApiResult::<Value>::error(
                404,
                "jsonpath was not found",
            )),
        }
    } else {
        Json(ManageApiResult::<Value>::error(
            500,
            "Body was not validate",
        ))
    }
}

#[endpoint]
pub async fn tool_tera_test(req: &mut Request) -> Json<ManageApiResult<String>> {
    let body = match req.parse_body::<Value>().await {
        Ok(t) => t,
        Err(err) => {
            return Json(ManageApiResult::<String>::error(
                500,
                &format!("Could not Parse body to JSON format {err:?}"),
            ));
        }
    };

    if let Some(mp) = body.as_object() {
        let template = match mp.get("template") {
            Some(jp) => jp.as_str().map(|f| f.to_owned()).unwrap_or_default(),
            None => String::new(),
        };
        if template.is_empty() {
            return Json(ManageApiResult::<String>::error(
                500,
                "template was not provided",
            ));
        }

        let val = match mp.get("inputs") {
            Some(sp) => sp.to_owned(),
            None => Value::Null,
        };

        let args = match val {
            Value::Array(ts) => ts,
            _ => vec![val],
        };

        match template_eval(&template, json!({"args": args})) {
            Ok(mt) => Json(ManageApiResult::ok(mt)),
            Err(err) => Json(ManageApiResult::<String>::error(500, &format!("{err}"))),
        }
    } else {
        Json(ManageApiResult::<String>::error(
            500,
            "Body was not validate",
        ))
    }
}

#[endpoint]
pub async fn tool_rhai_test(req: &mut Request) -> Json<ManageApiResult<Value>> {
    let body = match req.parse_body::<Value>().await {
        Ok(t) => t,
        Err(err) => {
            return Json(ManageApiResult::<Value>::error(
                500,
                &format!("Could not Parse body to JSON format {err:?}"),
            ));
        }
    };

    if let Some(mp) = body.as_object() {
        let script = match mp.get("script") {
            Some(jp) => jp.as_str().map(|f| f.to_owned()).unwrap_or_default(),
            None => String::new(),
        };

        let return_type = match mp.get("return_type") {
            Some(jp) => jp.as_str().map(|f| f.to_owned()).unwrap_or_default(),
            None => "Single".to_owned(),
        };

        if script.is_empty() {
            return Json(ManageApiResult::<Value>::error(
                500,
                "script was not provided",
            ));
        }

        let val = match mp.get("inputs") {
            Some(sp) => sp.to_owned(),
            None => Value::Null,
        };

        let args = match val {
            Value::Array(ts) => ts,
            _ => vec![val],
        };

        if let Some(rhai) = ExtensionRegistry::get_extension("rhai") {
            let ctx = Arc::new(Mutex::new(InvocationContext::new()));
            let rhai_eval_result = match return_type.as_str() {
                "List" => match rhai.fn_return_vec_script {
                    Some(eval_func) => match eval_func(&script, ctx, &args) {
                        Ok(tv) => Ok(Value::Array(tv)),
                        Err(err) => Err(err),
                    },
                    None => Err(anyhow!("No fn_return_vec_script defined")),
                },
                "Page" => match rhai.fn_return_page_script {
                    Some(eval_func) => match eval_func(&script, ctx, &args) {
                        Ok(tv) => {
                            let page_val = serde_json::to_value(&tv).unwrap_or(Value::Null);
                            Ok(page_val)
                        }
                        Err(err) => Err(err),
                    },
                    None => Err(anyhow!("No fn_return_page_script defined")),
                },
                _ => match rhai.fn_return_option_script {
                    Some(eval_func) => match eval_func(&script, ctx, &args) {
                        Ok(tv) => {
                            if let Some(sv) = tv {
                                Ok(sv)
                            } else {
                                Ok(Value::Null)
                            }
                        }
                        Err(err) => Err(err),
                    },
                    None => Err(anyhow!("No fn_return_option_script defined")),
                },
            };

            match rhai_eval_result {
                Ok(ts) => Json(ManageApiResult::ok(ts)),
                Err(err) => Json(ManageApiResult::<Value>::error(
                    500,
                    &format!("rhai eval function with error {err}"),
                )),
            }
        } else {
            Json(ManageApiResult::<Value>::error(
                500,
                "rhai lang extends was not register",
            ))
        }
    } else {
        Json(ManageApiResult::<Value>::error(500, "rhai was not found"))
    }
}

#[endpoint]
pub async fn tool_common_test(req: &mut Request) -> Json<ManageApiResult<Value>> {
    let body = match req.parse_body::<Value>().await {
        Ok(t) => t,
        Err(err) => {
            return Json(ManageApiResult::<Value>::error(
                500,
                &format!("Could not Parse body to JSON format {err:?}"),
            ));
        }
    };

    if let Some(mp) = body.as_object() {
        let script = match mp.get("script") {
            Some(jp) => jp.as_str().map(|f| f.to_owned()).unwrap_or_default(),
            None => String::new(),
        };

        let cmd = match mp.get("command") {
            Some(jp) => jp.as_str().map(|f| f.to_owned()).unwrap_or_default(),
            None => "rbatis".to_owned(),
        };

        if script.is_empty() {
            return Json(ManageApiResult::<Value>::error(
                500,
                "script was not provided",
            ));
        }

        let val = match mp.get("inputs") {
            Some(sp) => match sp.clone() {
                Value::String(t) => {
                    if t.trim().is_empty() {
                        Value::Null
                    } else {
                        Value::String(t)
                    }
                }
                _ => sp.to_owned(),
            },
            None => Value::Null,
        };

        log::info!("Test {cmd} for {val}.");
        match cmd.as_str() {
            "rbatis" => match create_rbatis_v2(&script).await {
                Ok(rb) => {
                    if let Value::String(sql) = val {
                        match rb
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await
                        {
                            Ok(con) => {
                                match con.begin().await {
                                    Ok(tx_) => {
                                        let mut ictx = InvocationContext::new();
                                        ictx.set_tx_executor_sync(
                                            "com.test.enjoylost",
                                            Arc::new(tx_),
                                        );
                                        let rx_ = ictx
                                            .get_tx_executor_sync("com.test.enjoylost")
                                            .unwrap();
                                        // drop(ictx);

                                        match rx_.exec(&sql, vec![]).await {
                                            Ok(rt) => {
                                                log::warn!("Result: {rt:?}");
                                                Json(ManageApiResult::ok(json!("SUCCESS")))
                                            },
                                            Err(err) => Json(ManageApiResult::<Value>::error(
                                                500,
                                                &format!("{cmd} {script} was not be tested by error {err}."),
                                            )),
                                        }
                                    }
                                    Err(err) => Json(ManageApiResult::<Value>::error(
                                        500,
                                        &format!(
                                            "{cmd} {script} was not be tested by error {err}."
                                        ),
                                    )),
                                }
                            }
                            Err(err) => Json(ManageApiResult::<Value>::error(
                                500,
                                &format!("{cmd} {script} was not be tested by error {err}."),
                            )),
                        }
                    } else {
                        if let Ok(pool) = rb.get_pool() {
                            let state = pool.state().await;
                            log::error!("pool state: {state:?}");
                        }
                        match rb
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await
                        {
                            Ok(_tx) => {
                                if let Ok(pool) = rb.get_pool() {
                                    let state = pool.state().await;
                                    log::error!("after state: {state:?}");
                                }
                                Json(ManageApiResult::ok(json!("SUCCESS")))
                            }
                            Err(err) => Json(ManageApiResult::<Value>::error(
                                500,
                                &format!("{cmd} {script} was not be connected by error {err}."),
                            )),
                        }
                    }
                }
                Err(err) => {
                    return Json(ManageApiResult::<Value>::error(
                        500,
                        &format!("{cmd} {script} was not be connected by error {err}."),
                    ))
                }
            },
            "rbatis-conn" => match create_rbatis_v2(&script).await {
                Ok(rb) => {
                    if let Value::String(sql) = val {
                        match rb
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await
                        {
                            Ok(con) => {
                                let mut ictx = InvocationContext::new();
                                // ictx.set_tx_executor_sync("com.test.enjoylost", Arc::new(tx_));
                                ictx.set_rbatis_connection("com.test.enjoylost", Arc::new(con));
                                let rx_ = ictx.get_rbatis_connection("com.test.enjoylost").unwrap();
                                // drop(ictx);
                                match rx_.query(&sql, vec![]).await {
                                    Ok(rt) => {
                                        log::info!("Result: {rt:?}");
                                        Json(ManageApiResult::ok(json!("SUCCESS")))
                                    }
                                    Err(err) => Json(ManageApiResult::<Value>::error(
                                        500,
                                        &format!(
                                            "{cmd} {script} was not be tested by error {err}."
                                        ),
                                    )),
                                }
                            }
                            Err(err) => Json(ManageApiResult::<Value>::error(
                                500,
                                &format!("{cmd} {script} was not be tested by error {err}."),
                            )),
                        }
                    } else {
                        if let Ok(pool) = rb.get_pool() {
                            let state = pool.state().await;
                            log::error!("pool state: {state:?}");
                        }
                        match rb
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await
                        {
                            Ok(_tx) => {
                                if let Ok(pool) = rb.get_pool() {
                                    let state = pool.state().await;
                                    log::error!("after state: {state:?}");
                                }
                                Json(ManageApiResult::ok(json!("SUCCESS")))
                            }
                            Err(err) => Json(ManageApiResult::<Value>::error(
                                500,
                                &format!("{cmd} {script} was not be connected by error {err}."),
                            )),
                        }
                    }
                }
                Err(err) => {
                    return Json(ManageApiResult::<Value>::error(
                        500,
                        &format!("{cmd} {script} was not be connected by error {err}."),
                    ))
                }
            },
            "redis" => match to_redis_client(&script) {
                Ok(tcn) => match tcn.get_redis_connection() {
                    Ok(mut cnn) => match cnn.raw_keys("*") {
                        Ok(_) => Json(ManageApiResult::ok(json!("SUCCESS"))),
                        Err(err) => Json(ManageApiResult::<Value>::error(
                            500,
                            &format!("{cmd} {script} was not be tested by error {err}."),
                        )),
                    },
                    Err(err) => Json(ManageApiResult::<Value>::error(
                        500,
                        &format!("{cmd} {script} was not be tested by error {err}."),
                    )),
                },
                Err(err) => Json(ManageApiResult::<Value>::error(
                    500,
                    &format!("{cmd} {script} was not be connected by error {err}."),
                )),
            },
            _ => {
                if let Some(_plc) = MxStoreService::get_plugin_service(&cmd) {
                    return Json(ManageApiResult::<Value>::error(
                        500,
                        &format!(
                            "{cmd} can be found as plugin-service. but now we don't support it."
                        ),
                    ));
                } else {
                    Json(ManageApiResult::<Value>::error(
                        500,
                        &format!("{cmd} was not provided be testable."),
                    ))
                }
            }
        }
    } else {
        Json(ManageApiResult::<Value>::error(500, "rhai was not found"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct HmacHashRequest {
    pub text: Option<String>,
    pub value: Option<Value>,
    pub secret: Option<String>,
    pub algorithm: Option<String>, // sha256, sha512, sha1, sm3
}

#[endpoint]
pub async fn tool_hmac_sha(req: &mut Request) -> Json<ManageApiResult<Value>> {
    let body = match req.parse_body::<HmacHashRequest>().await {
        Ok(t) => t,
        Err(err) => {
            return Json(ManageApiResult::<Value>::error(
                500,
                &format!("Could not Parse body to JSON format {err:?}"),
            ));
        }
    };

    let algorithm = body.algorithm.unwrap_or("sha256".to_string());
    let text = if body.text.is_some() {
        body.text.unwrap_or(String::new())
    } else {
        let val = body.value.map(|v| canonicalized_value(&v, &None, false).unwrap_or(Value::Null)).unwrap_or(Value::Null);
        match val {
            Value::Null => String::new(),
            Value::String(t) => t,
            _ => serde_json::to_string(&val).unwrap_or(String::new())
        }
    };

    let Some(key) = body.secret else {
        return Json(ManageApiResult::<Value>::error(
                400,
                &format!("Secret for Hmac hash is not provided."),
            ));
    };

    let hash = match algorithm.as_str() {
        "sha1" => {
            hmac_sha1(&key, &text)
        },
        "sm3" => {
            hmac_sm3(&key, &text)
        },
        "sha512" => {
            hmac_sha512(&key, &text)
        }
        _ => {
            hmac_sha256(&key, &text)
        }
    };

    let ts = Value::String(hex_encode_string(&hash));
    Json(ManageApiResult::ok(ts))
}



#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct EncryptDecryptRequest {
    pub process_text: Option<String>,
    pub namespace: Option<String>,
    pub algorithm: Option<String>, // sha256, sha512, sha1, sm3
}

#[endpoint]
pub async fn tool_encrypt_decrypt(req: &mut Request) -> Json<ManageApiResult<Value>> {
    let body = match req.parse_body::<EncryptDecryptRequest>().await {
        Ok(t) => t,
        Err(err) => {
            return Json(ManageApiResult::<Value>::error(
                500,
                &format!("Could not Parse body to JSON format {err:?}"),
            ));
        }
    };

    let algorithm = body.algorithm.unwrap_or("sha256".to_string());
    let text = body.process_text.unwrap_or_default();
    let ns = body.namespace.unwrap_or_default();

    if let Some(mx) = MxStoreService::get(&ns) {
        let process = match algorithm.as_str() {
            "rsa_encrypt" => {
                mx.rsa_encrypt_text(&text)
            },
            "rsa_decrypt" => {
                mx.rsa_decrypt_text(&text)
            },
            "aes_encrypt" => {
                mx.aes_encode_text(&text)
            },
            "aes_decrypt" => {
                mx.aes_decode_text(&text)
            },
            _ => {
                text
            }
        };

        let ts = Value::String(process);
        Json(ManageApiResult::ok(ts))
    } else {
        Json(ManageApiResult::<Value>::error(
            500,
            &format!("Unknown algorithm"),
        ))
    }
}


#[endpoint]
pub async fn tool_plugin_test(_req: &mut Request) -> Json<ManageApiResult<Value>> {
    Json(ManageApiResult::<Value>::error(
        500,
        "Plugin testing was not implemented.",
    ))
}
