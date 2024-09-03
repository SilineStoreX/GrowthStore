use std::mem::MaybeUninit;
use std::sync::{Mutex, Once};
use std::time::SystemTime;

use change_case::{camel_case, pascal_case, snake_case};
use chimes_store_core::dbs::probe::ColumnInfo;
use chimes_store_core::utils::get_local_timestamp;
use chrono::{DateTime, Local, NaiveDateTime};
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};

use crate::config::WebConfig;

pub mod zip;
pub mod change_case;

#[cfg(windows)]
mod windows_performance;

#[cfg(not(target_os = "windows"))]
mod linux_performance;

mod performance;
pub use performance::*;

pub struct AppConfig {
    web_config: WebConfig,
}

impl AppConfig {
    pub fn get() -> &'static Mutex<AppConfig> {
        // 使用MaybeUninit延迟初始化
        static mut CONF: MaybeUninit<Mutex<AppConfig>> = MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static ONCE: Once = Once::new();

        ONCE.call_once(|| unsafe {
            CONF.as_mut_ptr().write(Mutex::new(AppConfig {
                web_config: WebConfig::default(),
            }));
        });
        unsafe { &*CONF.as_ptr() }
    }

    #[allow(dead_code)]
    pub fn update(webc: &WebConfig) {
        Self::get().lock().unwrap().web_config = webc.clone();
    }

    #[allow(dead_code)]
    pub fn config() -> WebConfig {
        Self::get().lock().unwrap().web_config.clone()
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

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ManageApiResult<T: ToSchema + 'static> {
    pub status: i32,
    pub message: String,
    pub data: Option<T>,
    pub timestamp: Option<u64>,
}

unsafe impl<T: ToSchema> Send for ManageApiResult<T> {}

unsafe impl<T: ToSchema> Sync for ManageApiResult<T> {}

impl<T: ToSchema> ManageApiResult<T> {
    pub fn ok(dt: T) -> Self {
        ManageApiResult {
            status: 200,
            message: "OK".to_string(),
            data: Option::Some(dt),
            timestamp: Some(get_local_timestamp()),
        }
    }

    pub fn error(code: i32, msg: &str) -> Self {
        ManageApiResult {
            status: code,
            message: msg.to_owned(),
            data: None,
            timestamp: Some(get_local_timestamp()),
        }
    }

    #[allow(dead_code)]
    pub fn new(code: i32, msg: &str, data: T, ts: u64) -> Self {
        ManageApiResult {
            status: code,
            message: msg.to_owned(),
            data: Some(data),
            timestamp: Some(ts),
        }
    }
}

pub fn mixup_case(_name: &str) -> String {
    let cp = generate_rand_string_v2(8);
    let chs = cp.as_bytes();
    if chs[0] < b'a' || chs[0] > b'z' {
        format!("p{}_", cp.to_lowercase())
    } else {
        format!("{}_", cp.to_lowercase())
    }
}

pub fn naming_property(col: &ColumnInfo, rule: &str) -> Option<String> {
    if let Some(col_name) = col.column_name.clone() {
        match rule {
            "camelcase" => {
                Some(camel_case(&col_name.to_lowercase()))
            },
            "snakecase" => {
                Some(snake_case(&col_name.to_lowercase()))
            },
            "pascalcase" => {
                Some(pascal_case(&col_name.to_lowercase()))
            },
            "mixup" => {
                Some(mixup_case(&col_name))
            },
            _ => {
                Some(col_name)
            }
        }
    } else {
        None
    }
}