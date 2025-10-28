use chimes_store_core::service::invoker::{release_context, InvocationContext};
use chimes_store_core::service::starter::MxStoreService;
use chimes_store_core::utils::{ApiResult, ApiResult2};
use chimes_store_core::{config::QueryCondition, service::invoker::JwtFromDepot};
use rbatis::Page;
use salvo::prelude::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[handler]
pub async fn schemas(_depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    
    match MxStoreService::get(&ns) {
        Some(mx) => {
            match mx.get_object(&name) {
                Some(st) => {
                    let stc = mx.get_config();
                    let basic_sch = st.to_validate_schema(&stc, false, true)
                                .map(|s| serde_json::to_value(s).map(Some).unwrap_or(None))
                                .unwrap_or(None);

                    let pri_sch = st.to_validate_schema(&stc, true, true)
                                .map(|s| serde_json::to_value(s).map(Some).unwrap_or(None))
                                .unwrap_or(None);

                    let ins_sch = st.to_validate_schema(&stc, false, false)
                                .map(|s| serde_json::to_value(s).map(Some).unwrap_or(None))
                                .unwrap_or(None);

                    Json(ApiResult::ok(Some(json!({"request": { "primary_schema": pri_sch, "insert_schema": ins_sch, "body_schema": basic_sch }, "response": basic_sch}))))
                },
                None => {
                    Json(ApiResult::ok(None))
                }
            }
        },
        None => Json(ApiResult::ok(None)),
    }
}

#[handler]
pub async fn query_schemas(_depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    
    match MxStoreService::get(&ns) {
        Some(mx) => {
            match mx.get_query(&name) {
                Some(st) => {
                    let stc = mx.get_config();
                    let param_schema = st.to_params_schema(&stc)
                                .map(|s| serde_json::to_value(s).map(Some).unwrap_or(None))
                                .unwrap_or(None);

                    let result_schema = st.to_result_schema(&stc)
                                .map(|s| serde_json::to_value(s).map(Some).unwrap_or(None))
                                .unwrap_or(None);

                    Json(ApiResult::ok(Some(json!({"request": param_schema, "response": result_schema}))))
                },
                None => {
                    Json(ApiResult::ok(None))
                }
            }
        },
        None => Json(ApiResult::ok(None)),
    }
}

#[handler]
pub async fn select(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let id = req.param::<String>("id").unwrap_or_default();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{ns}/{name}#select"),
        ctx.clone(),
        vec![Value::String(id)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            if status == 0 {
                release_context(&ctx).await;
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                release_context(&ctx).await;
                Json(ApiResult::new(status as i32, &msg, ret))
            }
            // Json(ApiResult::ok(rs))
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("object://{ns}/{name}#select failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn find_one(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req
        .parse_body::<QueryCondition>()
        .await
        .unwrap_or(QueryCondition::default());

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{ns}/{name}#find_one"),
        ctx.clone(),
        vec![json!(cond)],
    )
    .await
    {
        Ok(ret) => {
            // Json(ApiResult::ok(rs))
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("object://{ns}/{name}#find_one failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn insert(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<Value>().await.unwrap_or(Value::Null);

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{ns}/{name}#insert"),
        ctx.clone(),
        vec![json!(cond)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
            // Json(ApiResult::ok(rs))
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("object://{ns}/{name}#insert failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn update(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<Value>().await.unwrap_or(Value::Null);

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{ns}/{name}#update"),
        ctx.clone(),
        vec![json!(cond)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("object://{ns}/{name}#update failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn save_batch(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<Vec<Value>>().await.unwrap_or(vec![]);

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{ns}/{name}#save_batch"),
        ctx.clone(),
        cond,
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("object://{ns}/{name}#save_batch failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn upsert(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<Value>().await.unwrap_or(Value::Null);
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
        format!("object://{ns}/{name}#upsert"),
        ctx.clone(),
        args,
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("object://{ns}/{name}#upsert failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn delete(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<Value>().await.unwrap_or(Value::Null);

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{ns}/{name}#delete"),
        ctx.clone(),
        vec![json!(cond)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("object://{ns}/{name}#delete failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn delete_by(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<Value>().await.unwrap_or(Value::Null);
    log::info!("Cond: {cond:?}");
    let condition: QueryCondition = if let Some(dync) = cond.get("_cond") {
        if let Ok(c) = serde_json::from_value::<QueryCondition>(dync.to_owned()) {
            log::info!("QCC: {c:?}");
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
        format!("object://{ns}/{name}#delete_by"),
        ctx.clone(),
        vec![cond, json!(condition)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("object://{ns}/{name}#delete_by failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn update_by(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<Value>().await.unwrap_or(Value::Null);

    let condition: QueryCondition = if let Some(dync) = cond.get("_cond") {
        serde_json::from_value(dync.to_owned()).unwrap_or_default()
    } else {
        QueryCondition::default()
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("object://{ns}/{name}#update_by"),
        ctx.clone(),
        vec![cond, json!(condition)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("object://{ns}/{name}#update_by failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn query(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Vec<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req
        .parse_body::<QueryCondition>()
        .await
        .unwrap_or(QueryCondition::default());

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_vec(
        format!("object://{ns}/{name}#query"),
        ctx.clone(),
        vec![json!(cond)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("object://{ns}/{name}#query failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn paged_query(depot: &mut Depot, req: &mut Request) -> Json<ApiResult2<Page<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<QueryCondition>().await.unwrap_or_default();

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_page(
        format!("object://{ns}/{name}#paged_query"),
        ctx.clone(),
        vec![json!(cond)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult2::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult2::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("object://{ns}/{name}#paged_query failed by {err}");
            Json(ApiResult2::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn query_search(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Vec<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<Value>().await.unwrap_or_default();

    let condition: QueryCondition = if let Some(dync) = cond.get("_cond") {
        serde_json::from_value(dync.to_owned()).unwrap_or_default()
    } else {
        QueryCondition::default()
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_vec(
        format!("query://{ns}/{name}#search"),
        ctx.clone(),
        vec![cond, json!(condition)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("query://{ns}/{name}#search failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn query_single(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<Value>().await.unwrap_or_default();

    let condition: QueryCondition = if let Some(dync) = cond.get("_cond") {
        serde_json::from_value(dync.to_owned()).unwrap_or_default()
    } else {
        QueryCondition::default()
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("query://{ns}/{name}#find_one"),
        ctx.clone(),
        vec![cond, json!(condition)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("query://{ns}/{name}#find_one failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn query_execute(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<Value>().await.unwrap_or_default();

    let condition: QueryCondition = if let Some(dync) = cond.get("_cond") {
        serde_json::from_value(dync.to_owned()).unwrap_or_default()
    } else {
        QueryCondition::default()
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_one(
        format!("query://{ns}/{name}#execute"),
        ctx.clone(),
        vec![cond, json!(condition)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("query://{ns}/{name}#execute failed by {err}");
            Json(ApiResult::error(500, format!("{err}").as_str()))
        }
    }
}

#[handler]
pub async fn query_paged_search(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult2<Page<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let cond = req.parse_body::<Value>().await.unwrap_or_default();

    let condition: QueryCondition = if let Some(dync) = cond.get("_cond") {
        serde_json::from_value(dync.to_owned()).unwrap_or_default()
    } else {
        QueryCondition::default()
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

    match MxStoreService::invoke_return_page(
        format!("query://{ns}/{name}#paged_search"),
        ctx.clone(),
        vec![cond, json!(condition)],
    )
    .await
    {
        Ok(ret) => {
            let status = ctx.lock().unwrap().get_status();
            release_context(&ctx).await;
            if status == 0 {
                Json(ApiResult2::ok(ret))
            } else {
                let msg = ctx.lock().unwrap().get_message();
                Json(ApiResult2::new(status as i32, &msg, ret))
            }
        }
        Err(err) => {
            release_context(&ctx).await;
            log::error!("query://{ns}/{name}#paged_search failed by {err}");
            Json(ApiResult2::error(500, format!("{err}").as_str()))
        }
    }
}
