use crate::utils::get_local_timestamp;
use std::fmt::Debug;
use salvo::oapi::{schema, Array, Object, RefOr, Schema};
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

    pub fn new(code: i32, msg: &str, data: T) -> Self {
        ApiResult {
            status: code,
            message: msg.to_owned(),
            data: Some(data),
            timestamp: Some(get_local_timestamp()),
        }
    }

    pub fn new_ts(code: i32, msg: &str, data: T, ts: u64) -> Self {
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

    pub fn new(code: i32, msg: &str, data: T) -> Self {
        ApiResult2 {
            status: code,
            message: msg.to_owned(),
            data: Some(data),
            timestamp: Some(get_local_timestamp()),
        }
    }

    pub fn new_ts(code: i32, msg: &str, data: T, ts: u64) -> Self {
        ApiResult2 {
            status: code,
            message: msg.to_owned(),
            data: Some(data),
            timestamp: Some(ts),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CallbackResult<T> {
    pub namesapce: String,
    pub uuid: String,
    pub status: i32,
    pub message: String,
    pub data: Option<T>,
    pub timestamp: Option<u64>,
}

unsafe impl<T> Send for CallbackResult<T> {}

unsafe impl<T> Sync for CallbackResult<T> {}

impl<T> CallbackResult<T> {
    pub fn ok(ns: &str, uuid: &str, dt: T) -> Self {
        CallbackResult {
            namesapce: ns.to_owned(),
            uuid: uuid.to_owned(),
            status: 200,
            message: "OK".to_string(),
            data: Option::Some(dt),
            timestamp: Some(get_local_timestamp()),
        }
    }

    pub fn error(ns: &str, uuid: &str, code: i32, msg: &str) -> Self {
        CallbackResult {
            namesapce: ns.to_owned(),
            uuid: uuid.to_owned(),
            status: code,
            message: msg.to_owned(),
            data: None,
            timestamp: Some(get_local_timestamp()),
        }
    }
}

pub fn to_api_result_schema(t: RefOr<Schema>, array: bool) -> schema::Schema {
    let mut apiresult = Object::new();
    apiresult = apiresult.property(
        "status",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "message",
        Object::new().schema_type(schema::BasicType::String),
    );
    if array {
        apiresult = apiresult.property("data", Array::new().items(t));
    } else {
        apiresult = apiresult.property("data", t);
    }
    apiresult = apiresult.property(
        "timestamp",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    schema::Schema::Object(Box::new(apiresult))
}

pub  fn to_api_result_page_schema(t: RefOr<Schema>) -> schema::Schema {
    let mut apiresult = Object::new();
    apiresult = apiresult.property(
        "total",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "page_no",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "page_size",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "do_count",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property("records", Array::new().items(t));
    to_api_result_schema(
        RefOr::Type(schema::Schema::Object(Box::new(apiresult))),
        false,
    )
}
