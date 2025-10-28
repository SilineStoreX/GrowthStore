use chimes_store_core::{
    pin_async_process,
    service::{registry::SchemaRegistry, starter::MxStoreService},
    utils::redis::init_ns_scoped_redis,
};
use dbs::{
    invoker::{DbQueryServiceInvocation, DbStoreServiceInvocation},
    redis::RedisInvocation,
};
pub mod api;
pub mod dbs;
pub mod utils;

pub fn register_objects_and_querys(ns: &str) {
    SchemaRegistry::get_mut().register("object", Box::new(DbStoreServiceInvocation()));
    SchemaRegistry::get_mut().register("query", Box::new(DbQueryServiceInvocation()));
    SchemaRegistry::get_mut().register("redis", Box::new(RedisInvocation()));
    let nms = ns.to_owned();
    // 初始REDIS时，如果REDIS的配置是错误的，则会出现一堆的异常，而影响其它部分的启动
    pin_async_process!(async move {
        if let Some(ms) = MxStoreService::get(&nms) {
            init_ns_scoped_redis(&ms.get_config());
        }
    });
}
