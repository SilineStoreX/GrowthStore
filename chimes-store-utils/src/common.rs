use std::{cmp::Ordering, io::Read};

use anyhow::anyhow;
use chrono::{DateTime, NaiveDateTime};
use itertools::Itertools;
use salvo::http::ParseError;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use substring::Substring;

pub fn option_value_to_vec_value(ret: &Option<Value>) -> Vec<Value> {
    match ret {
        None => vec![],
        Some(tt) => {
            if tt.is_array() {
                if let Some(tvec) = tt.as_array() {
                    tvec.to_owned()
                } else {
                    vec![]
                }
            } else {
                vec![tt.to_owned()]
            }
        }
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn generate_rand_string(len: usize) -> String {
    let mut retkey = "".to_string();

    while retkey.len() < len {
        let rng = rand::random::<u16>();
        let key = num_to_string(rng as i64);
        retkey += key.as_str();
    }

    retkey.chars().take(len).collect()
}

#[allow(dead_code)]
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

pub fn to_xml_response<S: Serialize>(data: &S) -> Result<String, anyhow::Error> {
    serde_xml_rs::to_string(data).map_err(|o| anyhow!("error for to_string {o}"))
}

pub fn parse_xml(data: &str) -> Result<Value, anyhow::Error> {
    serde_xml_rs::from_str::<Value>(data).map_err(|e| anyhow!("error to convert to json value {e}"))
}

#[allow(dead_code)]
pub async fn parse_request_body<'de, T>(
    req: &'de mut salvo::Request,
    max_size: usize,
) -> Result<T, anyhow::Error>
where
    T: DeserializeOwned,
{
    match req.content_type() {
        Some(ctype) => {
            tracing::info!("Request Content-Type: {ctype}");
            if ctype.subtype() == salvo::http::mime::WWW_FORM_URLENCODED
                || ctype.subtype() == salvo::http::mime::FORM_DATA
                || ctype.subtype() == salvo::http::mime::JSON
            {
                req.parse_body_with_max_size::<T>(max_size)
                    .await
                    .map_err(|e| anyhow!("parse error {e}"))
            } else if ctype.subtype() == salvo::http::mime::XML {
                req.payload_with_max_size(max_size)
                    .await
                    .and_then(|payload| {
                        // fix issue https://github.com/salvo-rs/salvo/issues/545
                        let payload = if payload.is_empty() {
                            "<empty />".as_bytes()
                        } else {
                            payload.as_ref()
                        };
                        serde_xml_rs::from_reader(payload).map_err(|_| ParseError::ParseFromStr)
                    })
                    .map_err(|o| anyhow!("parse error {o}"))
            } else if ctype.subtype() == salvo::http::mime::PLAIN
                || ctype.subtype() == salvo::http::mime::TEXT
            {
                req.payload_with_max_size(max_size)
                    .await
                    .and_then(|payload| {
                        // fix issue https://github.com/salvo-rs/salvo/issues/545
                        let mut payload = if payload.is_empty() {
                            "<empty />".as_bytes()
                        } else {
                            payload.as_ref()
                        };
                        let mut text = String::new();
                        match payload.read_to_string(&mut text) {
                            Ok(_) => {
                                let val = if text.starts_with("aes+xml:") {
                                    json!({"encbody": text.substring("aes+xml:".len(), text.len()), "format": "xml", "algorithm": "aes" })
                                } else if text.starts_with("aes:") {
                                    json!({"encbody": text.substring("aes:".len(), text.len()), "format": "json", "algorithm": "aes" })
                                } else if text.starts_with("rsa:") {
                                    json!({"encbody": text.substring("rsa:".len(), text.len()), "format": "json", "algorithm": "rsa" })
                                } else if text.starts_with("rsa+xml:") {
                                    json!({"encbody": text.substring("rsa+xml:".len(), text.len()), "format": "xml", "algorithm": "rsa" })
                                } else {
                                    json!({"encbody": text, "format": "json", "algorithm": "aes" })
                                };

                                // let valbytes = serde_json::to_string(&val).unwrap_or_default();

                                serde_json::from_value(val).map_err(|_| ParseError::ParseFromStr)
                                // serde_json::from_str::<T>(&valbytes).map_err(|_| ParseError::ParseFromStr)
                            },
                            Err(_err) => {
                                Err(ParseError::ParseFromStr)
                            }
                        }
                    })
                    .map_err(|o| anyhow!("parse error {o}"))
            } else {
                Err(anyhow!("Unknow Content-Type"))
            }
        }
        None => Err(anyhow!("Unknow Content-Type")),
    }
}

pub fn is_number(ts: &str) -> bool {
    if ts.parse::<f64>().is_err() && ts.parse::<i64>().is_err() {
        return false;
    }
    true
}

pub fn datetime_from_timestamp(ts: i64) -> Result<NaiveDateTime, anyhow::Error> {
    match DateTime::from_timestamp_millis(ts) {
        Some(dt) => Ok(dt.naive_local()),
        None => Err(anyhow!("could not convert to DateTime")),
    }
}

pub fn parse_datetime(ts: &str) -> Result<NaiveDateTime, anyhow::Error> {
    let fmts: Vec<&str> = vec![
        "%Y %b %d %H:%M:%S%.3f %z",
        "%Y-%m-%d %H:%M:%S,%z",
        "%Y-%m-%d %H:%M:%S %z",
        "%Y-%m-%d %H:%M:%S%z",
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%Y-%m-%d %H:%M:%S",
    ];
    let normz = ts.trim().replace("T", " ");
    for fmt in fmts {
        if let Ok(dt) = NaiveDateTime::parse_from_str(&normz, fmt) {
            return Ok(dt);
        }
    }

    Err(anyhow!("Unknown format"))
}

pub fn fmt_datetime(dt: &NaiveDateTime) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/**
 * 将文字之间的空格移除
 */
pub fn text_intern_trim(txt: &str) -> String {
    txt.lines()
        .map(|l| {
            let spi = l.split_whitespace().collect::<Vec<_>>();
            let mut rets = vec![];

            match spi.len().cmp(&1) {
                Ordering::Greater => {
                    for i in 0..(spi.len() - 1) {
                        if spi[i].is_ascii() && spi[i + 1].is_ascii() {
                            rets.push(spi[i]);
                            rets.push(" ");
                        } else {
                            rets.push(spi[i]);
                        }

                        if i == spi.len() - 2 {
                            // last one
                            rets.push(spi[i + 1]);
                        }
                    }
                }
                Ordering::Equal => {
                    rets.push(spi[0]);
                }
                _ => {}
            }

            rets.join("")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/**
 * 根据里JSON对象中的Key进行排序
 */
pub fn json_object_sorting(val: serde_json::Value, desc: bool, _deepin: bool) -> serde_json::Value {
    match val {
        serde_json::Value::Object(h) => {
            let newval = h
                .iter()
                .sorted_by(|a, b| if desc { b.0.cmp(a.0) } else { a.0.cmp(b.0) })
                .map(|f| {
                    let sortedval = json_object_sorting(f.1.to_owned(), desc, _deepin);
                    (f.0.to_owned(), sortedval)
                })
                .collect::<serde_json::Map<String, serde_json::Value>>();
            Value::Object(newval)
        }
        serde_json::Value::Array(tv) => {
            let sorted = tv
                .iter()
                .map(|f| json_object_sorting(f.to_owned(), desc, _deepin))
                .sorted_by(|a, b| cmp_json_value(a.to_owned(), b.to_owned()))
                .collect::<Vec<Value>>();
            serde_json::Value::Array(sorted)
        }
        _ => val,
    }
}

fn cmp_json_value(a: serde_json::Value, b: serde_json::Value) -> Ordering {
    match a {
        Value::String(tt) => match b {
            Value::String(xt) => tt.cmp(&xt),
            _ => Ordering::Less,
        },
        Value::Number(mm) => match b {
            Value::Number(xt) => {
                if mm.is_f64() {
                    mm.as_f64()
                        .partial_cmp(&xt.as_f64())
                        .unwrap_or(Ordering::Less)
                } else if mm.is_i64() {
                    mm.as_i64().cmp(&xt.as_i64())
                } else if mm.is_u64() {
                    mm.as_u64().cmp(&xt.as_u64())
                } else {
                    Ordering::Less
                }
            }
            _ => Ordering::Less,
        },
        Value::Bool(bl) => match b {
            Value::Bool(mb) => bl.cmp(&mb),
            _ => Ordering::Less,
        },
        Value::Null => match b {
            Value::Null => Ordering::Equal,
            _ => Ordering::Less,
        },
        _ => Ordering::Less,
    }
}


pub fn copy_to_slice<T: Copy>(to: &mut [T], source: &[T]) {
    let to_len = to.len();
    to[..(to_len.min(source.len()))].copy_from_slice(&source[..(to_len.min(source.len()))]);
}


#[test]
pub fn test_json_sort() {
    let json = json!({
        "x": ["a", "10", "b", "z", "y"],
        "b": "start",
        "p": {
            "d": "aff",
            "b": "x",
            "z": "c",
            "h": "loop"
        },
        "h": [1,38, 23, 22, 10],
        "ef": "nortmal",
    });

    println!(
        "JSON-Unsort: {}",
        serde_json::to_string(&json).unwrap_or_default()
    );

    let newj = json_object_sorting(json, true, true);
    println!("JSON: {}", serde_json::to_string(&newj).unwrap_or_default());
}
