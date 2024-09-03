use crate::utils::get_local_timestamp;
use std::fmt::Debug;
// use salvo::oapi::{ToResponse, ToSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiResult<T> {
    pub status: i32,
    pub message: String,
    pub data: Option<T>,
    pub timestamp: Option<u64>,
}

unsafe impl<T> Send for ApiResult<T> {}

unsafe impl<T> Sync for ApiResult<T> {}

impl<T> ApiResult<T> {
    pub fn ok(dt: T) -> Self {
        ApiResult {
            status: 200,
            message: "OK".to_string(),
            data: Option::Some(dt),
            timestamp: Some(get_local_timestamp()),
        }
    }

    pub fn error(code: i32, msg: &str) -> Self {
        ApiResult {
            status: code,
            message: msg.to_owned(),
            data: None,
            timestamp: Some(get_local_timestamp()),
        }
    }

    pub fn new(code: i32, msg: &str, data: T, ts: u64) -> Self {
        ApiResult {
            status: code,
            message: msg.to_owned(),
            data: Some(data),
            timestamp: Some(ts),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiResult2<T> {
    pub status: i32,
    pub message: String,
    pub data: Option<T>,
    pub timestamp: Option<u64>,
}

unsafe impl<T> Send for ApiResult2<T> {}

unsafe impl<T> Sync for ApiResult2<T> {}

impl<T> ApiResult2<T> {
    pub fn ok(dt: T) -> Self {
        ApiResult2 {
            status: 200,
            message: "OK".to_string(),
            data: Option::Some(dt),
            timestamp: Some(get_local_timestamp()),
        }
    }

    pub fn error(code: i32, msg: &str) -> Self {
        ApiResult2 {
            status: code,
            message: msg.to_owned(),
            data: None,
            timestamp: Some(get_local_timestamp()),
        }
    }

    pub fn new(code: i32, msg: &str, data: T, ts: u64) -> Self {
        ApiResult2 {
            status: code,
            message: msg.to_owned(),
            data: Some(data),
            timestamp: Some(ts),
        }
    }
}
