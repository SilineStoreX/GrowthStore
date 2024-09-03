use base64::Engine;
use chimes_store_core::{
    config::{
        auth::{AuthorizationConfig, JwtUserClaims},
        Column, ConditionItem, QueryCondition, StoreServiceConfig,
    },
    service::starter::MxStoreService,
    utils::{
        global_data::{rsa_encrypt_by_public_key, rsa_encrypt_with_public_key},
        ChineseCount,
    },
};
use crud::{DbCrud, DbStoreObject};
use futures_lite::Future;
use rbatis::{executor::Executor, rbatis_codegen::ops::AsProxy};
use serde_json::{json, Map, Number, Value};
use std::{collections::HashMap, pin::Pin, sync::Arc};
use substring::Substring;

pub mod crud;
pub mod invoker;
pub mod query;
pub mod redis;

fn should_return_plain_text(des: &Option<String>, cs: bool) -> bool {
    if des.is_none() {
        true
    } else {
        let destize = des.clone().unwrap_or_default();
        cs && (destize == "aes" || destize == "base64" || destize == "rsa")
    }
}

fn is_desensitize_with_crypto_store(des: &Option<String>, cs: bool) -> bool {
    if des.is_none() {
        false
    } else {
        let destize = des.clone().unwrap_or_default();
        cs && (destize == "aes" || destize == "base64" || destize == "rsa")
    }
}

pub fn crypto_desenstize_process(text: String, ns: &str, desensitize: &Option<String>) -> String {
    match desensitize.clone().unwrap().as_str() {
        "aes" => match MxStoreService::get(ns) {
            Some(mts) => mts.aes_encode_text(&text),
            None => text,
        },
        "base64" => base64::engine::general_purpose::STANDARD.encode(text),
        "rsa" => match MxStoreService::get(ns) {
            Some(mss) => match mss.get_config().rsa_public_key {
                Some(pk) => rsa_encrypt_with_public_key(&text, &pk).unwrap_or_default(),
                None => rsa_encrypt_by_public_key(&text).unwrap_or_default(),
            },
            None => rsa_encrypt_by_public_key(&text).unwrap_or_default(),
        },
        _ => text,
    }
}

pub fn desensitize_process(text: String, ns: &str, desensitize: &Option<String>, cs: bool) -> String {
    if should_return_plain_text(desensitize, cs) {
        text
    } else {
        match desensitize.clone().unwrap().as_str() {
            "aes" => match MxStoreService::get(ns) {
                Some(mts) => mts.aes_encode_text(&text),
                None => text,
            },
            "replace" => {
                if text.chars_len() < 6 {
                    "*****".to_owned()
                } else if text.chars_len() > 10 {
                    let rt = text.substring(0, 4);
                    let et = text.substring(text.chars_len() - 5, text.chars_len());
                    log::info!("et: {et}, {}", text.chars_len() - 5);
                    format!("{rt}****{et}")
                } else {
                    let rt = text.substring(0, 2);
                    let et = text.substring(text.chars_len() - 3, text.chars_len());
                    format!("{rt}****{et}")
                }
            }
            "base64" => base64::engine::general_purpose::STANDARD.encode(text),
            "rsa" => match MxStoreService::get(ns) {
                Some(mss) => match mss.get_config().rsa_public_key {
                    Some(pk) => rsa_encrypt_with_public_key(&text, &pk).unwrap_or_default(),
                    None => rsa_encrypt_by_public_key(&text).unwrap_or_default(),
                },
                None => rsa_encrypt_by_public_key(&text).unwrap_or_default(),
            },
            "null" => String::new(),
            _ => text,
        }
    }
}

pub fn decode_relation(
    rb: Arc<dyn Executor>,
    jwt: JwtUserClaims,
    stconf: StoreServiceConfig,
    rs: rbs::Value,
    ns: String,
    col: Column) -> Pin<Box<dyn Future<Output = Value> + Send>> {
    Box::pin(async move {
        decode_relation_async(rb, &jwt, &stconf, rs, &ns, &col).await
    })
}

pub async fn decode_relation_async(
    rb: Arc<dyn Executor>,
    jwt: &JwtUserClaims,
    stconf: &StoreServiceConfig,
    rs: rbs::Value,
    ns: &str,
    col: &Column,
) -> Value {
    let col_type = col.col_type.clone().unwrap_or_default().to_lowercase();
    if "relation" == col_type.as_str() {
        // relation is doing another query for object
        let rlid = match rbatis::decode::<Value>(rs) {
            Ok(rt) => rt,
            Err(_) => {
                return Value::Null;
            }
        };
        if let Some(relation_object) = col.relation_object.clone() {
            if let Some(sto) = stconf.get_object(&relation_object) {
                if let Some(field) = col.relation_field.clone() {
                    let dso = DbStoreObject(
                        sto.to_owned(),
                        stconf.to_owned(),
                        AuthorizationConfig::get(),
                    );
                    let mut qs = QueryCondition::default();
                    qs.and.push(ConditionItem {
                        field,
                        op: "=".to_string(),
                        value: rlid,
                        value2: Value::Null,
                        and: vec![],
                        or: vec![],
                    });
                    if col.relation_array {
                        match dso.query(rb, jwt, &qs).await {
                            Ok(res) => Value::Array(res),
                            Err(_) => Value::Null,
                        }
                    } else {
                        match dso.query(rb, jwt, &qs).await {
                            Ok(res) => {
                                if res.is_empty() {
                                    Value::Null
                                } else {
                                    res[0].clone()
                                }
                            }
                            Err(_) => Value::Null,
                        }
                    }
                } else {
                    log::warn!(
                        "Column was defined as relation but the relation field was not specifield."
                    );
                    Value::Null
                }
            } else {
                log::warn!("Column was defined as relation but the relative object was not found in current namespace {ns}.");
                Value::Null
            }
        } else {
            log::warn!("Column was defined as relation but there is not target relation object specifield.");
            Value::Null
        }
    } else {
        Value::Null
    }
}

pub async fn decode_val_by_type(
    _rb: Arc<dyn Executor>,
    _jwt: &JwtUserClaims,
    stconf: &StoreServiceConfig,
    rs: rbs::Value,
    ns: &str,
    col: &Column,
) -> Value {
    let col_type = col.col_type.clone().unwrap_or_default().to_lowercase();
    match col_type.as_str() {
        "String" | "str" | "string" | "text" => match rs.clone() {
            rbs::Value::Binary(t) => Value::String(desensitize_process(
                String::from_utf8_lossy(&t).to_string(),
                ns,
                &col.desensitize,
                col.crypto_store,
            )),
            rbs::Value::String(t) => Value::String(desensitize_process(t, ns, &col.desensitize, col.crypto_store)),
            _ => match rbatis::decode::<String>(rs.clone()) {
                Ok(rt) => Value::String(desensitize_process(rt, ns, &col.desensitize, col.crypto_store)),
                Err(err) => {
                    log::debug!("Error {:?}", err);
                    Value::String(desensitize_process(rs.string(), ns, &col.desensitize, col.crypto_store))
                }
            },
        },
        "number" | "int" | "i64" | "u64" | "integer" | "long" | "i32" | "u32" | "bigint" => {
            match rbatis::decode::<Value>(rs) {
                Ok(rt) => rt,
                Err(_) => Value::Number(Number::from(0)),
            }
        }
        "float" | "f32" | "f64" | "double" => match rbatis::decode::<Value>(rs) {
            Ok(rt) => rt,
            Err(_) => Value::Number(Number::from(0)),
        },
        "datetime" | "timestamp" => match rbatis::decode::<Value>(rs.clone()) {
            Ok(rt) => {
                // log::info!("decode timestamp {rt:?}");
                if stconf.relaxy_timezone {
                    match rt {
                        Value::String(ts) => Value::String(ts.replace('Z', "")),
                        Value::Number(tm) => {
                            let t = rbatis::rbdc::DateTime::from_timestamp_millis(tm.as_i64().unwrap_or_default()); 
                            Value::String(t.to_string().replace('T', " ").replace('Z', ""))
                        },
                        _ => Value::String(rt.to_string().replace('Z', "")),
                    }
                } else {
                    match rt {
                        Value::Number(tm) => {
                            Value::String(rbatis::rbdc::DateTime::from_timestamp_millis(tm.as_i64().unwrap_or_default()).to_string())
                        },
                        _ => rt
                    }
                }
            }
            Err(err) => {
                log::info!("decode timestamp {err:?}");
                if stconf.relaxy_timezone {
                    Value::String(rs.string().replace('Z', ""))
                } else {
                    Value::String(rs.string())
                }
            }
        },
        "date" | "time" => match rbatis::decode::<Value>(rs.clone()) {
            Ok(rt) => {
                // log::info!("decode time {rt:?}");
                if stconf.relaxy_timezone {
                    match rt {
                        Value::String(ts) => Value::String(ts.replace('Z', "")),
                        _ => Value::String(rt.to_string()),
                    }
                } else {
                    rt
                }
            }
            Err(_err) => {
                // log::info!("decode timestamp {err:?}");
                if stconf.relaxy_timezone {
                    Value::String(rs.string().replace('Z', ""))
                } else {
                    Value::String(rs.string())
                }
            }
        },
        "numeric" | "decimal" => match rbatis::decode::<Value>(rs.clone()) {
            Ok(rt) => rt,
            Err(_) => Value::String(rs.string()),
        },
        "json" | "JSON" | "jsonb" => match rbatis::decode::<Value>(rs) {
            Ok(rt) => rt,
            Err(_) => Value::Null,
        },
        "bool" | "boolean" | "Boolean" => match rbatis::decode::<Value>(rs) {
            Ok(rt) => rt,
            Err(_) => Value::Bool(false),
        },
        "binnary" => {
            if col.base64 {
                let base64text = match rs {
                    rbs::Value::Binary(bin) => {
                        base64::engine::general_purpose::STANDARD.encode(bin)
                    }
                    _ => base64::engine::general_purpose::STANDARD.encode(rs.to_string()),
                };
                Value::String(base64text)
            } else {
                match rbatis::decode::<Value>(rs) {
                    Ok(rt) => rt,
                    Err(err) => {
                        log::debug!("Could not decode binary to json object {:?}", err);
                        Value::Null
                    }
                }
            }
        }
        _ => match rbatis::decode::<Value>(rs) {
            Ok(rt) => rt,
            Err(_) => Value::Null,
        },
    }
}

pub async fn decode_map_custom_fields_list(
    rb: Arc<dyn Executor>,
    jwt: &JwtUserClaims,
    stconf: &StoreServiceConfig,
    rs: rbs::Value,
    fields: &Vec<Column>,
    ns: &str,
) -> Result<Value, anyhow::Error> {
    match rs {
        rbs::Value::Map(mp) => {
            let mut obj = Map::new();
            for col in fields {
                let propname = col.prop_name.clone().unwrap_or(col.field_name.clone());
                if !mp
                    .0
                    .contains_key(&rbs::Value::String(col.field_name.clone()))
                {
                    obj.insert(propname, Value::Null);
                } else {
                    let v = mp[col.field_name.clone().as_str()].clone();
                    if v.is_null() {
                        obj.insert(propname, Value::Null);
                    } else {
                        let val = if col.col_type.clone().unwrap_or_default().to_lowercase()
                            == "relation"
                        {
                            let rb_ = rb.clone();
                            let jwt_ = jwt.clone();
                            let stconf_ = stconf.clone();
                            let ns_ = ns.to_owned().clone();
                            let col_ = col.clone();
                            decode_relation(rb_, jwt_.to_owned(), stconf_, v, ns_, col_).await
                        } else {
                            decode_val_by_type(rb.clone(), jwt, stconf, v, ns, col).await
                        };
                        obj.insert(propname, val);
                    }
                }
            }
            Ok(Value::Object(obj))
        }
        _ => Ok(rbatis::decode::<Value>(rs)?),
    }
}

pub async fn decode_map_custom_fields_map(
    rb: Arc<dyn Executor>,
    jwt: &JwtUserClaims,
    stconf: &StoreServiceConfig,
    rs: rbs::Value,
    fields: &HashMap<String, Column>,
    ns: &str,
) -> Result<Value, anyhow::Error> {
    match rs {
        rbs::Value::Map(mp) => {
            let mut obj = Map::new();
            for (k, v) in mp.clone() {
                let key = k.string();
                if let Some(col) = fields.get(&key) {
                    let prop_name = col.prop_name.clone().unwrap_or(col.field_name.clone());
                    if col.col_type != Some("relation".to_owned()) {
                        let val = decode_val_by_type(rb.clone(), jwt, stconf, v, ns, col).await;
                        obj.insert(prop_name, val);
                    } else {
                        let rb_ = rb.clone();
                        let jwt_ = jwt.clone();
                        let stconf_ = stconf.clone();
                        let ns_ = ns.to_owned().clone();
                        let col_ = col.clone();
                        log::info!("Decode Relation: {ns_}/{col_:?}");
                        let val = decode_relation(rb_, jwt_, stconf_, v, ns_, col_).await;
                        obj.insert(prop_name, val);
                    }
                } else if let Ok(val) = rbatis::decode::<Value>(v) {
                    obj.insert(key, val);
                } else {
                    obj.insert(key, Value::Null);
                }
            }

            for col in fields.values() {
                let propname = col.prop_name.clone().unwrap_or(col.field_name.clone());
                if !obj.contains_key(&propname) {
                    if let Some(v) = mp.0.get(&rbs::Value::String(col.field_name.clone())) {
                        let val = if col.col_type.clone().unwrap_or_default().to_lowercase()
                            == "relation"
                        {
                            let rb_ = rb.clone();
                            let jwt_ = jwt.clone();
                            let stconf_ = stconf.clone();
                            let ns_ = ns.to_owned().clone();
                            let col_ = col.clone();
                            log::info!("Decode Relation: {ns_}/{col_:?}");
                            let mtv = v.clone();
                            decode_relation(rb_, jwt_, stconf_, mtv, ns_, col_).await
                        } else {
                            decode_val_by_type(rb.clone(), jwt, stconf, v.to_owned(), ns, col).await
                        };
                        obj.insert(propname, val);
                    }
                }
            }
            Ok(Value::Object(obj))
        }
        _ => Ok(rbatis::decode::<Value>(rs)?),
    }
}

pub async fn decode_vec_custom_fields_list(
    rb: Arc<dyn Executor>,
    jwt: &JwtUserClaims,
    stconf: &StoreServiceConfig,
    rs: rbs::Value,
    fields: &Vec<Column>,
    ns: &str,
) -> Result<Vec<Value>, anyhow::Error> {
    match rs {
        rbs::Value::Array(list) => {
            log::info!("decode the array of query result. {ns}");
            let mut rets = vec![];
            for tp in list {
                let tv = if let Ok(ts) =
                    decode_map_custom_fields_list(rb.clone(), jwt, stconf, tp, fields, ns).await
                {
                    ts
                } else {
                    Value::Null
                };
                rets.push(tv);
            }
            Ok(rets)
        }
        _ => Ok(rbatis::decode::<Vec<Value>>(rs)?),
    }
}

pub async fn decode_vec_custom_fields(
    rb: Arc<dyn Executor>,
    jwt: &JwtUserClaims,
    stconf: &StoreServiceConfig,
    rs: rbs::Value,
    fields: &HashMap<String, Column>,
    ns: &str,
) -> Result<Vec<Value>, anyhow::Error> {
    match rs {
        rbs::Value::Array(list) => {
            log::info!("decode the array of query result. {ns}");
            let mut rets = vec![];
            for tp in list {
                let tv = if let Ok(ts) =
                    decode_map_custom_fields_map(rb.clone(), jwt, stconf, tp, fields, ns).await
                {
                    ts
                } else {
                    Value::Null
                };
                rets.push(tv);
            }
            Ok(rets)
        }
        _ => Ok(rbatis::decode::<Vec<Value>>(rs)?),
    }
}

pub fn refine_column_value_option(t: &Option<&Value>, col: &Column) -> Value {
    match t {
        Some(tt) => refine_column_value(tt, col),
        None => Value::Null
    }
}

pub fn refine_column_value(t: &Value, col: &Column) -> Value {
    let col_type = col.col_type.clone().unwrap_or(col.field_type.clone().unwrap_or_default());
    match col_type.to_lowercase().as_str() {
        "integer" => {
            match t {
                Value::String(tvs) => {
                    if let Ok(v) = tvs.parse::<i64>() {
                        json!(v)
                    } else {
                        json!(0)
                    }
                },
                Value::Number(_) => {
                    t.clone()
                },
                _ => json!(0.0)
            }
        },
        "float" | "double" => {
            match t {
                Value::String(tvs) => {
                    if let Ok(v) = tvs.parse::<f64>() {
                        json!(v)
                    } else {
                        json!(0)
                    }
                },
                Value::Number(_) => {
                    t.clone()
                },
                _ => json!(0.0)
            }
        },
        "bool" => {
            match t {
                Value::String(tvs) => {
                    if let Ok(v) = tvs.parse::<bool>() {
                        json!(v)
                    } else {
                        json!(false)
                    }
                },
                Value::Number(tmv) => {
                    if tmv.as_i64().unwrap_or_default() > 0 {
                        json!(true)
                    } else {
                        json!(false)
                    }
                },
                _ => t.clone()
            }
        },
        _ => {
            t.clone()
        }
    }
}

