use std::str::FromStr;

use anyhow::anyhow;
use chimes_store_core::{
    dbs::{proxy_get_namesapce_config, proxy_get_plugin_config}, service::{
        queue::SyncTaskQueue,
        sched::{convert_into_job_invoker, SchedulerHolder},
    }, utils::{
        algorithm::{md5_hash, sha1_256_hash, sha2_256_hash, snowflake_id, snowflake_id_custom},
        crypto::{hmac_sha1, hmac_sha256, hmac_sha512},
        global_data::generate_rand_string_v2,
        redis::redis_get,
    }
};
use rbatis::rbdc::Uuid;
use rhai::{Blob, EvalAltResult};
use serde_json::Value;

use crate::engine::resolver::json_path_get;

/**
 * function to impl hmac_sha1
 */
pub fn rand_text(len: i64) -> rhai::Dynamic {
    let ret = generate_rand_string_v2(len as usize);
    rhai::Dynamic::from(ret)
}

/**
 * function to impl hmac_sha1
 */
pub fn sha1_text(data: &str) -> rhai::Dynamic {
    let ret = sha1_256_hash(&data.as_bytes());
    rhai::Dynamic::from(ret)
}

/**
 * function to impl hmac_sha1
 */
pub fn text_md5(data: &str) -> rhai::Dynamic {
    let ret = md5_hash(&data.as_bytes());
    rhai::Dynamic::from(ret)
}

/**
 * function to impl hmac_sha1
 */
pub fn sha2_text(data: &str) -> rhai::Dynamic {
    let ret = sha2_256_hash(&data.as_bytes());
    rhai::Dynamic::from(ret)
}

/**
 * function to impl base64_encode
 */
pub fn text_base32_encode(data: &str) -> rhai::Dynamic {
    let ret = chimes_store_core::utils::algorithm::base32_encode(&data.as_bytes());
    rhai::Dynamic::from(ret)
}

/**
 * function to impl base64_decode
 */
pub fn text_base32_decode(uri: &str) -> rhai::Dynamic {
    let decode_data = chimes_store_core::utils::algorithm::base32_decode(uri);
    if let Ok(ret) = String::from_utf8(decode_data) {
        rhai::Dynamic::from(ret)
    } else {
        rhai::Dynamic::from(String::new())
    }
}

/**
 * function to impl base64_encode
 */
pub fn text_base64_encode(data: &str) -> rhai::Dynamic {
    let ret = chimes_store_core::utils::algorithm::base64_encode(&data.as_bytes());
    rhai::Dynamic::from(ret)
}

/**
 * function to impl base64_decode
 */
pub fn text_base64_decode(uri: &str) -> rhai::Dynamic {
    let decode_data = chimes_store_core::utils::algorithm::base64_decode(&uri.as_bytes());
    if let Ok(ret) = String::from_utf8(decode_data) {
        rhai::Dynamic::from(ret)
    } else {
        rhai::Dynamic::from(String::new())
    }
}

/**
 * function to impl base64_encode
 */
pub fn text_base64_encode_urlsafe(data: &str, urlsafe: bool) -> rhai::Dynamic {
    let ret = chimes_store_core::utils::algorithm::base64_encode_urlsafe(&data.as_bytes(), urlsafe);
    rhai::Dynamic::from(ret)
}

/**
 * function to impl base64_decode
 */
pub fn text_base64_decode_urlsafe(uri: &str, urlsafe: bool) -> rhai::Dynamic {
    let decode_data =
        chimes_store_core::utils::algorithm::base64_decode_urlsafe(&uri.as_bytes(), urlsafe);
    if let Ok(ret) = String::from_utf8(decode_data) {
        rhai::Dynamic::from(ret)
    } else {
        rhai::Dynamic::from(String::new())
    }
}

/**
 * function to impl base64_encode
 */
pub fn file_base64_encode_urlsafe(filename: &str, urlsafe: bool) -> rhai::Dynamic {
    match std::fs::read(filename) {
        Ok(ts) => {
            let ret = chimes_store_core::utils::algorithm::base64_encode_urlsafe(&ts, urlsafe);
            rhai::Dynamic::from(ret)
        }
        Err(err) => {
            log::warn!("read file error {err}");
            rhai::Dynamic::from_bool(false)
        }
    }
}

/**
 * function to impl base64_decode
 */
pub fn file_base64_decode_urlsafe(filename: &str, urlsafe: bool) -> rhai::Dynamic {
    match std::fs::read(filename) {
        Ok(ts) => {
            let ret = chimes_store_core::utils::algorithm::base64_decode_urlsafe(&ts, urlsafe);
            rhai::Dynamic::from(ret)
        }
        Err(err) => {
            log::warn!("read file error {err}");
            rhai::Dynamic::from_bool(false)
        }
    }
}

pub fn read_file_blob(filename: &str) -> rhai::Dynamic {
    match std::fs::read(filename) {
        Ok(ts) => rhai::Dynamic::from_blob(ts),
        Err(err) => {
            log::warn!("read file error {err}");
            rhai::Dynamic::from_bool(false)
        }
    }
}

pub fn read_file_string(filename: &str) -> rhai::Dynamic {
    match std::fs::read_to_string(filename) {
        Ok(ts) => rhai::Dynamic::from(ts),
        Err(err) => {
            log::warn!("write file error {err}");
            rhai::Dynamic::from_bool(false)
        }
    }
}

pub fn write_file_blob(filename: &str, data: Blob) -> rhai::Dynamic {
    match std::fs::write(filename, &data) {
        Ok(_) => rhai::Dynamic::from_bool(true),
        Err(err) => {
            log::warn!("write file error {err}");
            rhai::Dynamic::from_bool(false)
        }
    }
}

pub fn write_file_string(filename: &str, data: &str) -> rhai::Dynamic {
    match std::fs::write(filename, data) {
        Ok(_) => rhai::Dynamic::from_bool(true),
        Err(err) => {
            log::warn!("read file error {err}");
            rhai::Dynamic::from_bool(false)
        }
    }
}

/**
 * function to impl to_hex
 */
pub fn blob_to_hex(data: Blob) -> rhai::Dynamic {
    let ret = chimes_store_core::utils::algorithm::to_hex(&data.to_vec());
    rhai::Dynamic::from(ret)
}

/**
 * function to impl base64_encode
 */
pub fn blob_base64_encode(data: Blob) -> rhai::Dynamic {
    let ret = chimes_store_core::utils::algorithm::base64_encode(&data.to_vec());
    rhai::Dynamic::from(ret)
}

/**
 * function to impl base64_decode
 */
pub fn blob_base64_decode(data: Blob) -> rhai::Dynamic {
    let decode_data = chimes_store_core::utils::algorithm::base64_decode(&data.to_vec());
    if let Ok(ret) = String::from_utf8(decode_data) {
        rhai::Dynamic::from(ret)
    } else {
        rhai::Dynamic::from(String::new())
    }
}

/**
 * function to impl base64_encode
 */
pub fn blob_base64_encode_urlsafe(data: Blob, urlsafe: bool) -> rhai::Dynamic {
    let ret = chimes_store_core::utils::algorithm::base64_encode_urlsafe(&data.to_vec(), urlsafe);
    rhai::Dynamic::from(ret)
}

/**
 * function to impl base64_decode
 */
pub fn blob_base64_decode_urlsafe(data: Blob, urlsafe: bool) -> rhai::Dynamic {
    let decode_data =
        chimes_store_core::utils::algorithm::base64_decode_urlsafe(&data.to_vec(), urlsafe);
    if let Ok(ret) = String::from_utf8(decode_data) {
        rhai::Dynamic::from(ret)
    } else {
        rhai::Dynamic::from(String::new())
    }
}

pub fn hmac_sha1_rhai(key: &str, data: &str) -> rhai::Dynamic {
    let codec = hmac_sha1(key, data);
    rhai::Dynamic::from_blob(codec)
}

pub fn hmac_sha256_rhai(key: &str, data: &str) -> rhai::Dynamic {
    let codec = hmac_sha256(key, data);
    rhai::Dynamic::from_blob(codec)
}

pub fn hmac_sha512_rhai(key: &str, data: &str) -> rhai::Dynamic {
    let codec = hmac_sha512(key, data);
    rhai::Dynamic::from_blob(codec)
}

pub fn rhai_snowflake_id() -> rhai::Dynamic {
    let id = snowflake_id();
    rhai::Dynamic::from_int(id)
}

pub fn rhai_snowflake_id_custom(machine_id: i32, node_id: i32, mode: i32) -> rhai::Dynamic {
    let id = snowflake_id_custom(machine_id, node_id, mode);
    rhai::Dynamic::from_int(id)
}

pub fn rhai_uuid() -> rhai::Dynamic {
    let uuid = Uuid::new().to_string().to_ascii_lowercase();
    match rhai::Dynamic::from_str(&uuid) {
        Ok(t) => t,
        Err(_err) => {
            log::debug!("unhand to generate Dynamic from {uuid}");
            rhai::Dynamic::default()
        }
    }
}

pub fn url_encode(val: &str) -> rhai::Dynamic {
    let v = urlencoding::encode(val);
    match rhai::Dynamic::from_str(&v) {
        Ok(t) => t,
        Err(err) => {
            log::debug!("unhand to generate Dynamic from {err:?}");
            rhai::Dynamic::default()
        }
    }
}

pub fn url_decode(val: &str) -> rhai::Dynamic {
    match urlencoding::decode(val) {
        Ok(st) => match rhai::Dynamic::from_str(&st) {
            Ok(t) => t,
            Err(err) => {
                log::debug!("unhand to generate Dynamic by error {err:?}");
                rhai::Dynamic::default()
            }
        },
        Err(err) => {
            log::debug!("unhand to decode url component with error {err:?}");
            rhai::Dynamic::default()
        }
    }
}

pub fn rhai_parse_xml(val: &str) -> rhai::Map {
    match chimes_store_utils::common::parse_xml(val) {
        Ok(value) => match serde_json::from_value::<rhai::Map>(value) {
            Ok(m) => m,
            Err(err) => {
                log::debug!("Could not converto rhai map, {err:?}");
                rhai::Map::new()
            }
        },
        Err(err) => {
            log::warn!("could not parse xml to value {err}");
            rhai::Map::new()
        }
    }
}

pub fn rhai_get_namesapce_config(ns: &str) -> rhai::Dynamic {
    let val = proxy_get_namesapce_config(ns).unwrap_or(Value::Null);
    match serde_json::from_value::<rhai::Map>(val) {
        Ok(m) => rhai::Dynamic::from_map(m),
        Err(err) => {
            log::debug!("Could not converto rhai_object, {err:?}");
            rhai::Dynamic::from_map(rhai::Map::new())
        }
    }
}

pub fn rhai_get_plugin_config(nsuri: &str) -> rhai::Dynamic {
    let val = proxy_get_plugin_config(nsuri).unwrap_or(Value::Null);
    match serde_json::from_value::<rhai::Map>(val) {
        Ok(m) => rhai::Dynamic::from_map(m),
        Err(err) => {
            log::debug!("Could not converto rhai_object, {err:?}");
            rhai::Dynamic::from_map(rhai::Map::new())
        }
    }
}

pub fn queue_addtask(task_id: &str, task_info: Value, state: i64) {
    let taskname = json_path_get(&task_info, "$.name")
        .map(|t| match t {
            Value::String(text) => Some(text),
            _ => serde_json::to_string(&t)
                .map(|s| Some(s.to_owned()))
                .unwrap_or(None),
        })
        .unwrap_or(None);

    let subject = json_path_get(&task_info, "$.subject")
        .map(|t| match t {
            Value::String(text) => Some(text),
            _ => serde_json::to_string(&t)
                .map(|s| Some(s.to_owned()))
                .unwrap_or(None),
        })
        .unwrap_or(None);

    if let Err(err) =
        SyncTaskQueue::get_mut().push_task(task_id, &taskname, &subject, &task_info, state)
    {
        log::error!("Add task into {task_id} failed with error {err}");
    }
}

pub fn schedule_add_oneshot_task(
    task: Value,
    delay: i64,
) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let (jobid, job) = match convert_into_job_invoker(task) {
        Ok(tjob) => tjob,
        Err(err) => {
            return Err(Box::new(EvalAltResult::ErrorSystem(
                "Unable to convert JSON object into oneshot task definition."
                    .to_string()
                    .to_owned(),
                err.into_boxed_dyn_error(),
            )));
        }
    };

    SchedulerHolder::get().addjob_delay(&jobid, "", None, Some(delay as u64), job);

    Ok(rhai::Dynamic::from(jobid))
}

pub fn schedule_oneshot_query(ns: &str, jobid: &str) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    if ns.trim().is_empty() || jobid.trim().is_ascii() {
        let err = anyhow!("No enough parameter provided.");
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Unable to query oneshot task result."
                .to_string()
                .to_owned(),
            err.into_boxed_dyn_error(),
        )));
    }

    let key = format!("{ns}-{jobid}");
    match redis_get(ns, &key) {
        Ok(res) => match res {
            Some(rt) => match serde_json::from_str::<Value>(&rt) {
                Ok(val) => Ok(rhai::Dynamic::from(val)),
                Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Unable to query oneshot task result by serde_json error."
                        .to_string()
                        .to_owned(),
                    Box::new(err),
                ))),
            },
            None => Ok(rhai::Dynamic::from(Value::Null)),
        },
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "Unable to query oneshot task result by redis_get."
                .to_string()
                .to_owned(),
            err.into_boxed_dyn_error(),
        ))),
    }
}
