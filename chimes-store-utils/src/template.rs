use std::{collections::HashMap, str::FromStr, sync::Mutex};

use crate::{
    algorithm::{md5_hash, sha1_256_hash, sha2_256_hash},
    common::generate_rand_string_v2,
    crypto::{hmac_sha1, hmac_sha256, hmac_sha512, sha256_with_rsa_sign},
};
use anyhow::anyhow;
use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDateTime};
use chrono::{Days, Months, TimeDelta};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tera::{Context, Tera};

const TERA_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.6f %z";
const TERA_DATETIME_BASE_TZ_FORMAT: &str = "%Y-%m-%d %H:%M:%S %z";
const TERA_DATETIME_BASE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const TERA_DATE_BASE_FORMAT: &str = "%Y-%m-%d";

pub trait MxStoreServiceProxy: Sync + Send {
    fn get_config(&self, ns: &str) -> Value;
}


#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SchemaObjectType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, SchemaObjectType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<SchemaObjectType>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", rename="maxLength")]
    pub max_length: Option<i64>,
    
    #[serde(skip_serializing_if = "Option::is_none", rename="minLength")]
    pub min_length: Option<i64>,
}

impl SchemaObjectType {
    pub fn new() -> Self {
        Self {
            r#type: Some("object".to_string()), 
            properties: HashMap::new(),
            ..Default::default()
         }
    }

    pub fn properties(&self) -> Vec<&String> {
        self.properties.keys().sorted().collect_vec()
    }
}

pub static MX_STORE_SERVICE_PROXY: Mutex<Option<Box<dyn MxStoreServiceProxy>>> = Mutex::new(None);

pub fn set_mx_store_service_proxy(proxy: Box<dyn MxStoreServiceProxy>) {
    MX_STORE_SERVICE_PROXY.lock().unwrap().replace(proxy);
}

pub fn get_mx_config(ns: &str) -> Value {
    MX_STORE_SERVICE_PROXY
        .lock()
        .unwrap()
        .as_ref()
        .map(|f| f.get_config(ns))
        .unwrap_or(Value::Null)
}

pub fn get_json_value(prop: &str, conf: Value) -> Value {
    match conf {
        Value::Object(mp) => mp.get(prop).map(|t| t.to_owned()).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

pub fn value_to_string(val: Value) -> String {
    match val {
        Value::String(t) => t,
        _ => val.to_string(),
    }
}

/**
 * function to impl hmac_sha1
 */
pub fn sha256_with_rsa_sign_text(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        if let Some(Value::String(ns)) = args.get("ns") {
            let conf = get_mx_config(ns);
            let val_rsa_priv_key = get_json_value("rsa_private_key", conf);
            let rsa_priv_key = value_to_string(val_rsa_priv_key);
            let ret = sha256_with_rsa_sign(&data.as_bytes(), &rsa_priv_key);
            Ok(Value::String(ret))
        } else {
            Err(tera::Error::msg("No ns arg provided"))
        }
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }
}

/**
 * function to impl hmac_sha1
 */
pub fn random_string(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::Number(data)) = args.get("len") {
        let ret = generate_rand_string_v2(data.as_u64().unwrap_or(10) as usize);
        Ok(Value::String(ret))
    } else {
        Err(tera::Error::msg("No len arg provided"))
    }
}

pub fn snowflake_id(_args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    Ok(json!(crate::algorithm::snowflake_id()))
}

/**
 * function to impl hmac_sha1
 */
pub fn sha1_text(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let ret = sha1_256_hash(&data.as_bytes());
        Ok(Value::String(ret))
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }
}

/**
 * function to impl hmac_sha1
 */
pub fn text_md5(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let ret = md5_hash(&data.as_bytes());
        Ok(Value::String(ret))
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }
}

/**
 * function to impl hmac_sha1
 */
pub fn sha2_text(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let ret = sha2_256_hash(&data.as_bytes());
        Ok(Value::String(ret))
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }
}

/**
 * function to impl hmac_sha1
 */
pub fn hmac_sha1_tera(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(key)) = args.get("key") {
        if let Some(Value::String(data)) = args.get("text") {
            let codec = hmac_sha1(key, data);
            Ok(Value::Array(
                codec
                    .into_iter()
                    .map(|f| Value::Number(f.into()))
                    .collect_vec(),
            ))
        } else {
            Err(tera::Error::msg("No text arg provided"))
        }
    } else {
        Err(tera::Error::msg("No key arg provided"))
    }
}

/**
 * function to impl hmac_sha1
 */
pub fn hmac_sha256_tera(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(key)) = args.get("key") {
        if let Some(Value::String(data)) = args.get("text") {
            let codec = hmac_sha256(key, data);
            Ok(Value::Array(
                codec
                    .into_iter()
                    .map(|f| Value::Number(f.into()))
                    .collect_vec(),
            ))
        } else {
            Err(tera::Error::msg("No text arg provided"))
        }
    } else {
        Err(tera::Error::msg("No key arg provided"))
    }
}

/**
 * function to impl hmac_sha1
 */
pub fn hmac_sha512_tera(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(key)) = args.get("key") {
        if let Some(Value::String(data)) = args.get("text") {
            let codec = hmac_sha512(key, data);
            Ok(Value::Array(
                codec
                    .into_iter()
                    .map(|f| Value::Number(f.into()))
                    .collect_vec(),
            ))
        } else {
            Err(tera::Error::msg("No text arg provided"))
        }
    } else {
        Err(tera::Error::msg("No key arg provided"))
    }
}

/**
 * function to impl base64_encode
 */
pub fn text_base32_encode(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let ret = crate::algorithm::base32_encode(&data.as_bytes());
        Ok(Value::String(ret))
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }
}

/**
 * function to impl base64_decode
 */
pub fn text_base32_decode(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let decode_data = crate::algorithm::base32_decode(data);
        if let Ok(ret) = String::from_utf8(decode_data) {
            Ok(Value::String(ret))
        } else {
            Ok(Value::Null)
        }
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }
}

/**
 * function to impl base64_encode
 */
pub fn text_base64_encode(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let ret = crate::algorithm::base64_encode(&data.as_bytes());
        Ok(Value::String(ret))
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }
}

/**
 * function to impl base64_decode
 */
pub fn text_base64_decode(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let decode_data = crate::algorithm::base64_decode(&data.as_bytes());
        if let Ok(ret) = String::from_utf8(decode_data) {
            Ok(Value::String(ret))
        } else {
            Ok(Value::Null)
        }
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }
}

pub fn json_path(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    tracing::info!("args: {args:?}");
    if let Some(arg) = args.get("arg") {
        if let Some(Value::String(path)) = args.get("path") {
            if let Some(t) = json_path_get(arg, path) {
                return Ok(t);
            }
        } else {
            return Err(tera::Error::msg("No path provided"));
        }
    } else {
        return Err(tera::Error::msg("No arg provided"));
    }
    Ok(Value::Null)
}

pub fn to_json(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(arg) = args.get("value") {
        match serde_json::to_string_pretty(arg) {
            Ok(t) => Ok(Value::String(t)),
            Err(err) => Err(tera::Error::msg(err)),
        }
    } else {
        Err(tera::Error::msg("No value provided"))
    }
}

pub fn template_eval(script: &str, ctx: Value) -> Result<String, anyhow::Error> {
    let mut tera = match Tera::new("templates/**/*.tera") {
        Ok(t) => t,
        Err(err) => {
            tracing::info!("Could not found tera context {err}, then use default Tera.");
            // return Err(ChimesError::custom(310, err.to_string()));
            Tera::default()
        }
    };

    let context = match Context::from_serialize(ctx) {
        Ok(c) => c,
        Err(_) => Context::new(),
    };

    tera.register_function("random_string", random_string);
    tera.register_function("snowflake_id", snowflake_id);
    tera.register_function("sha1_text", sha1_text);
    tera.register_function("sha2_text", sha2_text);
    tera.register_function("hmac_sha1", hmac_sha1_tera);
    tera.register_function("hmac_sha256", hmac_sha256_tera);
    tera.register_function("hmac_sha512", hmac_sha512_tera);
    tera.register_function("md5string", text_md5);
    tera.register_function("base64_encode", text_base64_encode);
    tera.register_function("base64_decode", text_base64_decode);
    tera.register_function("base32_encode", text_base32_encode);
    tera.register_function("base32_decode", text_base32_decode);
    tera.register_function("sha256_with_rsa_sign", sha256_with_rsa_sign_text);
    tera.register_function("canonicalized_query", canonicalized_query);
    tera.register_filter("date_add", date_add);
    tera.register_filter("month_day", month_day);
    tera.register_function("jsonpath", json_path);
    tera.register_function("to_json", to_json);
    match tera.render_str(script, &context) {
        Ok(text) => Ok(text),
        Err(err) => {
            tracing::info!("err: {err:?}");
            Err(anyhow!(err.to_string()))
        }
    }
}

pub fn json_path_get(t: &Value, path: &str) -> Option<Value> {
    let jspath = if path.starts_with("$.") {
        path.to_owned()
    } else if path.is_empty() {
        "$".to_string()
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

#[allow(dead_code)]
pub fn json_path_get_string(t: &Value, path: &str) -> String {
    json_path_get(t, path)
        .map(|f| match f {
            Value::String(t) => t,
            _ => f.to_string(),
        })
        .unwrap_or_default()
}

#[allow(dead_code)]
pub fn json_path_get_string_with(t: &Value, path: &str, defval: &str) -> String {
    json_path_get(t, path)
        .map(|f| match f {
            Value::String(t) => t,
            _ => f.to_string(),
        })
        .unwrap_or(defval.to_owned())
}

/**
 * 对指定参数进行排序
 * 该参数须为Value::Object
 */
pub fn canonicalized_value(arg: &Value, param_schema: &Option<SchemaObjectType>, desc: bool) -> Result<Value, anyhow::Error> {
    if let Value::Object(val) = arg {
        let mut sorted = if let Some(schm) = param_schema {
            let props = schm.properties();
            if props.is_empty() {
                val.keys().filter(|p| **p != "sign".to_string()).collect_vec()
            } else {
                props
            }
        } else {
            val.keys().filter(|p| **p != "sign".to_string()).collect_vec()
        };


        if desc {
            sorted.sort_by(|a, b| b.cmp(a));
        } else {
            sorted.sort();
        }
        let mut text = vec![];
        for key in sorted {
            let val_str = val
                .get(key)
                .map(|f| {
                    match f {
                        Value::Null => String::new(),
                        Value::Bool(t) => t.to_string(),
                        Value::Number(n) => n.to_string(),
                        Value::String(s) => s.to_string(),
                        _ => serde_json::to_string(f).unwrap_or_default()
                    }
                })
                .unwrap_or_default();
            text.push(format!("{key}={val_str}"));
        }
        let ret = text.join("&");
        // println!("Text={ret}");
        Ok(Value::String(ret))
    } else {
        Err(anyhow!("No arg value provided"))
    }
}

/**
 * 对指定参数进行排序
 * 该参数须为Value::Object
 */
pub fn canonicalized_query(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let desc_sorted = args
        .get("order")
        .map(|f| f.as_bool().unwrap_or_default())
        .unwrap_or_default();
    if let Some(arg) = args.get("value") {
        match canonicalized_value(arg, &None, desc_sorted) {
            Ok(v) => Ok(v),
            Err(err) => Err(tera::Error::msg(err.to_string())),
        }
    } else {
        Err(tera::Error::msg("No arg value provided"))
    }
}

/**
 * Get the current datetime, with basic
 */
pub fn current_date() -> Result<Value, tera::Error> {
    Ok(Value::Null)
}

fn to_date(date: &Value, dtformat: Option<String>) -> Result<DateTime<FixedOffset>, tera::Error> {
    let dtvalue = match date {
        Value::String(t) => {
            if let Some(dtfmt) = dtformat {
                if let Ok(parsed) = DateTime::parse_from_str(t, &dtfmt) {
                    parsed
                } else {
                    match NaiveDateTime::parse_from_str(t, &dtfmt)  {
                        Ok(parsed) => {
                            let offset = Local::now().offset().clone();
                            DateTime::from_naive_utc_and_offset(parsed, offset)
                        },
                        Err(err) => {
                            return Err(tera::Error::msg(format!("Unspecial Date/DateTime Value to parsed {err}")));
                        }
                    }
                }
            } else {
                if let Ok(parsed) = DateTime::parse_from_rfc2822(t) {
                    parsed
                } else if let Ok(parsed) = DateTime::parse_from_rfc3339(t) {
                    parsed
                } else if let Ok(parsed) = DateTime::parse_from_str(t, TERA_DATETIME_FORMAT) {
                    parsed
                } else if let Ok(parsed) = DateTime::parse_from_str(t, TERA_DATETIME_BASE_TZ_FORMAT) {
                    parsed
                } else {
                    match DateTime::parse_from_str(t, TERA_DATETIME_BASE_FORMAT) {
                        Ok(parsed) => {
                            parsed
                        },
                        Err(_) => {
                            if let Ok(parsed) = NaiveDateTime::parse_from_str(t, TERA_DATETIME_BASE_FORMAT)  {
                                let offset = Local::now().offset().clone();
                                DateTime::from_naive_utc_and_offset(parsed, offset)
                            } else {
                                match DateTime::parse_from_str(t, TERA_DATE_BASE_FORMAT) {
                                    Ok(parsed) => {
                                        parsed
                                    },
                                    Err(err) => {
                                        if let Ok(parsed) = NaiveDateTime::parse_from_str(t, TERA_DATE_BASE_FORMAT)  {
                                            let offset = Local::now().offset().clone();
                                            DateTime::from_naive_utc_and_offset(parsed, offset)
                                        } else {
                                            return Err(tera::Error::msg(format!("Unspecial Date/DateTime Value to parsed {err}")));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    
                }
            }
        }
        Value::Number(m) => {
            let millis = m.as_i64().unwrap_or_default();
            match DateTime::from_timestamp_millis(millis) {
                Some(opt) => opt.fixed_offset(),
                None => {
                    return Err(tera::Error::msg("Unspecial Date/DateTime Value to convert by millis"));
                }
            }
        }
        _ => {
            return Err(tera::Error::msg("Unspecial Date/DateTime Value by millis"));
        }
    };

    Ok(dtvalue)
}

/**
 * 以当前时间为基础，加指定时间
 */
pub fn date_add(date: &Value, params: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let name = if let Some(Value::String(pname)) = params.get("name") {
        pname.to_lowercase()
    } else {
        "day".to_owned()
    };

    let value = if let Some(Value::Number(pvalue)) = params.get("value") {
        pvalue.as_i64().unwrap_or_default()
    } else {
        0
    };

    let dtformat = if let Some(Value::String(pvalue)) = params.get("format") {
        Some(pvalue.to_string())
    } else {
        None
    };

    let datevalue = to_date(date, dtformat.clone())?;

    let add_sub = value >= 0;
    let uvalue = value.unsigned_abs() as u32;

    let newdate = if add_sub {
        match name.as_str() {
            "day" => datevalue.checked_add_days(Days::new(uvalue as u64)),
            "month" => datevalue.checked_add_months(Months::new(uvalue)),
            "year" => datevalue.checked_add_months(Months::new(uvalue * 12)),
            "hour" => datevalue.checked_add_signed(TimeDelta::hours(value)),
            "minute" => datevalue.checked_add_signed(TimeDelta::minutes(value)),
            "second" => datevalue.checked_add_signed(TimeDelta::seconds(value)),
            _ => Some(datevalue),
        }
    } else {
        match name.as_str() {
            "day" => datevalue.checked_sub_days(Days::new(uvalue as u64)),
            "month" => datevalue.checked_sub_months(Months::new(uvalue)),
            "year" => datevalue.checked_sub_months(Months::new(uvalue * 12)),
            "hour" => datevalue.checked_sub_signed(TimeDelta::hours(value.abs())),
            "minute" => datevalue.checked_sub_signed(TimeDelta::minutes(value.abs())),
            "second" => datevalue.checked_sub_signed(TimeDelta::seconds(value.abs())),
            _ => Some(datevalue),
        }
    };

    let fmt = if let Some(pvalue) = dtformat {
        pvalue
    } else {
        TERA_DATETIME_FORMAT.to_string()
    };

    match newdate {
        Some(dt) => {
            let fmt = dt.format(&fmt);
            Ok(Value::String(fmt.to_string()))
        }
        None => Err(tera::Error::msg("Unrepresent Date/DateTime Value")),
    }
}

/**
 * 获得一个月的最后一天
 */
pub fn month_day(date: &Value, params: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let name = if let Some(Value::String(pname)) = params.get("name") {
        pname.to_lowercase()
    } else {
        "first".to_owned()
    };

    let dtformat = if let Some(Value::String(pvalue)) = params.get("format") {
        Some(pvalue.to_string())
    } else {
        None
    };

    let datevalue = to_date(date, dtformat.clone())?;

    let newdate = match name.as_str() {
        "first" => datevalue.with_day(1),
        "last" => datevalue
            .checked_add_months(Months::new(1))
            .and_then(|ct| ct.with_day(1).map(|xt| xt.checked_sub_days(Days::new(1))))
            .flatten(),
        "middle" => {
            if datevalue.month() == 2 {
                datevalue.with_day(14)
            } else {
                datevalue.with_day(15)
            }
        }
        _ => {
            let st = name.parse::<u32>().unwrap_or_default();
            if (1..=31).contains(&st) {
                datevalue.with_day(st)
            } else {
                None
            }
        }
    };

    let fmt = if let Some(pvalue) = dtformat {
        pvalue
    } else {
        TERA_DATETIME_FORMAT.to_string()
    };    

    match newdate {
        Some(dt) => {
            let fmt = dt.format(&fmt);
            Ok(Value::String(fmt.to_string()))
        }
        None => Err(tera::Error::msg("Unrepresent Date/DateTime Value")),
    }
}
