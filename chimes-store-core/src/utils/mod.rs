use anyhow::{anyhow, bail};
use chimes_dbs_factory::get_sql_driver;
use chimes_store_utils::file::get_current_dir;
use chrono::{DateTime, Local};
use rbatis::rbdc::db::Driver;
use rbatis::rbdc::pool::Pool;
use rbatis::{DefaultPool, Page, RBatis};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

pub mod algorithm;
pub mod crypto;
pub mod executor;
pub mod extractor;
pub mod filetypes;
pub mod global_data;
pub mod redis;
pub mod response;
pub use response::*;

pub mod futures_exeuctors {
    use std::future::Future;

    // pub use futures_lite::future::block_on as futures_executre_block_on;
    // pub use tokio::task::block_in_place as futures_executre_block_on;
    // pub use futures::executor::block_on as futures_executre_block_on;

    pub use futures::executor::block_on;
    pub fn futures_executre_block_on<F>(f: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = tokio::runtime::Handle::current();
        //std::thread::spawn(move || {
        //    handle.block_on(f)
        //}).join().unwrap()
        handle.block_on(f)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalSecurityConfig {
    pub console_code_page: Option<String>,
    pub rsa_password_public_key: Option<String>,
    pub rsa_password_private_key: Option<String>,
    pub aes_encryption_key: Option<String>,
    pub aes_encryption_solt: Option<String>,
    pub work_threads: usize,
    pub pool_size: usize,
    pub logfile: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    config: GlobalSecurityConfig,
}

impl GlobalConfig {
    pub fn get() -> &'static Mutex<GlobalConfig> {
        // 使用MaybeUninit延迟初始化
        static _CONF: OnceLock<Mutex<GlobalConfig>> = OnceLock::new();

        _CONF.get_or_init(|| {
            Mutex::new(GlobalConfig {
                config: GlobalSecurityConfig::default(),
            })
        })
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

fn get_max_opens_option(url: &str) -> (u64, u64, u64) {
    let mut maxopens = 0u64;
    let mut maxidle = 0u64;
    let mut lifetime = 0u64;

    match url::Url::parse(url) {
        Ok(u) => {
            for (key, value) in u.query_pairs().into_iter() {
                if key.trim() == "max_opens" {
                    maxopens = value.parse::<u64>().unwrap_or_default();
                } else if key.trim() == "max_idle" {
                    maxidle = value.parse::<u64>().unwrap_or_default();
                }
                if key.trim() == "lifetime" {
                    lifetime = value.parse::<u64>().unwrap_or_default();
                }
            }

            (maxopens, maxidle, lifetime)
        }
        Err(_) => (0u64, 0u64, 0u64),
    }
}

/**
 * 一个用于检查RBatis连接池还是否有效的线程
 */
#[allow(dead_code)]
async fn check_rbatis_validate(tp: &HashMap<String, RBatis>) {
    for (_, rb) in tp.iter() {
        if let Err(err) = rb.try_acquire_timeout(Duration::from_secs(60)).await {
            log::warn!("_timeout, so that remove it {err}");
        }
    }
}

#[allow(dead_code)]
pub fn get_multiple_rbatis_sync(url: &str) -> RBatis {
    // 使用MaybeUninit延迟初始化
    static STATIC_MULTI_RB: OnceLock<Mutex<HashMap<String, RBatis>>> = OnceLock::new();
    let rbmap = STATIC_MULTI_RB.get_or_init(|| Mutex::new(HashMap::new()));
    if let Entry::Vacant(e) = rbmap.lock().unwrap().entry(url.to_owned()) {
        let rb = RBatis::new();
        let (max_opens, max_idle, lifetime) = get_max_opens_option(url);
        let init_rbatis_result = if max_opens > 0 || max_idle > 0 || lifetime > 0 {
            let driver = get_sql_driver(url);
            let mut opts = driver.default_option();
            if let Err(err) = opts.set_uri(url) {
                log::error!("error {err} to setup url for db {url}");
            }
            // if let Ok(pool) = MobcPool::new(rbatis::rbdc::pool::conn_manager::ConnManager::new_arc(Arc::new(Box::new(driver)), Arc::new(opts))) {
            //     futures_blockon_async!(async {
            //         pool.inner.set_max_open_conns(max_opens).await;
            //         pool.inner.set_conn_max_lifetime(Some(Duration::from_secs(60))).await;
            //         pool.inner.set_max_idle_conns(max_opens / 10).await;
            //     });

            //     rb.init_pool(pool)
            // } else {
            if let Ok(pool) = DefaultPool::new(rbatis::rbdc::pool::ConnectionManager::new_arc(
                Arc::new(Box::new(driver)),
                Arc::new(opts),
            )) {
                pool.inner.set_max_open(max_opens);
                // if lifetime > 0 {
                //   pool.set_conn_max_lifetime(Some(Duration::from_secs(lifetime))).await;
                // }
                // pool.set_max_idle_conns(max_idle).await;
                rb.init_pool(pool)
            } else {
                rb.init(get_sql_driver(url), url)
            }
        } else {
            rb.init(get_sql_driver(url), url)
        };

        match init_rbatis_result {
            Ok(_) => {
                log::info!("Database {url} was connected. Rbatis was initialized successfully.");
            }
            Err(err) => {
                log::warn!("Error: {err}");
            }
        };
        e.insert(rb);
    }

    rbmap.lock().unwrap().get(url).unwrap().clone()
}

#[allow(dead_code)]
pub async fn get_multiple_rbatis_async(url: &str) -> RBatis {
    // 使用MaybeUninit延迟初始化
    static STATIC_MULTI_RB_MAP: OnceLock<Mutex<HashMap<String, RBatis>>> = OnceLock::new();

    let rb_map = STATIC_MULTI_RB_MAP.get_or_init(|| Mutex::new(HashMap::new()));

    if let Entry::Vacant(e) = rb_map.lock().unwrap().entry(url.to_owned()) {
        let rb = RBatis::new();
        let (max_opens, max_idle, lifetime) = get_max_opens_option(url);
        let init_rbatis_result = if max_opens > 0 || max_idle > 0 || lifetime > 0 {
            let driver = get_sql_driver(url);
            let mut opts = driver.default_option();
            if let Err(err) = opts.set_uri(url) {
                log::error!("error {err} to setup url for db {url}");
            }

            // if let Ok(pool) = MobcPool::new(rbatis::rbdc::pool::conn_manager::ConnManager::new_arc(Arc::new(Box::new(driver)), Arc::new(opts))) {
            //     pool.inner.set_max_open_conns(max_opens).await;
            //     pool.inner.set_conn_max_lifetime(Some(Duration::from_secs(60))).await;
            //     pool.inner.set_max_idle_conns(max_opens / 10).await;
            //     rb.init_pool(pool)
            // } else {
            if let Ok(pool) = DefaultPool::new(rbatis::rbdc::pool::ConnectionManager::new_arc(
                Arc::new(Box::new(driver)),
                Arc::new(opts),
            )) {
                if max_opens > 0 {
                    pool.inner.set_max_open(max_opens);
                }
                //    pool.set_max_open_conns(max_opens).await;
                //    if lifetime > 0 {
                //         pool.set_conn_max_lifetime(Some(Duration::from_secs(lifetime))).await;
                //    }
                //    pool.set_max_idle_conns(max_idle).await;
                rb.init_pool(pool)
            } else {
                rb.init(get_sql_driver(url), url)
            }
        } else {
            rb.init(get_sql_driver(url), url)
        };

        match init_rbatis_result {
            Ok(_) => {
                log::info!("Database {url} was connected. Rbatis was initialized successfully.");
            }
            Err(err) => {
                log::warn!("Error: {err}");
            }
        };

        e.insert(rb);
    }

    rb_map.lock().unwrap().get(url).unwrap().clone()
    // rb_map.lock().unwrap().get(url).unwrap().clone()
}

pub fn create_rbatis(url: &str) -> Result<RBatis, anyhow::Error> {
    let rb = RBatis::new();
    match rb.init(get_sql_driver(url), url) {
        Ok(_) => Ok(rb),
        Err(err) => {
            log::warn!("Error: {err}");
            Err(anyhow!(err))
        }
    }
}

pub async fn create_rbatis_v2(url: &str) -> Result<RBatis, anyhow::Error> {
    Ok(get_multiple_rbatis_async(url).await)
}

pub fn get_local_timestamp() -> u64 {
    let now = SystemTime::now();
    let date: DateTime<Local> = now.into();
    date.timestamp_millis() as u64
}

pub fn get_local_timestamp_micros() -> u64 {
    let now = SystemTime::now();
    let date: DateTime<Local> = now.into();
    date.timestamp_micros() as u64
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
            log::debug!(
                "Could not create dir for {}. err {err}",
                parent.to_string_lossy()
            );
        }
    }

    let permitted = match path.canonicalize() {
        Ok(tpath) => tpath,
        Err(err) => {
            log::debug!("error on canonicalize {err}, try to create the full path again.");
            if let Err(err) = create_dir_all(path) {
                log::debug!("error to create full path {err}");
            }
            match path.canonicalize() {
                Ok(mpath) => mpath,
                Err(err) => {
                    return Err(anyhow!(err));
                }
            }
        }
    };

    let s_path = match name.is_absolute() || name.starts_with(path) {
        true => name.to_path_buf(),
        false => permitted.join(name),
    };
    match s_path.starts_with(permitted.clone()) {
        true => Ok(s_path),
        false => {
            log::info!(
                "Path was not permitted {} of {}",
                s_path.to_string_lossy(),
                permitted.to_string_lossy()
            );
            bail!("path not permitted")
        }
    }
}

pub fn check_path_permitted(
    path: impl AsRef<Path>,
    permitted: &[&str],
) -> Result<(), anyhow::Error> {
    let current_path = get_current_dir()?;
    for sub in permitted {
        let permitted = current_path.join(sub).canonicalize()?;
        let path = path.as_ref().canonicalize()?;
        if path.starts_with(permitted) {
            return Ok(());
        }
    }
    bail!("path not permitted");
}

pub fn json_into_paged_value(ret: Option<Value>) -> Result<Page<Value>, anyhow::Error> {
   ret.map(|r| {
        serde_json::from_value::<Page<Value>>(r).map_err(|err| anyhow!("error to convert page {err}"))
    }).unwrap_or(Ok(Page::new_total(1, 20, 0)))
}

pub fn json_into_vec_value(ret: Option<Value>) -> Result<Vec<Value>, anyhow::Error> {
   ret.map(|r| {
        serde_json::from_value::<Vec<Value>>(r).map_err(|err| anyhow!("error to convert page {err}"))
    }).unwrap_or(Ok(vec![]))
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

pub fn num_of_cpus() -> usize {
    num_cpus::get()
}
