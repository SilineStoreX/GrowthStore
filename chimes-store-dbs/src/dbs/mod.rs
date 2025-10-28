use base64::Engine;
use chimes_store_core::{
    config::{
        auth::{AuthorizationConfig, JwtUserClaims},
        Column, ConditionItem, QueryCondition, StoreServiceConfig,
    },
    service::{convert::ConvertHolder, starter::MxStoreService},
    utils::{
        global_data::{rsa_encrypt_by_public_key, rsa_encrypt_with_public_key},
        ChineseCount,
    },
};
use crud::{DbCrud, DbStoreObject};
use fastdate::{Date, DateTime};
use futures_lite::Future;
use rbatis::{executor::Executor, rbatis_codegen::ops::AsProxy};
use serde_json::{json, Map, Number, Value};
use std::{collections::HashMap, pin::Pin, str::FromStr, sync::{Arc, Mutex}};
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

pub async fn convert_process(
    val: &Value,
    _ns: &str,
    col: &Column,
    holder: &mut ConvertHolder,
) -> Value {
    if let Some(convparam) = col.conv_params.clone() {
        let ret = match col.desensitize.clone().unwrap_or_default().as_str() {
            "dict" => {
                let dictcode = match val {
                    Value::String(dc) => dc,
                    Value::Number(tx) => &tx.to_string(),
                    _ => &val.to_string(),
                };
                if let Ok(Some(dc)) = holder
                    .transalte_dict(&convparam, &Value::String(dictcode.to_owned()))
                    .await
                {
                    dc.label.map(Value::String).unwrap_or(Value::Null)
                } else {
                    val.to_owned()
                }
            }
            "range" => match val.clone() {
                Value::String(text) => {
                    if col.col_type == Some("date".to_string())
                        || col.col_type == Some("time".to_string())
                        || col.col_type == Some("datetime".to_string())
                        || col.col_type == Some("timestamp".to_string())
                    {
                        let dateval = DateTime::from_str(&text).unwrap_or(DateTime::now());
                        if let Ok(Some(it)) = holder
                            .transalte_range::<DateTime>(&convparam, dateval)
                            .await
                        {
                            Value::String(it.item_label.unwrap_or_default())
                        } else {
                            val.to_owned()
                        }
                    } else {
                        let numval = Number::from_str(&text)
                            .unwrap_or(Number::from_f64(0.0).unwrap())
                            .as_f64()
                            .unwrap_or_default();
                        if let Ok(Some(it)) =
                            holder.transalte_range::<f64>(&convparam, numval).await
                        {
                            Value::String(it.item_label.unwrap_or_default())
                        } else {
                            val.to_owned()
                        }
                    }
                }
                Value::Number(num) => {
                    let numval = num.as_f64().unwrap_or_default();
                    if col.col_type == Some("datetime".to_string())
                        || col.col_type == Some("timestamp".to_string())
                    {
                        let dateval = DateTime::from_timestamp_millis(numval as i64);
                        if let Ok(Some(it)) = holder
                            .transalte_range::<DateTime>(&convparam, dateval)
                            .await
                        {
                            Value::String(it.item_label.unwrap_or_default())
                        } else {
                            val.to_owned()
                        }
                    } else if let Ok(Some(it)) =
                        holder.transalte_range::<f64>(&convparam, numval).await
                    {
                        Value::String(it.item_label.unwrap_or_default())
                    } else {
                        val.to_owned()
                    }
                }
                _ => val.to_owned(),
            },
            "dateformat" => {
                // if convert
                // 对数字类型进行格式化支持
                //match val.clone() {
                //    _ => {
                //        val.to_owned()
                //    }
                //}
                val.to_owned()
            }
            "subquery" => {
                if let Ok(Some(dc)) = holder.load_subquery(&convparam, val).await {
                    dc
                } else {
                    Value::Null
                }
            }
            _ => val.to_owned(),
        };
        ret
    } else {
        val.to_owned()
    }
}

pub async fn desensitize_process(
    text: String,
    ns: &str,
    stconf: &StoreServiceConfig,
    desensitize: &Option<String>,
    cs: bool,
    col: &Column,
    holder: &mut ConvertHolder,
) -> String {
    if should_return_plain_text(desensitize, cs) {
        text
    } else {
        match desensitize.clone().unwrap_or_default().as_str() {
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
            "dateformat" => {
                let basic_format = stconf
                    .datetime_format
                    .clone()
                    .unwrap_or("YYYY-MM-DDThh:mm:ss+00:00".to_owned());
                if let Some(convparam) = col.conv_params.clone() {
                    let dt = fastdate::DateTime::parse(&basic_format, &text)
                        .unwrap_or(fastdate::DateTime::now());
                    dt.format(&convparam)
                } else {
                    text
                }
            }
            "dict" => {
                if let Some(ctp) = col.conv_params.clone() {
                    if let Ok(Some(it)) = holder
                        .transalte_dict(&ctp, &Value::String(text.clone()))
                        .await
                    {
                        it.label.unwrap_or_default()
                    } else {
                        text
                    }
                } else {
                    text
                }
            }
            "range" => {
                if let Some(ctp) = col.conv_params.clone() {
                    if col.col_type == Some("date".to_string())
                        || col.col_type == Some("time".to_string())
                    {
                        let dateval = Date::from_str(&text)
                            .map(DateTime::from)
                            .unwrap_or(DateTime::now());
                        if let Ok(Some(it)) =
                            holder.transalte_range::<DateTime>(&ctp, dateval).await
                        {
                            it.item_label.unwrap_or_default()
                        } else {
                            text
                        }
                    } else if col.col_type == Some("datetime".to_string())
                        || col.col_type == Some("timestamp".to_string())
                    {
                        let dateval = DateTime::from_str(&text).unwrap_or(DateTime::now());
                        if let Ok(Some(it)) =
                            holder.transalte_range::<DateTime>(&ctp, dateval).await
                        {
                            it.item_label.unwrap_or_default()
                        } else {
                            text
                        }
                    } else {
                        let numval = Number::from_str(&text)
                            .unwrap_or(Number::from_f64(0.0).unwrap())
                            .as_f64()
                            .unwrap_or_default();
                        if let Ok(Some(it)) = holder.transalte_range::<f64>(&ctp, numval).await {
                            it.item_label.unwrap_or_default()
                        } else {
                            text
                        }
                    }
                } else {
                    text
                }
            }
            "subquery" => {
                let retcp = convert_process(&Value::String(text.clone()), ns, col, holder).await;
                match retcp {
                    Value::Null => String::new(),
                    Value::String(t) => t,
                    Value::Number(nb) => nb.to_string(),
                    _ => retcp.to_string(),
                }
            }
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
    col: Column,
    holder: Arc<Mutex<HashMap<String, Option<Value>>>>,
) ->  Pin<Box<dyn Future<Output = Value> + Send>> {
    Box::pin(async move { decode_relation_async(rb, &jwt, &stconf, rs, &ns, &col, holder).await })
    // decode_relation_async(rb, &jwt, &stconf, rs, &ns, &col, holder).await
}

pub async fn decode_relation_async(
    rb: Arc<dyn Executor>,
    jwt: &JwtUserClaims,
    stconf: &StoreServiceConfig,
    rs: rbs::Value,
    ns: &str,
    col: &Column,
    holder: Arc<Mutex<HashMap<String, Option<Value>>>>,
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
                    let cachec_key = format!("{field}={rlid}");
                    let cached_value =  holder.lock().unwrap().get(&cachec_key).map(|t| t.to_owned()).unwrap_or(None);
                    if let Some(val) = cached_value {
                        val
                    } else {
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

                        let result = if col.relation_array {
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
                        };
                        holder.lock().unwrap().insert(cachec_key, Some(result.clone()));
                        result
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
    holder: &mut ConvertHolder,
) -> Value {
    let col_type = col.col_type.clone().unwrap_or_default().to_lowercase();
    match col_type.as_str() {
        "String" | "str" | "string" | "text" => {
            let text = match rs.clone() {
                rbs::Value::Binary(t) => String::from_utf8_lossy(&t).to_string(),
                rbs::Value::String(t) => t,
                _ => match rbatis::decode::<String>(rs.clone()) {
                    Ok(rt) => rt,
                    Err(_err) => rs.string(),
                },
            };

            let desents = desensitize_process(
                text,
                ns,
                stconf,
                &col.desensitize,
                col.crypto_store,
                col,
                holder,
            )
            .await;

            Value::String(desents)
        }
        "int" | "i64" | "u64" | "integer" | "long" | "i32" | "u32" => {
            let numb = match rbatis::decode::<Value>(rs) {
                Ok(rt) => rt,
                Err(_) => Value::Number(Number::from(0)),
            };

            convert_process(&numb, ns, col, holder).await
        }
        "bigint" => {
            let numb = match rbatis::decode::<Value>(rs) {
                Ok(rt) => json!(rt.to_string()),
                Err(err) => {
                    log::warn!("err convert {err}");
                    Value::Null
                }
            };
            convert_process(&numb, ns, col, holder).await
        }
        "bigdecimal" => {
            let numb = match rbatis::decode::<Value>(rs) {
                Ok(rt) => json!(rt.to_string()),
                Err(_) => Value::Null,
            };
            convert_process(&numb, ns, col, holder).await
        }
        "number" | "float" | "f32" | "f64" | "double" => {
            let number = match rbatis::decode::<Value>(rs) {
                Ok(rt) => rt,
                Err(_) => Value::Number(Number::from(0)),
            };

            convert_process(&number, ns, col, holder).await
        }
        "datetime" | "timestamp" => {
            let tt = match rbatis::decode::<Value>(rs.clone()) {
                Ok(rt) => {
                    // log::info!("decode timestamp {rt:?}");
                    if stconf.relaxy_timezone {
                        match rt {
                            Value::String(ts) => Value::String(ts.replace('Z', "")),
                            Value::Number(tm) => {
                                let t = rbatis::rbdc::DateTime::from_timestamp_millis(
                                    tm.as_i64().unwrap_or_default(),
                                );
                                Value::String(t.to_string().replace('T', " ").replace('Z', ""))
                            }
                            _ => Value::String(rt.to_string().replace('Z', "")),
                        }
                    } else {
                        match rt {
                            Value::Number(tm) => Value::String(
                                rbatis::rbdc::DateTime::from_timestamp_millis(
                                    tm.as_i64().unwrap_or_default(),
                                )
                                .to_string(),
                            ),
                            _ => rt,
                        }
                    }
                }
                Err(err) => {
                    log::debug!("decode timestamp {err:?}");
                    if stconf.relaxy_timezone {
                        Value::String(rs.string().replace('Z', ""))
                    } else {
                        Value::String(rs.string())
                    }
                }
            };

            if col.desensitize == Some("dateformat".to_owned()) {
                let basic_format = stconf
                    .datetime_format
                    .clone()
                    .unwrap_or("YYYY-MM-DDThh:mm:ss+00:00".to_owned());
                if let Some(convparam) = col.conv_params.clone() {
                    // log::warn!("convert fmt: {convparam}");
                    // TODO: dateformat should be changed
                    match tt {
                        Value::String(t) => {
                            let dt = fastdate::DateTime::parse(&basic_format, &t)
                                .unwrap_or(fastdate::DateTime::now());
                            Value::String(dt.format(&convparam))
                        }
                        _ => {
                            let t = tt.to_string();
                            let dt = fastdate::DateTime::parse(&basic_format, &t)
                                .unwrap_or(fastdate::DateTime::now());
                            Value::String(dt.format(&convparam))
                        }
                    }
                } else {
                    tt
                }
            } else {
                convert_process(&tt, ns, col, holder).await
            }
        }
        "date" | "time" => {
            let tt = match rbatis::decode::<Value>(rs.clone()) {
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
            };

            if col.desensitize == Some("dateformat".to_owned()) {
                let basic_format = stconf
                    .datetime_format
                    .clone()
                    .unwrap_or("YYYY-MM-DDThh:mm:ss+00:00".to_owned());
                if let Some(convparam) = col.conv_params.clone() {
                    match tt {
                        Value::String(t) => {
                            let dt = fastdate::DateTime::parse(&basic_format, &t)
                                .unwrap_or(fastdate::DateTime::now());
                            Value::String(dt.format(&convparam))
                        }
                        _ => {
                            let t = tt.to_string();
                            let dt = fastdate::DateTime::parse(&basic_format, &t)
                                .unwrap_or(fastdate::DateTime::now());
                            Value::String(dt.format(&convparam))
                        }
                    }
                } else {
                    tt
                }
            } else {
                convert_process(&tt, ns, col, holder).await
            }
        }
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
                        log::debug!("Could not decode binary to json object {err:?}");
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
    holder: &mut ConvertHolder,
    cache_holder: Arc<Mutex<HashMap<String, Option<Value>>>>,
) -> Result<Value, anyhow::Error> {
    // let archolder = Arc::new(Mutex::new(holder.to_owned()));
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
                            decode_relation(rb_, jwt_.to_owned(), stconf_, v, ns_, col_, cache_holder.clone()).await
                        } else {
                            decode_val_by_type(rb.clone(), jwt, stconf, v, ns, col, holder).await
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
    holder: &mut ConvertHolder,
    cache_holder: Arc<Mutex<HashMap<String, Option<Value>>>>,
) -> Result<Value, anyhow::Error> {
    match rs {
        rbs::Value::Map(mp) => {
            let mut obj = Map::new();
            for (k, v) in mp.clone() {
                let key = k.string();
                if let Some(col) = fields.get(&key) {
                    let prop_name = col.prop_name.clone().unwrap_or(col.field_name.clone());
                    if col.col_type != Some("relation".to_owned()) {
                        let val =
                            decode_val_by_type(rb.clone(), jwt, stconf, v, ns, col, holder).await;
                        obj.insert(prop_name, val);
                    } else {
                        let rb_ = rb.clone();
                        let jwt_ = jwt.clone();
                        let stconf_ = stconf.clone();
                        let ns_ = ns.to_owned().clone();
                        let col_ = col.clone();
                        // log::info!("Decode Relation: {ns_}/{col_:?}");
                        let val = decode_relation(rb_, jwt_, stconf_, v, ns_, col_, cache_holder.clone()).await;
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
                            // log::info!("Decode Relation: {ns_}/{col_:?}");
                            let mtv = v.clone();
                            decode_relation(rb_, jwt_, stconf_, mtv, ns_, col_, cache_holder.clone()).await
                        } else {
                            decode_val_by_type(
                                rb.clone(),
                                jwt,
                                stconf,
                                v.to_owned(),
                                ns,
                                col,
                                holder,
                            )
                            .await
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
    holder: &mut ConvertHolder,
    cache_holder: Arc<Mutex<HashMap<String, Option<Value>>>>,
) -> Result<Vec<Value>, anyhow::Error> {
    match rs {
        rbs::Value::Array(list) => {
            // log::info!("decode the array of query result. {ns}");
            let mut rets = vec![];
            for tp in list {
                let tv = if let Ok(ts) =
                    decode_map_custom_fields_list(rb.clone(), jwt, stconf, tp, fields, ns, holder, cache_holder.clone())
                        .await
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
    holder: &mut ConvertHolder,
    cache_holder: Arc<Mutex<HashMap<String, Option<Value>>>>,
) -> Result<Vec<Value>, anyhow::Error> {
    match rs {
        rbs::Value::Array(list) => {
            // log::info!("decode the array of query result. {ns}");
            let mut rets = vec![];
            for tp in list {
                let tv = if let Ok(ts) =
                    decode_map_custom_fields_map(rb.clone(), jwt, stconf, tp, fields, ns, holder, cache_holder.clone())
                        .await
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
        None => Value::Null,
    }
}

pub fn refine_column_value(t: &Value, col: &Column) -> Value {
    let col_type = col
        .col_type
        .clone()
        .unwrap_or(col.field_type.clone().unwrap_or_default());
    match col_type.to_lowercase().as_str() {
        "integer" | "bigint" => match t {
            Value::String(tvs) => {
                if let Ok(v) = tvs.parse::<i64>() {
                    json!(v)
                } else {
                    json!(0)
                }
            }
            Value::Number(_) => t.clone(),
            _ => json!(0.0),
        },
        "float" | "double" => match t {
            Value::String(tvs) => {
                if let Ok(v) = tvs.parse::<f64>() {
                    json!(v)
                } else {
                    json!(0)
                }
            }
            Value::Number(_) => t.clone(),
            _ => json!(0.0),
        },
        "bigdecimal" => match t {
            Value::String(tvs) => {
                if let Ok(v) = tvs.parse::<rbdc::Decimal>() {
                    json!(v)
                } else {
                    json!(0)
                }
            }
            Value::Number(_) => t.clone(),
            _ => json!(0.0),
        },
        "bool" => match t {
            Value::String(tvs) => {
                if let Ok(v) = tvs.parse::<bool>() {
                    json!(v)
                } else {
                    json!(false)
                }
            }
            Value::Number(tmv) => {
                if tmv.as_i64().unwrap_or_default() > 0 {
                    json!(true)
                } else {
                    json!(false)
                }
            }
            _ => t.clone(),
        },
        _ => t.clone(),
    }
}
