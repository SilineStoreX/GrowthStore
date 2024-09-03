use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::starter::MxStoreService;
use chimes_store_core::utils::redis::{redis_delexp_cmd, redis_flushall_cmd};
use chimes_store_core::utils::ApiResult;
use chimes_store_core::{service::invoker::JwtFromDepot, utils::redis::redis_keys};
use rbatis::Page;
use salvo::prelude::*;
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[handler]
pub async fn redis_get_object(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let query = req.query::<String>("key").unwrap_or_default();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("redis://{}/redis?{}#get", ns, query),
        ctx,
        vec![],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(506, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn redis_get_vec(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Vec<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let query = req.query::<String>("key").unwrap_or_default();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_vec(
        format!("redis://{}/redis?{}#get", ns, query),
        ctx,
        vec![],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }

}

#[handler]
pub async fn redis_get_page(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Page<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let query = req.query::<String>("key").unwrap_or_default();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_page(
        format!("redis://{}/redis?{}#get", ns, query),
        ctx,
        vec![],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }

}

#[handler]
pub async fn redis_set_infinit(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let query = req.query::<String>("key").unwrap_or_default();
    let body = match req.parse_body::<Value>().await {
        Ok(b) => b,
        Err(err) => {
            return Json(ApiResult::error(400, format!("{}", err).as_str()));
        }
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("redis://{}/redis?{}#set", ns, query),
        ctx,
        vec![body],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(506, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn redis_del_infinit(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let query = req.query::<String>("key").unwrap_or_default();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("redis://{}/redis?{}#del", ns, query),
        ctx,
        vec![],
    )
    .await
    {
        Ok(rs) => {
            log::info!("RS: {:?}", rs);
            Json(ApiResult::ok(rs))
        }
        Err(err) => Json(ApiResult::error(506, format!("{}", err).as_str())),
    }

}

#[handler]
pub async fn redis_keys_vec(_depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Vec<String>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let query = req.query::<String>("key").unwrap_or_default();
    log::info!("invoke redis_keys cmd");
    match redis_keys(&ns, &query) {
        Ok(t) => Json(ApiResult::ok(t)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn redis_flushall(
    _depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Option<String>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    match redis_flushall_cmd(&ns) {
        Ok(t) => Json(ApiResult::ok(t)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn redis_delexp(
    _depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Option<String>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let query = req.query::<String>("key").unwrap_or_default();

    match redis_delexp_cmd(&ns, &query) {
        Ok(t) => Json(ApiResult::ok(t)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }
}
