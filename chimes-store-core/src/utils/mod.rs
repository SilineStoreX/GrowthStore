use anyhow::bail;
use chimes_dbs_factory::get_sql_driver;
use chrono::{DateTime, Local};
use rbatis::RBatis;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once};
use std::time::SystemTime;

pub mod algorithm;
pub mod crypto;
pub mod executor;
pub mod global_data;
pub mod redis;
pub mod response;
pub use response::*;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalSecurityConfig {
    pub console_code_page: Option<String>,
    pub rsa_password_public_key: Option<String>,
    pub rsa_password_private_key: Option<String>,
    pub work_threads: usize,
    pub pool_size: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    config: GlobalSecurityConfig,
}

impl GlobalConfig {
    pub fn get() -> &'static Mutex<GlobalConfig> {
        // 使用MaybeUninit延迟初始化
        static mut CONF: MaybeUninit<Mutex<GlobalConfig>> = MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static ONCE: Once = Once::new();

        ONCE.call_once(|| unsafe {
            CONF.as_mut_ptr().write(Mutex::new(GlobalConfig {
                config: GlobalSecurityConfig::default(),
            }));
        });
        unsafe { &*CONF.as_ptr() }
    }

    pub fn update(webc: &GlobalSecurityConfig) {
        Self::get().lock().unwrap().config = webc.clone();
    }

    pub fn config() -> GlobalSecurityConfig {
        Self::get().lock().unwrap().config.clone()
    }

    pub fn get_pool_size() -> usize {
        Self::config().pool_size
    }

    pub fn get_worker_threads() -> usize {
        Self::config().work_threads
    }
}

#[allow(dead_code)]
pub fn get_multiple_rbatis(url: &str) -> &'static RBatis {
    // 使用MaybeUninit延迟初始化
    static mut STATIC_MULTI_RB: MaybeUninit<HashMap<String, RBatis>> = MaybeUninit::uninit();
    // Once带锁保证只进行一次初始化
    static ONCE_HASH: Once = Once::new();

    ONCE_HASH.call_once(|| unsafe {
        STATIC_MULTI_RB.as_mut_ptr().write(HashMap::new());
    });

    unsafe {
        if (*STATIC_MULTI_RB.as_mut_ptr()).contains_key(url) {
            let hashmap = &*STATIC_MULTI_RB.as_mut_ptr();
            &hashmap[url]
        } else {
            //async_std::task::block_on(async {
                log::info!("Call the block on to create the sql connection.");
                let rb = RBatis::new();
                match rb.init(get_sql_driver(url), url) {
                    Ok(_) => {
                        log::info!(
                            "Database {} was connected. Rbatis was initialized successfully.",
                            url
                        );
                    }
                    Err(err) => {
                        log::warn!("Error: {}", err);
                    }
                };
                (*STATIC_MULTI_RB.as_mut_ptr()).insert(url.to_string(), rb);
            //});
            let hashmap = &*STATIC_MULTI_RB.as_mut_ptr();
            &hashmap[url]
        }
    }
}

#[allow(dead_code)]
pub async fn get_multiple_rbatis_async(url: &str) -> &'static RBatis {
    // 使用MaybeUninit延迟初始化
    static mut STATIC_MULTI_RB: MaybeUninit<HashMap<String, RBatis>> = MaybeUninit::uninit();
    // Once带锁保证只进行一次初始化
    static ONCE_HASH: Once = Once::new();

    ONCE_HASH.call_once(|| unsafe {
        STATIC_MULTI_RB.as_mut_ptr().write(HashMap::new());
    });

    unsafe {
        if (*STATIC_MULTI_RB.as_mut_ptr()).contains_key(url) {
            let hashmap = &*STATIC_MULTI_RB.as_mut_ptr();
            &hashmap[url]
        } else {
            log::info!("Call the block on to create the sql connection.");
            let rb = RBatis::new();
            match rb.link(get_sql_driver(url), url).await {
                Ok(_) => {
                    log::info!(
                        "Database {} was connected. Rbatis was initialized successfully.",
                        url
                    );
                }
                Err(err) => {
                    log::warn!("Error: {}", err);
                }
            };
            (*STATIC_MULTI_RB.as_mut_ptr()).insert(url.to_string(), rb);
            let hashmap = &*STATIC_MULTI_RB.as_mut_ptr();
            &hashmap[url]
        }
    }
}

pub fn get_local_timestamp() -> u64 {
    let now = SystemTime::now();
    let date: DateTime<Local> = now.into();
    date.timestamp_millis() as u64
}

pub fn build_path_ns(
    path: impl AsRef<Path>,
    ns: &str,
    name: impl AsRef<Path>,
) -> Result<PathBuf, anyhow::Error> {
    let pathbuf = path.as_ref().join(ns);
    build_path(pathbuf, name)
}

pub fn build_path(
    path: impl AsRef<Path>,
    name: impl AsRef<Path>,
) -> Result<PathBuf, anyhow::Error> {
    let path = path.as_ref();
    let name = name.as_ref();
    if let Some(parent) = path.parent() {
        if let Err(err) = create_dir_all(parent) {
            log::debug!("Could not create dir for {}. err {err}", parent.to_string_lossy());
        }
    }
        
    let permitted = path.canonicalize()?;
    let s_path = match name.is_absolute() || name.starts_with(path) {
        true => name.to_path_buf(),
        false => permitted.join(name),
    };    
    match s_path.starts_with(permitted) {
        true => Ok(s_path),
        false => bail!("path not permitted"),
    }
}

pub fn check_path_permitted(
    path: impl AsRef<Path>,
    permitted: &[&str],
) -> Result<(), anyhow::Error> {
    let current_path = std::env::current_dir()?;
    for sub in permitted {
        let permitted = current_path.join(sub).canonicalize()?;
        let path = path.as_ref().canonicalize()?;
        if path.starts_with(permitted) {
            return Ok(());
        }
    }
    bail!("path not permitted");
}

pub trait ChineseCount {
    fn chinese_length(&self) -> usize;

    fn chars_len(&self) -> usize;
}

impl ChineseCount for &str {
    fn chinese_length(&self) -> usize {
        self.chars()
            .filter(|c| {
                // 基本汉字区间
                (0x4E00..=0x9FFF).contains(&(c.to_owned() as i32)) ||
            // 扩展A区汉字区间
            (0x3400..=0x4DBF).contains(&(c.to_owned() as i32)) ||
            // 扩展B区汉字区间
            (0x20000..=0x2A6DF).contains(&(c.to_owned() as i32))
            })
            .count()
    }

    fn chars_len(&self) -> usize {
        self.chars().count()
    }
}

impl ChineseCount for String {
    fn chinese_length(&self) -> usize {
        self.chars()
            .filter(|c| {
                // 基本汉字区间
                (0x4E00..=0x9FFF).contains(&(c.to_owned() as i32)) ||
            // 扩展A区汉字区间
            (0x3400..=0x4DBF).contains(&(c.to_owned() as i32)) ||
            // 扩展B区汉字区间
            (0x20000..=0x2A6DF).contains(&(c.to_owned() as i32))
            })
            .count()
    }

    fn chars_len(&self) -> usize {
        self.chars().count()
    }
}

pub fn copy_to_slice<T: Copy>(to: &mut [T], source: &[T]) {
    let to_len = to.len();
    to[..(to_len.min(source.len()))].copy_from_slice(&source[..(to_len.min(source.len()))]);
}


pub fn num_of_cpus() -> usize {
    num_cpus::get()
}
