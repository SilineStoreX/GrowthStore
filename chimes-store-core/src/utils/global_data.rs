use super::get_local_timestamp;
use super::GlobalConfig;
use base64::display::Base64Display;
use base64::prelude::*;
use chrono::offset::Local;
use chrono::{DateTime, NaiveDateTime};
use crypto::digest::Digest;
use crypto::md5;
use lazy_static::lazy_static;
use rand::thread_rng;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::{PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::SystemTime;

pub struct ValuePaire {
    value: String,
    key: String,
    timestamp: u64,
    expired: u64,
}

lazy_static! {
    pub static ref APP_DATA: Mutex<HashMap<String, ValuePaire>> = Mutex::new(HashMap::new());
}

/**
 * 当这个APP_DATA的大小太大了后，就需要进行resizing操作
 */
pub fn global_app_data_resizing() {
    let dts = APP_DATA.lock().unwrap();
    let len = dts.len();
    let mut keys = vec![];
    if len > 10usize {
        let t = get_local_timestamp();
        let values = dts.values();

        values.for_each(|f| {
            let ts = f.timestamp + f.expired;
            if t >= ts {
                // global_app_data_remove(&f.key);
                keys.push(f.key.clone());
            }
        });
    }
    drop(dts);

    for key in keys {
        global_app_data_remove(&key);
    }
}

#[allow(dead_code)]
pub fn global_app_data_insert(key: &str, val: &str) {
    //let mut hs = APP_DATA.lock().as_mut().unwrap().to_owned();
    //hs.insert(key.to_string(), val.to_string());
    //log::info!("Size Of Hash: {}", hs.len());
    global_app_data_insert_with_expire(key, val, 1000 * 60);
}

#[allow(dead_code)]
pub fn global_app_data_insert_with_expire(key: &str, val: &str, exp: u64) {
    //let mut hs = APP_DATA.lock().as_mut().unwrap().to_owned();
    //hs.insert(key.to_string(), val.to_string());
    //log::info!("Size Of Hash: {}", hs.len());
    let value = ValuePaire {
        value: val.to_owned(),
        key: key.to_owned(),
        timestamp: get_local_timestamp(),
        expired: exp,
    };

    APP_DATA
        .lock()
        .as_mut()
        .unwrap()
        .insert(key.to_string(), value);

    global_app_data_resizing();
}

#[allow(dead_code)]
pub fn global_app_data_remove(key: &String) {
    //let mut hs = APP_DATA.lock().as_mut().unwrap().to_owned();
    //hs.insert(key.to_string(), val.to_string());
    //log::info!("Size Of Hash: {}", hs.len());
    APP_DATA.lock().as_mut().unwrap().remove(key);
}

#[allow(dead_code)]
pub fn global_app_data_get(key: &String) -> Option<String> {
    let dt = APP_DATA.lock().unwrap();
    let cp = dt.get(key);
    if cp.is_none() {
        None
    } else {
        let mpt = cp.unwrap();
        if mpt.expired > 0 {
            let tm = get_local_timestamp();
            if tm > mpt.timestamp + mpt.expired {
                drop(dt);
                global_app_data_remove(key);
                return None;
            }
        }
        Some(cp.unwrap().value.clone())
    }
}

#[allow(dead_code)]
pub fn rsa_decrypt_by_private_key(token: &String) -> Option<String> {
    let private_key = GlobalConfig::get()
        .lock()
        .unwrap()
        .to_owned()
        .config
        .rsa_password_private_key
        .unwrap_or_default();

    let bs = match BASE64_STANDARD.decode(private_key) {
        Ok(rs) => rs,
        Err(_) => {
            vec![]
        }
    };

    let priv_key = match RsaPrivateKey::from_pkcs8_der(&bs) {
        Ok(r) => Some(r),
        Err(err) => {
            log::warn!("Decode the Private Key with an error {}", err);
            None
        }
    };

    match priv_key {
        Some(pkey) => {
            let basedecode = match BASE64_STANDARD.decode(token) {
                Ok(ts) => ts,
                Err(_) => vec![],
            };
            let dcode = pkey.decrypt(PaddingScheme::PKCS1v15Encrypt, &basedecode);
            match dcode {
                Ok(rs) => match String::from_utf8(rs) {
                    Ok(text) => Some(text),
                    Err(err) => {
                        log::warn!("Convert to utf8 with an error {}", err);
                        None
                    }
                },
                Err(err) => {
                    log::warn!("Decode the token with an error {}", err.to_string());
                    None
                }
            }
        }
        None => None,
    }
}

#[allow(dead_code)]
pub fn rsa_encrypt_by_public_key(token: &String) -> Option<String> {
    let public_key = GlobalConfig::get()
        .lock()
        .unwrap()
        .to_owned()
        .config
        .rsa_password_public_key
        .unwrap_or_default();

    let bs = match BASE64_STANDARD.decode(public_key) {
        Ok(rs) => rs,
        Err(_) => {
            vec![]
        }
    };

    let pub_key = match RsaPublicKey::from_public_key_der(&bs) {
        Ok(r) => Some(r),
        Err(err) => {
            log::warn!("Decode the Private Key with an error {}", err);
            None
        }
    };

    match pub_key {
        Some(pkey) => {
            let mut rng = thread_rng();
            let encoded = pkey.encrypt(&mut rng, PaddingScheme::PKCS1v15Encrypt, token.as_bytes());
            match encoded {
                Ok(rs) => {
                    let encodebase = Base64Display::new(&rs, &BASE64_STANDARD).to_string(); // .decode(rs);
                    Some(encodebase)
                }
                Err(err) => {
                    log::warn!("Decode the token with an error {}", err.to_string());
                    None
                }
            }
        }
        None => None,
    }
}

#[allow(dead_code)]
pub fn rsa_decrypt_with_private_key(token: &str, private_key: &str) -> Option<String> {
    let bs = match BASE64_STANDARD.decode(private_key) {
        Ok(rs) => rs,
        Err(_) => {
            vec![]
        }
    };

    let priv_key = match RsaPrivateKey::from_pkcs8_der(&bs) {
        Ok(r) => Some(r),
        Err(err) => {
            log::warn!("Decode the Private Key with an error {}", err);
            None
        }
    };

    match priv_key {
        Some(pkey) => {
            let basedecode = match BASE64_STANDARD.decode(token) {
                Ok(ts) => ts,
                Err(_) => vec![],
            };
            let dcode = pkey.decrypt(PaddingScheme::PKCS1v15Encrypt, &basedecode);
            match dcode {
                Ok(rs) => match String::from_utf8(rs) {
                    Ok(text) => Some(text),
                    Err(err) => {
                        log::warn!("Convert to utf8 with an error {}", err);
                        None
                    }
                },
                Err(err) => {
                    log::warn!("Decode the token with an error {}", err.to_string());
                    None
                }
            }
        }
        None => None,
    }
}

#[allow(dead_code)]
pub fn rsa_encrypt_with_public_key(token: &str, public_key: &str) -> Option<String> {
    let bs = match BASE64_STANDARD.decode(public_key) {
        Ok(rs) => rs,
        Err(_) => {
            vec![]
        }
    };

    let pub_key = match RsaPublicKey::from_public_key_der(&bs) {
        Ok(r) => Some(r),
        Err(err) => {
            log::warn!("Decode the Private Key with an error {}", err);
            None
        }
    };

    match pub_key {
        Some(pkey) => {
            let mut rng = thread_rng();
            log::warn!("Token: {token}");
            let encoded = pkey.encrypt(&mut rng, PaddingScheme::PKCS1v15Encrypt, token.as_bytes());
            match encoded {
                Ok(rs) => {
                    let encodebase = Base64Display::new(&rs, &BASE64_STANDARD).to_string(); // .decode(rs);
                    Some(encodebase)
                }
                Err(err) => {
                    log::warn!("Decode the token with an error {}", err.to_string());
                    None
                }
            }
        }
        None => None,
    }
}

#[derive(Deserialize)]
#[serde(untagged)] // 枚举类型的无标签方式
enum StrOrU64 {
    None,
    String(String),
    U64(u64),
}

#[derive(Deserialize)]
#[serde(untagged)] // 枚举类型的无标签方式
enum StrOrI64 {
    None,
    String(String),
    I64(i64),
}

#[derive(Deserialize)]
#[serde(untagged)] // 枚举类型的无标签方式
enum StrOrF64 {
    None,
    String(String),
    F64(f64),
}

#[derive(Deserialize)]
#[serde(untagged)] // 枚举类型的无标签方式
enum StrOrF32 {
    None,
    String(String),
    F32(f32),
}

#[derive(Deserialize)]
#[serde(untagged)] // 枚举类型的无标签方式
enum StrOrBool {
    String(String),
    I64(i64),
    Bool(bool),
    None,
}

#[allow(dead_code)]
pub fn u64_from_str<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match StrOrU64::deserialize(deserializer)? {
        StrOrU64::String(v) => v.parse().unwrap_or_default(),
        StrOrU64::U64(v) => v,
        StrOrU64::None => 0u64,
    })
}

#[allow(dead_code)]
pub fn i64_from_str<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match StrOrI64::deserialize(deserializer)? {
        StrOrI64::String(v) => match v.parse::<i64>() {
            Ok(st) => Some(st),
            Err(_) => None,
        },
        StrOrI64::I64(v) => Some(v),
        StrOrI64::None => None,
    })
}

#[allow(dead_code)]
pub fn i32_from_str<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match StrOrI64::deserialize(deserializer)? {
        StrOrI64::String(v) => match v.parse::<i64>() {
            Ok(st) => Some(st as i32),
            Err(_) => None,
        },
        StrOrI64::I64(v) => Some(v as i32),
        StrOrI64::None => None,
    })
}

#[allow(dead_code)]
pub fn f64_from_str<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match StrOrF64::deserialize(deserializer)? {
        StrOrF64::String(v) => Some(v.parse().unwrap_or_default()),
        StrOrF64::F64(v) => Some(v),
        StrOrF64::None => None,
    })
}

#[allow(dead_code)]
pub fn f32_from_str<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match StrOrF32::deserialize(deserializer)? {
        StrOrF32::String(v) => v.parse().unwrap_or_default(),
        StrOrF32::F32(v) => v,
        StrOrF32::None => 0.0f32,
    })
}

#[allow(dead_code)]
pub fn bool_from_str<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match StrOrBool::deserialize(deserializer) {
        Ok(t) => match t {
            StrOrBool::String(v) => match v.parse::<bool>() {
                Ok(tf) => Some(tf),
                Err(err) => {
                    log::warn!("Parse erroor {}", err);
                    None
                }
            },
            StrOrBool::I64(v) => Some(v != 0i64),
            StrOrBool::Bool(v) => Some(v),
            StrOrBool::None => Some(false),
        },
        Err(err) => {
            log::warn!("Deserializer erroor {}", err);
            None
        }
    })
}

pub fn num_to_string(n: i64) -> String {
    let base_codec = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
        'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '2', '3', '4', '5', '7', '8', '9',
    ];
    let len = base_codec.len() as i64;
    let mut t = n;
    let mut result = "".to_string();
    while t > 0 {
        let idx = (t % len) as usize;
        let ch = base_codec[idx];
        t /= len;
        result.insert(0, ch);
    }
    result
}

pub fn number_to_string(n: i64) -> String {
    let base_codec = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'];
    let len = base_codec.len() as i64;
    let mut t = n;
    let mut result = "".to_string();
    while t > 0 {
        let idx = (t % len) as usize;
        let ch = base_codec[idx];
        t /= len;
        result.insert(0, ch);
    }
    result
}

pub fn generate_rand_string(len: usize) -> String {
    let mut retkey = "".to_string();

    while retkey.len() < len {
        let rng = rand::random::<u16>();
        let key = num_to_string(rng as i64);
        retkey += key.as_str();
    }

    retkey.chars().take(len).collect()
}

pub fn generate_rand_numberstring(len: usize) -> String {
    let mut retkey = "".to_string();

    while retkey.len() < len {
        let rng = rand::random::<u16>();
        let key = number_to_string(rng as i64);
        retkey += key.as_str();
    }

    retkey.chars().take(len).collect()
}

#[allow(dead_code)]
pub fn num_to_string_v2(n: i64) -> String {
    let base_codec = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'j', 'k', 'l', 'm', 'n', 'p', 'q', 'r', 's', 't',
        'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M',
        'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '2', '3', '4', '5', '6', '7',
        '8', '9',
    ];
    let len = base_codec.len() as i64;
    let mut t = n;
    let mut result = "".to_string();
    while t > 0 {
        let idx = (t % len) as usize;
        let ch = base_codec[idx];
        t /= len;
        result.insert(0, ch);
    }
    result
}

#[allow(dead_code)]
pub fn generate_rand_string_v2(len: usize) -> String {
    let mut retkey = "".to_string();

    while retkey.len() < len {
        let rng = rand::random::<u16>();
        let key = num_to_string_v2(rng as i64);
        retkey += key.as_str();
    }

    retkey.chars().take(len).collect()
}

#[allow(dead_code)]
pub fn get_date_string() -> String {
    let now = SystemTime::now();
    let date: DateTime<Local> = now.into();
    let fmt = format!("{}", date.format("%Y%m%d"));
    fmt
}

#[allow(dead_code)]
pub fn format_date_string(dt: &NaiveDateTime) -> String {
    let fmt = format!("{}", dt.format("%Y年%m月%d日 %H:%M"));
    fmt
}

pub fn get_url_encode2(c: &str) -> String {
    urlencoding::encode_binary(c.as_bytes()).into_owned()
}

/**
 * 从源JSON对象中产生一个新的JSON对象，但排除掉excludes中指定的字段
 * ** 只处理一层
 */
pub fn copy_value_excluded(src_val: &Value, excludes: &[String]) -> Value {
    let mut des_val = Map::new();
    for (key, val) in src_val.as_object().unwrap() {
        if !excludes.contains(key) {
            des_val.insert(key.to_owned(), val.to_owned());
        }
    }
    Value::Object(des_val)
}

/**
 * 从源JSON对象中产生一个新的JSON对象，但只添加指定includes的字段
 * ** 只处理一层
 */
pub fn copy_value_included(src_val: &Value, includes: &[String]) -> Value {
    let mut des_val = Map::new();
    for key in includes {
        if let Some(val) = src_val.get(key) {
            des_val.insert(key.to_owned(), val.to_owned());
        } else {
            des_val.insert(key.to_owned(), Value::Null);
        }
    }
    Value::Object(des_val)
}

/**
 * 从源JSON对象中产生一个新的JSON对象，并使用replace中的对应的字段的值对源值进行替换
 * ** 只处理一层
 */
pub fn copy_value_replaced(src_val: &Value, replace: &Value) -> Value {
    let mut des_val = Map::new();
    if let Some(opmap) = replace.as_object() {
        for (key, val) in src_val.as_object().unwrap() {
            if opmap.contains_key(key) {
                if let Some(opval) = opmap.get(key) {
                    des_val.insert(key.to_owned(), opval.to_owned());
                } else {
                    des_val.insert(key.to_owned(), Value::Null);
                }
            } else {
                des_val.insert(key.to_owned(), val.to_owned());
            }
        }
    }
    Value::Object(des_val)
}

/**
 * 使用replace对src_val进行替换
 * 如果具体的字段内容相同，则根据remove_equal来确定是否删除
 * 同时，如果replace不存在的字段，则不进行比较
 */
pub fn copy_value_compared_replaced(
    src_val: &Value,
    replace: &Value,
    remove_equal: bool,
    keep: &[String],
) -> Value {
    let mut des_val = Map::new();
    if let Some(opmap) = replace.as_object() {
        for (key, val) in src_val.as_object().unwrap() {
            if opmap.contains_key(key) {
                if let Some(opval) = opmap.get(key) {
                    if *opval == *val {
                        if !remove_equal || keep.contains(key) {
                            des_val.insert(key.to_owned(), opval.to_owned());
                        }
                    } else {
                        des_val.insert(key.to_owned(), opval.to_owned());
                    }
                } else {
                    des_val.insert(key.to_owned(), Value::Null);
                }
            }
        }
    }
    Value::Object(des_val)
}


/**
 * 对字符串执行MD5
 */
pub fn md5text(text: &str) -> String {
    let mut md5 = md5::Md5::new();
    md5.input_str(text);
    md5.result_str()
}
