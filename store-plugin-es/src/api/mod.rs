use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use chimes_store_core::service::invoker::JwtFromDepot;
use chimes_store_core::service::perfs::InvokeCounter;
use chimes_store_core::service::registry::SchemaRegistry;
use chimes_store_core::service::{
    invoker::InvocationContext, sdk::InvokeUri, starter::MxStoreService,
};
use chimes_store_core::utils::ApiResult;
use salvo::handler;
use salvo::{writing::Json, Depot, Request};
use serde_json::Value;

#[handler]
pub async fn execute_elastic_request(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Option<Value>>> {
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let method = req.param::<String>("method").unwrap_or_default();
    let uri = format!("elasticsearch://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_body::<Value>().await {
        Ok(tt) => {
            match tt.clone() {
                Value::Array(mut tms) => {
                    args.append(&mut tms);
                }
                Value::Object(_tm) => {
                    args.push(tt);
                }
                _ => {
                    return Json(ApiResult::error(400, "No payload provided"));
                }
            };
        }
        Err(err) => {
            log::debug!("Could not parse the body as json value {err:?}");
            args.push(Value::Null);
        }
    }

    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
        let mut ict = InvokeCounter::new(&invoke_uri, ctx.clone(), &args);
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => match pls.invoke_return_option(invoke_uri, ctx, args).await {
                Ok(ret) => {
                    SchemaRegistry::send_invoke_count(&mut ict);
                    Json(ApiResult::ok(ret))
                }
                Err(err) => {
                    SchemaRegistry::send_invoke_err(&mut ict, &err);
                    Json(ApiResult::error(
                        500,
                        &format!("Runtime exception: {err:?}"),
                    ))
                }
            },
            None => {
                SchemaRegistry::send_invoke_err(
                    &mut ict,
                    &anyhow!("Not-Found for plugin-service {}", uri),
                );
                Json(ApiResult::error(
                    404,
                    &format!("Not-Found for plugin-service {uri}"),
                ))
            }
        }
    } else {
        Json(ApiResult::error(404, &format!("Could not parse URI {uri}")))
    }
}

#[handler]
pub async fn execute_elastic_management(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Option<Value>>> {
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let method = req.param::<String>("method").unwrap();
    let uri = format!("elasticsearch://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_body::<Value>().await {
        Ok(tt) => {
            match tt.clone() {
                Value::Array(mut tms) => {
                    args.append(&mut tms);
                }
                Value::Object(_tm) => {
                    args.push(tt);
                }
                _ => {
                    return Json(ApiResult::error(400, "No payload provided"));
                }
            };
        }
        Err(err) => {
            log::debug!("Could not parse the body as json value {err:?}");
            args.push(Value::Null);
        }
    }

    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
        let mut ict = InvokeCounter::new(&invoke_uri, ctx.clone(), &args);
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => match pls.invoke_return_option(invoke_uri, ctx, args).await {
                Ok(ret) => {
                    SchemaRegistry::send_invoke_count(&mut ict);
                    Json(ApiResult::ok(ret))
                }
                Err(err) => {
                    SchemaRegistry::send_invoke_err(&mut ict, &err);
                    Json(ApiResult::error(
                        500,
                        &format!("Runtime exception: {err:?}"),
                    ))
                }
            },
            None => {
                SchemaRegistry::send_invoke_err(
                    &mut ict,
                    &anyhow!("Not-Found for plugin-service {}", uri),
                );
                Json(ApiResult::error(
                    404,
                    &format!("Not-Found for plugin-service {uri}"),
                ))
            }
        }
    } else {
        Json(ApiResult::error(404, &format!("Could not parse URI {uri}")))
    }
}
