use chimes_store_core::utils::{
    global_data::generate_rand_numberstring,
    redis::{
        redis_del, redis_flushall_cmd, redis_get, redis_keys, redis_lock_expire_nx, redis_set,
        redis_set_expire, redis_set_expire_nx,
    },
};
use rhai::EvalAltResult;
use serde_json::{json, Value};

/**
 * function to impl redis_get
 */
pub fn rhai_redis_get(ns: &str, key: &str) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
    match redis_get(ns, key) {
        Ok(t) => Ok(rhai::Dynamic::from(
            t.map(|f| serde_json::from_str::<Value>(&f).unwrap_or(Value::String(f)))
                .unwrap_or(Value::Null),
        )),
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "Unable to get the key".to_string().to_owned(),
            err.into_boxed_dyn_error(),
        ))),
    }
}

/**
 * function to impl redis_set
 */
pub fn rhai_redis_set(
    ns: &str,
    key: &str,
    value: &str,
) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
    match redis_set(ns, key, value) {
        Ok(t) => Ok(rhai::Dynamic::from(t.unwrap_or_default())),
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "Unable to get the key".to_string().to_owned(),
            err.into_boxed_dyn_error(),
        ))),
    }
}

/**
 * function to impl redis_set
 */
pub fn rhai_redis_set_expire(
    ns: &str,
    key: &str,
    value: &str,
    expire: i64,
) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
    match redis_set_expire(ns, key, value, expire as u64) {
        Ok(t) => Ok(rhai::Dynamic::from(t.unwrap_or_default())),
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "Unable to get the key".to_string().to_owned(),
            err.into_boxed_dyn_error(),
        ))),
    }
}

pub fn rhai_redis_set_expire_nx(
    ns: &str,
    key: &str,
    value: &str,
    expire: i64,
    nx: bool,
) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
    match redis_set_expire_nx(ns, key, value, expire as u64, Some(nx)) {
        Ok(t) => Ok(rhai::Dynamic::from(t.unwrap_or_default())),
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "Unable to get the key".to_string().to_owned(),
            err.into_boxed_dyn_error(),
        ))),
    }
}

/**
 * function to impl redis_set
 */
pub fn rhai_redis_lock(
    ns: &str,
    key: &str,
    expire: i64,
) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
    let value = generate_rand_numberstring(10);
    match redis_lock_expire_nx(ns, key, &value, expire as u64, Some(true)) {
        Ok(t) => Ok(rhai::Dynamic::from(t.unwrap_or_default())),
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "Unable to get the key".to_string().to_owned(),
            err.into_boxed_dyn_error(),
        ))),
    }
}

pub fn rhai_redis_unlock(ns: &str, key: &str) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
    rhai_redis_del(ns, key)
}

/**
 * function to impl redis_del
 */
pub fn rhai_redis_del(ns: &str, key: &str) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
    match redis_del(ns, key) {
        Ok(t) => Ok(rhai::Dynamic::from(t.unwrap_or_default())),
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "Unable to get the key".to_string().to_owned(),
            err.into_boxed_dyn_error(),
        ))),
    }
}

/**
 * function to impl redis_del
 */
pub fn rhai_redis_keys(ns: &str, key: &str) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
    match redis_keys(ns, key) {
        Ok(t) => Ok(rhai::Dynamic::from(json!(t))),
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "Unable to get the key".to_string().to_owned(),
            err.into_boxed_dyn_error(),
        ))),
    }
}

/**
 * function to impl redis_del
 */
pub fn rhai_redis_flushall(ns: &str) -> Result<rhai::Dynamic, Box<EvalAltResult>> {
    match redis_flushall_cmd(ns) {
        Ok(t) => Ok(rhai::Dynamic::from(t.unwrap_or_default())),
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "Unable to get the key".to_string().to_owned(),
            err.into_boxed_dyn_error(),
        ))),
    }
}
