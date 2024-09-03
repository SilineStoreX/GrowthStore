use chimes_store_core::service::starter::MxStoreService;
use itertools::Itertools;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

pub mod auth;
pub use auth::*;
pub mod common;
pub mod management;
pub mod performance;
// pub mod crud;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRegistry {
    pub id: String,
    pub label: String,
    pub name: String,
    pub icon: String,
    pub leaf: bool,
}

lazy_static! {
    static ref FUNCTION_REGISTRY_LIST: Vec<FunctionRegistry> = vec![
        FunctionRegistry {
            id: "_config".to_owned(),
            label: "配置".to_owned(),
            name: "Setting".to_owned(),
            icon: "SetUp".to_owned(),
            leaf: true
        },
        FunctionRegistry {
            id: "_object".to_owned(),
            label: "存储服务".to_owned(),
            name: "Object".to_owned(),
            icon: "Basketball".to_owned(),
            leaf: false
        },
        FunctionRegistry {
            id: "_query".to_owned(),
            label: "查询服务".to_owned(),
            name: "Query".to_owned(),
            icon: "Football".to_owned(),
            leaf: false
        },
        FunctionRegistry {
            id: "_plugin".to_owned(),
            label: "扩展服务".to_owned(),
            name: "Plugin".to_owned(),
            icon: "Suitcase".to_owned(),
            leaf: false
        },
        FunctionRegistry {
            id: "_redis".to_owned(),
            label: "Redis服务".to_owned(),
            name: "Redis".to_owned(),
            icon: "Coin".to_owned(),
            leaf: true
        },
        FunctionRegistry {
            id: "_es".to_owned(),
            label: "ElasticSearch".to_owned(),
            name: "elasticsearch".to_owned(),
            icon: "Money".to_owned(),
            leaf: true
        },
    ];
}

impl FunctionRegistry {
    
    #[allow(dead_code)]
    pub fn get_all_functions() -> Vec<Self> {
        FUNCTION_REGISTRY_LIST.to_vec()
    }

    #[allow(dead_code)]
    pub fn get_active_functions(ns: &str) -> Vec<Self> {
        log::info!("active function for {ns}");
        FUNCTION_REGISTRY_LIST.to_vec().into_iter().filter(|p| {
            (p.id != "_es" && p.id != "_redis")
            || (p.id == "_es" && MxStoreService::get(ns).map(|f| !f.get_plugin_config_by_protocol("elasticsearch").is_empty()).unwrap_or(false))
            || (p.id == "_redis" && MxStoreService::get(ns).map(|f| f.get_config().redis_url.map(|url| !url.is_empty()).unwrap_or_default()).unwrap_or(false))
        }).collect_vec()
    }
}
