use chimes_store_core::pin_blockon_async;
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::starter::MxStoreService;
use chimes_store_core::utils::{ApiResult, ApiResult2};
use chimes_store_core::{config::QueryCondition, service::invoker::JwtFromDepot};
use rbatis::Page;
use salvo::prelude::*;
use serde_json::{json, Value};
use std::any::Any;
use std::sync::{Arc, Mutex};

#[handler]
pub async fn select(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let id = req.param::<String>("id").unwrap();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{}/{}#select", ns, name),
        ctx,
        vec![Value::String(id)],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn find_one(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<QueryCondition>().await.unwrap();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{}/{}#find_one", ns, name),
        ctx,
        vec![json!(cond)],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }

}

#[handler]
pub async fn insert(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<Value>().await.unwrap();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{}/{}#insert", ns, name),
        ctx,
        vec![json!(cond)],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn update(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<Value>().await.unwrap();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{}/{}#update", ns, name),
        ctx,
        vec![json!(cond)],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn save_batch(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<Vec<Value>>().await.unwrap();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{}/{}#save_batch", ns, name),
        ctx,
        cond,
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn upsert(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<Value>().await.unwrap();
    let condition: Option<QueryCondition> = if let Some(dync) = cond.get("_cond") {
        if let Ok(c) = serde_json::from_value::<QueryCondition>(dync.to_owned()) {
            if c.is_empty() {
                None
            } else {
                Some(c)
            }
        } else {
            None
        }
    } else {
        None
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
    let args = if condition.is_none() {
        vec![cond]
    } else {
        vec![cond, json!(condition.unwrap())]
    };
    match MxStoreService::invoke_return_one(
        format!("object://{}/{}#upsert", ns, name),
        ctx,
        args,
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn delete(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<Value>().await.unwrap();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{}/{}#delete", ns, name),
        ctx,
        vec![json!(cond)],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }

}

#[handler]
pub async fn delete_by(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<Value>().await.unwrap();
    log::info!("Cond: {:?}", cond);
    let condition: QueryCondition = if let Some(dync) = cond.get("_cond") {
        if let Ok(c) = serde_json::from_value::<QueryCondition>(dync.to_owned()) {
            log::info!("QCC: {:?}", c);
            c
        } else {
            return Json(ApiResult::error(
                400,
                "Could not convert to QueryCondition by _cond",
            ));
        }
    } else {
        return Json(ApiResult::error(400, "No _cond defined in the JSON Body"));
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{}/{}#delete_by", ns, name),
        ctx,
        vec![cond, json!(condition)],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn update_by(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<Value>().await.unwrap();

    let condition: QueryCondition = if let Some(dync) = cond.get("_cond") {
        if let Ok(c) = serde_json::from_value(dync.to_owned()) {
            c
        } else {
            QueryCondition::default()
        }
    } else {
        QueryCondition::default()
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(format!("object://{}/{}#update_by", ns, name), ctx, vec![cond, json!(condition)]).await {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn query(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Vec<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<QueryCondition>().await.unwrap();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_vec(
        format!("object://{}/{}#query", ns, name),
        ctx,
        vec![json!(cond)],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }
}

#[handler]
pub async fn paged_query(depot: &mut Depot, req: &mut Request) -> Json<ApiResult2<Page<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<QueryCondition>().await.unwrap();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_page(
        format!("object://{}/{}#paged_query", ns, name),
        ctx,
        vec![json!(cond)],
    )
    .await
    {
        Ok(rs) => Json(ApiResult2::ok(rs)),
        Err(err) => Json(ApiResult2::error(500, format!("{}", err).as_str())),
    }

}

#[handler]
pub async fn query_search(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Vec<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<Value>().await.unwrap();

    let condition: QueryCondition = if let Some(dync) = cond.get("_cond") {
        if let Ok(c) = serde_json::from_value(dync.to_owned()) {
            c
        } else {
            QueryCondition::default()
        }
    } else {
        QueryCondition::default()
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_vec(
        format!("query://{}/{}#search", ns, name),
        ctx,
        vec![cond, json!(condition)],
    )
    .await
    {
        Ok(rs) => Json(ApiResult::ok(rs)),
        Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
    }

}

#[handler]
pub async fn query_paged_search(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult2<Page<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<Value>().await.unwrap();

    let condition: QueryCondition = if let Some(dync) = cond.get("_cond") {
        if let Ok(c) = serde_json::from_value(dync.to_owned()) {
            c
        } else {
            QueryCondition::default()
        }
    } else {
        QueryCondition::default()
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_page(
        format!("query://{}/{}#paged_search", ns, name),
        ctx,
        vec![cond, json!(condition)],
    )
    .await
    {
        Ok(rs) => Json(ApiResult2::ok(rs)),
        Err(err) => Json(ApiResult2::error(500, format!("{}", err).as_str())),
    }

}


#[handler]
pub async fn test(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let cond = req.parse_body::<Value>().await.unwrap();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    pin_blockon_async!(async move {    
        let ret = match MxStoreService::invoke_return_one(
            format!("object://{}/{}#select", ns, name),
            ctx,
            vec![json!(cond)],
        )
        .await
        {
            Ok(rs) => Json(ApiResult::ok(rs)),
            Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
        };
        Box::new(ret) as Box<dyn Any + Send + Sync>
    }).unwrap_or(Json(ApiResult::error(500, "Unknonw error".to_string().as_str())))
}