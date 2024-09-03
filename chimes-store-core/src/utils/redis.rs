use crate::config::StoreServiceConfig;
use anyhow::{anyhow, Error};
use itertools::Itertools;
use once_cell::sync::OnceCell;
use r2d2::Pool;
use redis::{
    cluster::ClusterClientBuilder, from_redis_value, ConnectionLike, FromRedisValue, RedisError,
    RedisResult,
};
use redis::{Commands, Value};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum InstanceType {
    Single,
    Cluster,
}

#[derive(Debug, Clone, Default)]
pub struct RedisPoolConfig {
    pub connection_timeout: u64,
    pub max_size: u32,
    pub mini_idel: u32,
}

#[derive(Debug, Clone, Default)]
pub struct RedisConfig {
    pub urls: Vec<String>,
    pub database: i64,
    pub username: Option<String>,
    pub password: Option<String>,
    pub instance_type: Option<InstanceType>,
    pub pool: RedisPoolConfig,
}

impl RedisConfig {
    #[allow(dead_code)]
    fn instance_type_default() -> InstanceType {
        InstanceType::Single
    }
}

pub fn to_redis_client(full_urls: &str) -> RedisResult<RedisClient> {
    let urls = full_urls.split(';').collect_vec();
    if urls.len() == 1 {
        let cl = redis::Client::open(urls[0])?;
        Ok(RedisClient::Single(cl))
    } else {
        let cb = ClusterClientBuilder::new(urls.clone());
        let cl = cb.build()?;
        Ok(RedisClient::Cluster(cl))
    }
}

#[derive(Clone)]
pub enum RedisClient {
    Single(redis::Client),
    Cluster(redis::cluster::ClusterClient),
}

impl RedisClient {
    pub fn get_redis_connection(&self) -> RedisResult<RedisConnection> {
        match self {
            RedisClient::Single(s) => {
                let conn = s.get_connection()?;
                Ok(RedisConnection::Single(Box::new(conn)))
            }
            RedisClient::Cluster(c) => {
                let conn = c.get_connection()?;
                Ok(RedisConnection::Cluster(Box::new(conn)))
            }
        }
    }
}

pub enum RedisConnection {
    Single(Box<redis::Connection>),
    Cluster(Box<redis::cluster::ClusterConnection>),
}

impl RedisConnection {
    pub fn is_open(&self) -> bool {
        match self {
            RedisConnection::Single(sc) => sc.is_open(),
            RedisConnection::Cluster(cc) => cc.is_open(),
        }
    }

    pub fn query<T: FromRedisValue>(&mut self, cmd: &redis::Cmd) -> RedisResult<T> {
        match self {
            RedisConnection::Single(sc) => match sc.as_mut().req_command(cmd) {
                Ok(val) => from_redis_value(&val),
                Err(e) => Err(e),
            },
            RedisConnection::Cluster(cc) => match cc.req_command(cmd) {
                Ok(val) => from_redis_value(&val),
                Err(e) => Err(e),
            },
        }
    }

    pub fn raw_del(&mut self, key: &str) -> RedisResult<Value> {
        match self {
            RedisConnection::Single(sc) => match sc.as_mut().del(key) {
                Ok(val) => Ok(val),
                Err(e) => Err(e),
            },
            RedisConnection::Cluster(cc) => match cc.del(key) {
                Ok(val) => Ok(val),
                Err(e) => Err(e),
            },
        }
    }

    pub fn raw_keys(&mut self, key: &str) -> RedisResult<Vec<String>> {
        log::info!("redis keys {}", key);
        match self {
            RedisConnection::Single(sc) => match sc.as_mut().keys(key) {
                Ok(val) => {
                    log::info!("Result: {:?}", val);
                    match from_redis_value(&val) {
                        Ok(t) => Ok(t),
                        Err(err) => {
                            log::info!("Err parse {:?}", err);
                            Err(err)
                        }
                    }
                }
                Err(e) => Err(e),
            },
            RedisConnection::Cluster(cc) => match cc.keys(key) {
                Ok(val) => from_redis_value(&val),
                Err(e) => Err(e),
            },
        }
    }
}

#[derive(Clone)]
pub struct RedisConnectionManager {
    pub redis_client: RedisClient,
}

// ToDo 实现 broken 函数
impl r2d2::ManageConnection for RedisConnectionManager {
    type Connection = RedisConnection;
    type Error = RedisError;

    fn connect(&self) -> Result<RedisConnection, Self::Error> {
        let conn = self.redis_client.get_redis_connection()?;
        Ok(conn)
    }

    fn is_valid(&self, conn: &mut RedisConnection) -> Result<(), Self::Error> {
        match conn {
            RedisConnection::Single(sc) => {
                redis::cmd("PING").query(sc)?;
            }
            RedisConnection::Cluster(cc) => {
                redis::cmd("PING").query(cc)?;
            }
        }
        Ok(())
    }

    fn has_broken(&self, conn: &mut RedisConnection) -> bool {
        !conn.is_open()
    }
}

// pub fn init_redis_pool() {
//     GLOBAL_REDIS_POOL.get_or_init(|| {
//         let pool = gen_redis_conn_pool().unwrap();
//         pool
//     });
// }

pub fn gen_redis_conn_pool(url: &str) -> Result<Pool<RedisConnectionManager>, Error> {
    let redis_client: RedisClient = match to_redis_client(url) {
        Ok(rc) => rc,
        Err(err) => {
            return Err(anyhow!(err.to_string()));
        }
    };
    let manager = RedisConnectionManager { redis_client };
    match r2d2::Pool::builder()
        .max_size(128)
        .min_idle(Some(10))
        .connection_timeout(Duration::from_secs(30))
        .build(manager)
    {
        Ok(pool) => Ok(pool),
        Err(err) => Err(anyhow!(err.to_string())),
    }
}

pub static GLOBAL_REDIS_POOL: OnceCell<Mutex<HashMap<String, Arc<Pool<RedisConnectionManager>>>>> =
    OnceCell::new();

pub fn init_global_redis() {
    GLOBAL_REDIS_POOL.get_or_init(|| Mutex::new(HashMap::new()));
}

pub fn init_ns_scoped_redis(conf: &StoreServiceConfig) {
    if let Some(mp) = GLOBAL_REDIS_POOL.get() {
        if let Some(url) = conf.redis_url.clone() {
            log::info!("{}'s redis-url: {}", conf.namespace, url);
            if let Ok(t) = gen_redis_conn_pool(&url) {
                mp.lock()
                    .unwrap()
                    .insert(conf.namespace.clone(), Arc::new(t));
            }
        }
    }
}

pub fn get_ns_scoped_redis(ns: &str) -> Option<Arc<Pool<RedisConnectionManager>>> {
    if let Some(mp) = GLOBAL_REDIS_POOL.get() {
        mp.lock().unwrap().get(ns).cloned()
    } else {
        None
    }
}

pub fn get_redis_connection(ns: &str) -> Option<Arc<Pool<RedisConnectionManager>>> {
    get_ns_scoped_redis(ns)
}

pub fn redis_get(ns: &str, key: &str) -> Result<Option<String>, Error> {
    let conn = get_redis_connection(ns);
    match conn {
        Some(c) => match c.get() {
            Ok(mut tc) => match tc.query::<redis::Value>(redis::cmd("GET").arg(key)) {
                Ok(xv) => {
                    let cn = match xv {
                        redis::Value::Bulk(_) => String::new(),
                        redis::Value::Okay => String::new(),
                        redis::Value::Status(st) => st,
                        redis::Value::Data(tp) => String::from_utf8(tp).unwrap(),
                        redis::Value::Int(tp) => tp.to_string(),
                        redis::Value::Nil => String::new(),
                    };
                    log::info!("resp: {cn}");
                    if cn.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(cn))
                    }
                }
                Err(err) => Err(anyhow!(err.to_string())),
            },
            Err(err) => Err(anyhow!(err.to_string())),
        },
        None => Ok(None),
    }
}

pub fn redis_set(ns: &str, key: &str, value: &str) -> Result<Option<String>, Error> {
    let conn = get_redis_connection(ns);
    match conn {
        Some(c) => match c.get() {
            Ok(mut tc) => match tc.query::<String>(redis::cmd("SET").arg(key).arg(value)) {
                Ok(xv) => Ok(Some(xv)),
                Err(err) => Err(anyhow!(err.to_string())),
            },
            Err(err) => Err(anyhow!(err.to_string())),
        },
        None => Ok(None),
    }
}

pub fn redis_set_expire(
    ns: &str,
    key: &str,
    value: &str,
    expire: u64,
) -> Result<Option<String>, Error> {
    let conn = get_redis_connection(ns);
    match conn {
        Some(c) => match c.get() {
            Ok(mut tc) => {
                match tc
                    .query::<String>(redis::cmd("SET").arg(key).arg(value).arg("EX").arg(expire))
                {
                    Ok(xv) => Ok(Some(xv)),
                    Err(err) => Err(anyhow!(err.to_string())),
                }
            }
            Err(err) => Err(anyhow!(err.to_string())),
        },
        None => Ok(None),
    }
}

pub fn redis_del(ns: &str, key: &str) -> Result<Option<String>, Error> {
    log::info!("key to del: {}", key);
    let conn = get_redis_connection(ns);
    match conn {
        Some(c) => match c.get() {
            Ok(mut tc) => match tc.raw_del(key) {
                Ok(xv) => {
                    log::info!("was query send.");
                    let cv = match xv {
                        redis::Value::Int(t) => t.to_string(),
                        _ => "err".to_string(),
                    };
                    log::info!("response: {}", cv);
                    Ok(Some(cv))
                }
                Err(err) => {
                    log::info!("del error {err:?}");
                    Err(anyhow!(err.to_string()))
                }
            },
            Err(err) => {
                log::info!("remove error {err:?}");
                Err(anyhow!(err.to_string()))
            }
        },
        None => {
            log::info!("No connection.");
            Ok(None)
        }
    }
}

pub fn redis_delexp_cmd(ns: &str, key: &str) -> Result<Option<String>, Error> {
    let conn = get_redis_connection(ns);
    match conn {
        Some(c) => match c.get() {
            Ok(mut tc) => {
                match tc.query::<Vec<String>>(redis::cmd("KEYS").arg(&format!("{}*", key))) {
                    Ok(xv) => {
                        for mc in xv {
                            if let Err(err) = tc.query::<String>(redis::cmd("DEL").arg(&mc)) {
                                log::info!("Del {} by an error {}", mc, err);
                            }
                        }
                        Ok(None)
                    }
                    Err(err) => Err(anyhow!(err.to_string())),
                }
            }
            Err(err) => Err(anyhow!(err.to_string())),
        },
        None => Ok(None),
    }
}

pub fn redis_keys(ns: &str, key: &str) -> Result<Vec<String>, Error> {
    let conn = get_redis_connection(ns);
    match conn {
        Some(c) => match c.get() {
            Ok(mut tc) => match tc.raw_keys(&format!("{}*", key)) {
                Ok(xv) => {
                    log::info!("result: {xv:?}");
                    Ok(xv)
                }
                Err(err) => Err(anyhow!(err.to_string())),
            },
            Err(err) => Err(anyhow!(err.to_string())),
        },
        None => {
            log::info!("No redis connection.");
            Ok(vec![])
        }
    }
}

pub fn redis_flushall_cmd(ns: &str) -> Result<Option<String>, Error> {
    let conn = get_redis_connection(ns);
    match conn {
        Some(c) => match c.get() {
            Ok(mut tc) => match tc.query::<String>(&redis::cmd("FLUSHALL")) {
                Ok(xv) => Ok(Some(xv)),
                Err(err) => Err(anyhow!(err.to_string())),
            },
            Err(err) => Err(anyhow!(err.to_string())),
        },
        None => Ok(None),
    }
}
