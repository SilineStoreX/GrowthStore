use std::time::SystemTime;

use chrono::offset::Local;
use chrono::{DateTime, NaiveDateTime};
use serde::{Deserialize, Deserializer};

pub struct ValuePaire {
    value: String,
    key: String,
    timestamp: u64,
    expired: u64,
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

pub fn get_url_encode(c: &str) -> String {
    use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
    const FRAGMENT: &AsciiSet = &CONTROLS
        .add(b' ')
        .add(b'"')
        .add(b'<')
        .add(b'>')
        .add(b'`')
        .add(b'+')
        .add(b'=')
        // .add(b'/')
        ;
    utf8_percent_encode(c, FRAGMENT).to_string()
}

pub fn get_url_encode2(c: &str) -> String {
    urlencoding::encode_binary(c.as_bytes()).into_owned()
}
