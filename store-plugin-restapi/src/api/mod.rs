use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use chimes_store_core::config::auth::JwtUserClaims;
use chimes_store_core::service::invoker::JwtFromDepot;
use chimes_store_core::service::perfs::InvokeCounter;
use chimes_store_core::service::registry::SchemaRegistry;
use chimes_store_core::service::{
    invoker::InvocationContext, sdk::InvokeUri, starter::MxStoreService,
};
use chimes_store_core::utils::ApiResult;
use futures_lite::StreamExt;
use rbatis::Page;
use reqwest::StatusCode;
use salvo::prelude::JwtAuthDepotExt;
use salvo::sse::{SseError, SseEvent};
use salvo::{handler, Response};
use salvo::{writing::Json, Depot, Request};
use serde_json::{json, Value};

#[handler]
pub async fn get_schema(
    req: &mut Request,
    resp: &mut salvo::Response,
) {
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let method = req.param::<String>("method").unwrap_or_default();
    let uri = format!("restapi://{ns}/{name}#{method}");
    
    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                let req_sch = pls.get_invoke_param_schema(&invoke_uri).unwrap_or(json!({"type": "object"}));
                let resp_sch = pls.get_response_schema(&invoke_uri).unwrap_or(json!({"type": "object"}));
                resp.status_code(StatusCode::OK)
                    .render(Json(ApiResult::<Value>::ok(json!({"request": req_sch, "response": resp_sch}))));
            },
            None => {
                resp.status_code(StatusCode::NOT_FOUND)
                    .render(Json(ApiResult::<String>::error(
                        400,
                        &format!("restapi service {invoke_uri:?} was not found"),
                    )));
            }
        }
    } else {
        resp.status_code(StatusCode::NOT_FOUND)
            .render(Json(ApiResult::<String>::error(
                404,
                &format!("Not found by this uri {uri}"),
            )));
        return;
    }
}


// 如何来确定传入的参数
// 对于自行定义的REST API接口的话，我们需要根据Query和Body两个部分来进行参数定义（可以传递到程序中的）。
// 如此，由于来源有这Query和Body这两个问分，我们因此需要对Query的参数，和Body的参数进行一个说明。
// 因为，传给Script的Args是一个Value数组，我们可以通过Value数据来方式来处理它们。
// Query：总是放在Args列表中的第一个参数，可以在脚本中通过args[0]来获得。
// Body: 总是放在Args列表中的第二个参数，可以在脚本中通过args[1]来获取。
#[handler]
pub async fn execute_single_request(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Option<Value>>> {
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let method = req.param::<String>("method").unwrap_or_default();
    let uri = format!("restapi://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::debug!("Could not parse the body as json value, {err:?}");
            args.push(Value::Null);
        }
    }

    match req.parse_body::<Value>().await {
        Ok(tt) => {
            args.push(tt);
        }
        Err(err) => {
            log::debug!("Could not parse the body as json value {err:?}");
            args.push(Value::Null);
        }
    }

    let jwt = depot.jwt_auth_data::<JwtUserClaims>().map(|c| c.claims.clone());

    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
        let mut ict = InvokeCounter::new(&invoke_uri, ctx.clone(), &args);
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                let newargs = match pls.decrypt_body_args(&invoke_uri, &args) {
                    Ok(tsarg) => tsarg,
                    Err(err) => {
                        SchemaRegistry::send_invoke_err(&mut ict, &err);
                        return Json(ApiResult::error(
                                400,
                                &format!("Params has not beed decrypted by {err}"),
                            ));
                    }
                };

                let signkey = pls.get_api_secret(depot);
                match pls.verify_args_sign(&invoke_uri, &jwt, signkey, newargs.clone()) {
                    Ok(verified) => {
                        if !verified {
                            SchemaRegistry::send_invoke_err(&mut ict, &anyhow!("Request params was not signed correctly"));
                            return Json(ApiResult::error(
                                500,
                                &format!("Request params was not signed correctly."),
                            ));
                        } 
                    },
                    Err(err) => {
                        SchemaRegistry::send_invoke_err(&mut ict, &err);
                        return Json(ApiResult::error(
                            500,
                            &format!("Error to verify request params {err}"),
                        ));
                    }
                }

                // if let Err(err) = pls.verify_params(&invoke_uri, newargs.clone()) {
                //     SchemaRegistry::send_invoke_err(&mut ict, &err);
                //     return Json(ApiResult::error(
                //                 400,
                //                 &format!("Params verified failed by {err}"),
                //             ));
                // };

                match pls
                    .invoke_return_option(invoke_uri, ctx.clone(), newargs)
                    .await
                {
                    Ok(ret) => {
                        SchemaRegistry::send_invoke_count(&mut ict);
                        // Json(ApiResult::ok(ret))
                        let status = ctx.lock().unwrap().get_status();
                        if status == 0 {
                            Json(ApiResult::ok(ret))
                        } else {
                            let msg = ctx.lock().unwrap().get_message();
                            Json(ApiResult::new(status as i32, &msg, ret))
                        }
                    }
                    Err(err) => {
                        SchemaRegistry::send_invoke_err(&mut ict, &err);
                        Json(ApiResult::error(
                            500,
                            &format!("Runtime exception: {err:?}"),
                        ))
                    }
                }
            }
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
pub async fn execute_vec_request(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Vec<Value>>> {
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let method = req.param::<String>("method").unwrap_or_default();
    let uri = format!("restapi://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::debug!("Could not parse the body as json value, {err:?}");
            args.push(Value::Null);
        }
    }

    match req.parse_body::<Value>().await {
        Ok(tt) => {
            args.push(tt);
        }
        Err(err) => {
            log::debug!("Could not parse the body as json value {err:?}");
            args.push(Value::Null);
        }
    }

    let jwt = depot.jwt_auth_data::<JwtUserClaims>().map(|c| c.claims.clone());

    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
        let mut ict = InvokeCounter::new(&invoke_uri, ctx.clone(), &args);
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                let newargs = match pls.decrypt_body_args(&invoke_uri, &args) {
                    Ok(tsarg) => tsarg,
                    Err(err) => {
                        SchemaRegistry::send_invoke_err(&mut ict, &err);
                        return Json(ApiResult::error(
                                400,
                                &format!("Params has not beed decrypted by {err}"),
                            ));
                    }
                };

                let signkey = pls.get_api_secret(depot);
                match pls.verify_args_sign(&invoke_uri, &jwt, signkey, newargs.clone()) {
                    Ok(verified) => {
                        if !verified {
                            SchemaRegistry::send_invoke_err(&mut ict, &anyhow!("Request params was not signed correctly"));
                            return Json(ApiResult::error(
                                500,
                                &format!("Request params was not signed correctly."),
                            ));
                        } 
                    },
                    Err(err) => {
                        SchemaRegistry::send_invoke_err(&mut ict, &err);
                        return Json(ApiResult::error(
                            500,
                            &format!("Error to verify request params {err}"),
                        ));
                    }
                }

                // if let Err(err) = pls.verify_params(&invoke_uri, newargs.clone()) {
                //     SchemaRegistry::send_invoke_err(&mut ict, &err);
                //     return Json(ApiResult::error(
                //                 400,
                //                 &format!("Params verified failed by {err}"),
                //             ));
                // };

                match pls.invoke_return_vec(invoke_uri, ctx.clone(), newargs).await {
                    Ok(ret) => {
                        SchemaRegistry::send_invoke_count(&mut ict);
                        //Json(ApiResult::ok(ret))
                        let status = ctx.lock().unwrap().get_status();
                        if status == 0 {
                            Json(ApiResult::ok(ret))
                        } else {
                            let msg = ctx.lock().unwrap().get_message();
                            Json(ApiResult::new(status as i32, &msg, ret))
                        }
                    }
                    Err(err) => {
                        SchemaRegistry::send_invoke_err(&mut ict, &err);
                        Json(ApiResult::error(
                            500,
                            &format!("Runtime exception: {err:?}"),
                        ))
                    }
                }
            }
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
pub async fn execute_paged_request(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Page<Value>>> {
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let method = req.param::<String>("method").unwrap_or_default();
    let uri = format!("restapi://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::debug!("Could not parse the body as json value, {err:?}");
            args.push(Value::Null);
        }
    }

    match req.parse_body::<Value>().await {
        Ok(tt) => {
            args.push(tt);
        }
        Err(err) => {
            log::debug!("Could not parse the body as json value {err:?}");
            args.push(Value::Null);
        }
    }

    let jwt = depot.jwt_auth_data::<JwtUserClaims>().map(|c| c.claims.clone());
    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
        let mut ict = InvokeCounter::new(&invoke_uri, ctx.clone(), &args);
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                let newargs = match pls.decrypt_body_args(&invoke_uri, &args) {
                    Ok(tsarg) => tsarg,
                    Err(err) => {
                        SchemaRegistry::send_invoke_err(&mut ict, &err);
                        return Json(ApiResult::error(
                                400,
                                &format!("Params has not beed decrypted by {err}"),
                            ));
                    }
                };

                let signkey = pls.get_api_secret(depot);
                match pls.verify_args_sign(&invoke_uri, &jwt, signkey, newargs.clone()) {
                    Ok(verified) => {
                        if !verified {
                            SchemaRegistry::send_invoke_err(&mut ict, &anyhow!("Request params was not signed correctly"));
                            return Json(ApiResult::error(
                                500,
                                &format!("Request params was not signed correctly."),
                            ));
                        } 
                    },
                    Err(err) => {
                        SchemaRegistry::send_invoke_err(&mut ict, &err);
                        return Json(ApiResult::error(
                            500,
                            &format!("Error to verify request params {err}"),
                        ));
                    }
                }

                match pls.invoke_return_page(invoke_uri, ctx.clone(), newargs).await {
                    Ok(ret) => {
                        SchemaRegistry::send_invoke_count(&mut ict);
                        // Json(ApiResult::ok(ret))
                        let status = ctx.lock().unwrap().get_status();
                        if status == 0 {
                            Json(ApiResult::ok(ret))
                        } else {
                            let msg = ctx.lock().unwrap().get_message();
                            Json(ApiResult::new(status as i32, &msg, ret))
                        }
                    }
                    Err(err) => {
                        SchemaRegistry::send_invoke_err(&mut ict, &err);
                        Json(ApiResult::error(
                            500,
                            &format!("Runtime exception: {err:?}"),
                        ))
                    }
                }
            }
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

// 用于执行SSE的请求代理
#[handler]
pub async fn execute_sse_request(depot: &mut Depot, req: &mut Request, resp: &mut Response) {
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let method = req.param::<String>("method").unwrap_or_default();
    let uri = format!("restapi://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::debug!("Could not parse the body as json value, {err:?}");
            args.push(Value::Null);
        }
    }

    match req.parse_body::<Value>().await {
        Ok(tt) => {
            args.push(tt);
        }
        Err(err) => {
            log::debug!("Could not parse the body as json value {err:?}");
            args.push(Value::Null);
        }
    }

    let jwt = depot.jwt_auth_data::<JwtUserClaims>().map(|c| c.claims.clone());
    
    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
        let mut ict = InvokeCounter::new(&invoke_uri, ctx.clone(), &args);
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                let newargs = match pls.decrypt_body_args(&invoke_uri, &args) {
                    Ok(tsarg) => tsarg,
                    Err(err) => {
                        SchemaRegistry::send_invoke_err(&mut ict, &err);
                        resp.status_code(StatusCode::BAD_REQUEST)
                            .render(Json(ApiResult::<String>::error(
                                400,
                                &format!("Params has not beed decrypted by {err}"),
                            )));
                        return;
                    }
                };

                let signkey = pls.get_api_secret(depot);
                match pls.verify_args_sign(&invoke_uri, &jwt, signkey, newargs.clone()) {
                    Ok(verified) => {
                        if !verified {
                            SchemaRegistry::send_invoke_err(&mut ict, &anyhow!("Request params was not signed correctly"));
                            resp.status_code(StatusCode::SERVICE_UNAVAILABLE)
                                .render(Json(ApiResult::<ApiResult<String>>::error(
                                    500,
                                    &format!("Request params was not signed correctly"),
                                )));
                            return;
                        } 
                    },
                    Err(err) => {
                        SchemaRegistry::send_invoke_err(&mut ict, &err);
                        resp.status_code(StatusCode::SERVICE_UNAVAILABLE)
                            .render(Json(ApiResult::<ApiResult<String>>::error(
                                500,
                                &format!("Unable to verify sign with error {err}"),
                            )));
                        return;
                    }
                }

                // if let Err(err) = pls.verify_params(&invoke_uri, newargs.clone()) {
                //     SchemaRegistry::send_invoke_err(&mut ict, &err);
                //     resp.status_code(StatusCode::BAD_REQUEST)
                //             .render(Json(ApiResult::<ApiResult<String>>::error(
                //                 400,
                //                 &format!("Params verified failed by {err}"),
                //             )));
                //         return;
                // };

                if let Some(ssepls) = pls.to_sse_request() {
                    match ssepls.invoke_sse_request(invoke_uri, ctx, newargs).await {
                        Ok(stream) => {
                            SchemaRegistry::send_invoke_count(&mut ict);
                            salvo::sse::stream(
                                resp,
                                stream.map(|t| match t {
                                    Ok(inner) => Result::<SseEvent, SseError>::Ok(inner),
                                    Err(err) => {
                                        log::error!("error on render {err:?}");
                                        Ok(SseEvent::default().text("Stream ended"))
                                    }
                                }),
                            );
                        }
                        Err(err) => {
                            SchemaRegistry::send_invoke_err(&mut ict, &err);
                            resp.status_code(StatusCode::SERVICE_UNAVAILABLE)
                                .render(Json(ApiResult::<ApiResult<String>>::error(
                                    500,
                                    &format!("Service unavailable {uri} for {err:?}"),
                                )));
                        }
                    }
                } else {
                    SchemaRegistry::send_invoke_err(&mut ict, &anyhow!("Plugin was not support SSE for plugin-service {uri}"));
                    resp.status_code(StatusCode::BAD_REQUEST)
                        .render(Json(ApiResult::<ApiResult<String>>::error(
                            400,
                            &format!("Plugin was not support SSE for plugin-service {uri}"),
                        )));
                }
            }
            None => {
                SchemaRegistry::send_invoke_err(
                    &mut ict,
                    &anyhow!("Not-Found for plugin-service {}", uri),
                );
                resp.status_code(StatusCode::BAD_REQUEST)
                    .render(Json(ApiResult::<ApiResult<String>>::error(
                        400,
                        &format!("Not-Found for plugin-service {uri}"),
                    )));
            }
        }
    } else {
        resp.status_code(StatusCode::BAD_REQUEST).render(Json(
            ApiResult::<ApiResult<String>>::error(400, &format!("Could not parse URI {uri}")),
        ));
    }
}
