use chimes_store_core::{
    pin_submit,
    service::{registry::SchemaRegistry, starter::MxStoreService},
    utils::redis::init_ns_scoped_redis,
};
use dbs::{
    invoker::{DbQueryServiceInvocation, DbStoreServiceInvocation},
    redis::RedisInvocation,
};
pub mod api;
pub mod dbs;
pub mod docs;
pub mod utils;

pub fn register_objects_and_querys(ns: &str) {
    SchemaRegistry::get_mut().register("object", Box::new(DbStoreServiceInvocation()));
    SchemaRegistry::get_mut().register("query", Box::new(DbQueryServiceInvocation()));
    SchemaRegistry::get_mut().register("redis", Box::new(RedisInvocation()));
    let nms = ns.to_owned();
    pin_submit!(async move {
        if let Some(ms) = MxStoreService::get(&nms) {
            log::info!("init redis connection for {}", ms.get_namespace());
            init_ns_scoped_redis(&ms.get_config());
        }
    });
}
