use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use chimes_store_core::config::auth::JwtUserClaims;
use chimes_store_core::service::files::UploadFileInfo;
use chimes_store_core::service::invoker::{release_context, JwtFromDepot};
use chimes_store_core::service::perfs::InvokeCounter;
use chimes_store_core::service::registry::SchemaRegistry;
use chimes_store_core::service::{
    invoker::InvocationContext, sdk::InvokeUri, starter::MxStoreService,
};
use chimes_store_core::utils::ApiResult;
use chimes_store_utils::common::to_xml_response;
use salvo::handler;
use salvo::http::StatusCode;
use salvo::prelude::JwtAuthDepotExt;
use salvo::writing::Text;
use salvo::{writing::Json, Depot, Request};
use serde_json::{json, Map, Value};

use crate::proc::fileparts::process_filepart;
use crate::proc::ComposePluginConfig;


#[handler]
pub async fn get_schema(
    req: &mut Request,
    resp: &mut salvo::Response,
) {
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let method = req.param::<String>("method").unwrap_or_default();
    let uri = format!("compose://{ns}/{name}#{method}");
    
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
                        &format!("compose service {invoke_uri:?} was not found"),
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
    resp: &mut salvo::Response,
) {
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let method = req.param::<String>("method").unwrap_or_default();
    let uri = format!("compose://{ns}/{name}#{method}");
    let mut args = vec![];

    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::debug!("Could not parse the body as json value, {err:?}");
            args.push(Value::Null);
        }
    }

    match chimes_store_utils::common::parse_request_body::<Value>(req, req.secure_max_size()).await
    {
        Ok(tv) => {
            args.push(tv);
        }
        Err(err) => {
            log::debug!("Could not parse the body as value {err:?}");
            args.push(Value::Null);
        }
    }

    // log::warn!("Args: {args:?}");

    let jwt = depot.jwt_auth_data::<JwtUserClaims>().map(|c| c.claims.clone());

    let (respdata, xmlresp, rawdata) = if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
        let mut ict = InvokeCounter::new(&invoke_uri, ctx.clone(), &args);
        let resdata = match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                // 根据配置进行解密操作
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
                //                 &format!("Params verified faild by {err}"),
                //             )));
                //     return;
                // };

                match pls
                    .invoke_return_option(invoke_uri, ctx.clone(), newargs)
                    .await
                {
                    Ok(ret) => {
                        SchemaRegistry::send_invoke_count(&mut ict);
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
        };

        let xmlrsp = ctx.lock().unwrap().get_response_xml();
        let retraw = ctx.lock().unwrap().get_return_rawdata();

        release_context(&ctx).await;
        drop(ctx);

        (resdata, xmlrsp, retraw)
    } else {
        (
            Json(ApiResult::error(404, &format!("Could not parse URI {uri}"))),
            false,
            false,
        )
    };

    // log::warn!("data: {respdata:?}, {xmlresp}, {rawdata}");

    if rawdata {
        if xmlresp {
            let text = to_xml_response(&respdata.0.data).unwrap_or("<xml>ERR</xml>".to_owned());
            // log::warn!("text xml: {text}");
            resp.render(Text::Xml(text));
        } else {
            resp.render(Json(respdata.0.data));
        }
    } else if xmlresp {
        let text = to_xml_response(&respdata.0).unwrap_or("<xml>ERR</xml>".to_owned());
        // log::warn!("text2 xml: {text}");
        resp.render(Text::Xml(text));
    } else {
        resp.render(respdata);
    }
}

#[handler]
pub async fn execute_vec_request(depot: &mut Depot, req: &mut Request, resp: &mut salvo::Response) {
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let method = req.param::<String>("method").unwrap();
    let uri = format!("compose://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::debug!("Could not parse the body as json value, {err:?}");
            args.push(Value::Null);
        }
    }

    match chimes_store_utils::common::parse_request_body::<Value>(req, req.secure_max_size()).await
    {
        Ok(tv) => {
            args.push(tv);
        }
        Err(err) => {
            log::debug!("Could not parse the body as value {err:?}");
            args.push(Value::Null);
        }
    }

    let jwt = depot.jwt_auth_data::<JwtUserClaims>().map(|c| c.claims.clone());

    let (respdata, xmlresp, rawdata) = if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
        let mut ict = InvokeCounter::new(&invoke_uri, ctx.clone(), &args);
        let resdata = match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
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
                //                 &format!("Params verified faild by {err}"),
                //             )));
                //     return; 
                // };                

                match pls.invoke_return_vec(invoke_uri, ctx.clone(), newargs).await {
                    Ok(ret) => {
                        SchemaRegistry::send_invoke_count(&mut ict);
                        let status = ctx.lock().unwrap().get_status();
                        if status == 0 {
                            Json(ApiResult::ok(ret))
                        } else {
                            let msg = ctx.lock().unwrap().get_message();
                            Json(ApiResult::new(status as i32, &msg, ret))
                        }
                        // Json(ApiResult::ok(ret))
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
        };

        let xmlrsp = ctx.lock().unwrap().get_response_xml();
        let retraw = ctx.lock().unwrap().get_return_rawdata();
        release_context(&ctx).await;
        drop(ctx);
        (resdata, xmlrsp, retraw)
    } else {
        (
            Json(ApiResult::error(404, &format!("Could not parse URI {uri}"))),
            false,
            false,
        )
    };

    if rawdata {
        if xmlresp {
            let text = to_xml_response(&respdata.0.data).unwrap_or("<xml>ERR</xml>".to_owned());
            log::warn!("text xml: {text}");
            resp.render(Text::Xml(text));
        } else {
            resp.render(Json(respdata.0.data));
        }
    } else if xmlresp {
        let text = to_xml_response(&respdata.0).unwrap_or("<xml>ERR</xml>".to_owned());
        log::warn!("text2 xml: {text}");
        resp.render(Text::Xml(text));
    } else {
        resp.render(respdata);
    }
}

#[handler]
pub async fn execute_paged_request(
    depot: &mut Depot,
    req: &mut Request,
    resp: &mut salvo::Response,
) {
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let method = req.param::<String>("method").unwrap();
    let uri = format!("compose://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::debug!("Could not parse the body as json value, {err:?}");
            args.push(Value::Null);
        }
    }

    match chimes_store_utils::common::parse_request_body::<Value>(req, req.secure_max_size()).await
    {
        Ok(tv) => {
            args.push(tv);
        }
        Err(err) => {
            log::debug!("Could not parse the body as value {err:?}");
            args.push(Value::Null);
        }
    }

    let jwt = depot.jwt_auth_data::<JwtUserClaims>().map(|c| c.claims.clone());
    let (respdata, xmlresp, rawdata) = if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
        let mut ict = InvokeCounter::new(&invoke_uri, ctx.clone(), &args);
        let resdata = match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
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
                //                 &format!("Params verified faild by {err}"),
                //             )));
                //     return; 
                // };

                match pls.invoke_return_page(invoke_uri, ctx.clone(), newargs).await {
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
        };

        let xmlrsp = ctx.lock().unwrap().get_response_xml();
        let retraw = ctx.lock().unwrap().get_return_rawdata();
        release_context(&ctx).await;
        drop(ctx);

        (resdata, xmlrsp, retraw)
    } else {
        (
            Json(ApiResult::error(404, &format!("Could not parse URI {uri}"))),
            false,
            false,
        )
    };

    if rawdata {
        if xmlresp {
            let text = to_xml_response(&respdata.0.data).unwrap_or("<xml>ERR</xml>".to_owned());
            log::warn!("text xml: {text}");
            resp.render(Text::Xml(text));
        } else {
            resp.render(Json(respdata.0.data));
        }
    } else if xmlresp {
        let text = to_xml_response(&respdata.0).unwrap_or("<xml>ERR</xml>".to_owned());
        log::warn!("text2 xml: {text}");
        resp.render(Text::Xml(text));
    } else {
        resp.render(respdata);
    }
}

#[handler]
pub async fn execute_upload_request(
    depot: &mut Depot,
    req: &mut Request,
    resp: &mut salvo::Response,
) {
    let ns = req.param::<String>("ns").unwrap_or_default();
    let name = req.param::<String>("name").unwrap_or_default();
    let method = req.param::<String>("method").unwrap_or_default();
    let uri = format!("compose://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::debug!("Could not parse the body as json value, {err:?}");
            args.push(Value::Null);
        }
    }

    match chimes_store_utils::common::parse_request_body::<Value>(req, req.secure_max_size()).await
    {
        Ok(tv) => {
            args.push(tv);
        }
        Err(err) => {
            log::debug!("Could not parse the body as value {err:?}");
            args.push(Value::Null);
        }
    }

    let jwt = depot.jwt_auth_data::<JwtUserClaims>().map(|c| c.claims.clone());
    let (respdata, xmlresp, rawdata) = if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
        let mut ict = InvokeCounter::new(&invoke_uri, ctx.clone(), &[]);
        let resdata = match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                // Upload was not support encrytion for body. just verify sign
                let signkey = pls.get_api_secret(depot);
                match pls.verify_args_sign(&invoke_uri, &jwt, signkey, args.clone()) {
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

                // if let Err(err) = pls.verify_params(&invoke_uri, args.clone()) {
                //     SchemaRegistry::send_invoke_err(&mut ict, &err);
                //     resp.status_code(StatusCode::BAD_REQUEST)
                //             .render(Json(ApiResult::<ApiResult<String>>::error(
                //                 400,
                //                 &format!("Params verified faild by {err}"),
                //             )));
                //     return; 
                // };

                if let Some(val) = pls.get_config() {
                    if let Ok(tconf) = serde_json::from_value::<ComposePluginConfig>(val) {
                        if let Some(css) = tconf.get(&invoke_uri.method) {
                            if !css.fileupload {
                                SchemaRegistry::send_invoke_err(
                                    &mut ict,
                                    &anyhow!(
                                        "Could not execute none-upload function by this {}",
                                        uri
                                    ),
                                );
                                Json(ApiResult::error(
                                    400,
                                    &format!(
                                        "Could not execute none-upload function by this {uri}"
                                    ),
                                ))
                            } else if let Ok(form_data) = req.form_data().await {
                                let form_object = form_data
                                    .fields
                                    .flat_iter()
                                    .map(|(k, v)| (k.to_owned(), Value::String(v.to_owned())))
                                    .collect::<Map<String, Value>>();

                                let file_field = css.file_field.unwrap_or("file".to_string());
                                let files_vec = form_data
                                    .files
                                    .get_vec(&file_field)
                                    .map(|f| f.to_owned())
                                    .unwrap_or(vec![]);

                                if let Some(mss) = MxStoreService::get(&invoke_uri.namespace) {
                                    let fm = mss.get_filestore();
                                    let ufls = files_vec
                                        .iter()
                                        .map(|fl| process_filepart(&fm, fl, &form_object))
                                        .collect::<Vec<UploadFileInfo>>();

                                    if ufls.iter().any(|f| !f.copied && f.file_id.is_none()) {
                                        SchemaRegistry::send_invoke_err(
                                            &mut ict,
                                            &anyhow!(
                                                "Size exceed {} for {}",
                                                fm.max_filesize(),
                                                uri
                                            ),
                                        );
                                        Json(ApiResult::error(
                                            413,
                                            &format!("Size exceed {}", fm.max_filesize()),
                                        ))
                                    } else {
                                        let ufls_args = ufls
                                            .iter()
                                            .filter_map(|f| serde_json::to_value(f.clone()).ok())
                                            .collect::<Vec<Value>>();

                                        ict.set_payload(&ufls_args);

                                        match pls
                                            .invoke_return_vec(invoke_uri, ctx.clone(), ufls_args)
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
                                } else {
                                    SchemaRegistry::send_invoke_err(
                                        &mut ict,
                                        &anyhow!("No file parts provided {}", uri),
                                    );
                                    Json(ApiResult::error(
                                        400,
                                        &format!("No file parts provided {uri}"),
                                    ))
                                }
                            } else {
                                SchemaRegistry::send_invoke_err(
                                    &mut ict,
                                    &anyhow!("No file parts provided {}", uri),
                                );
                                Json(ApiResult::error(
                                    400,
                                    &format!("No file parts provided {uri}"),
                                ))
                            }
                        } else {
                            SchemaRegistry::send_invoke_err(
                                &mut ict,
                                &anyhow!("Could not found compose service defined by this {}", uri),
                            );
                            Json(ApiResult::error(
                                400,
                                &format!("Could not found compose service defined by this {uri}"),
                            ))
                        }
                    } else {
                        SchemaRegistry::send_invoke_err(
                            &mut ict,
                            &anyhow!(
                                "Could not convert  to compose service configuration {}",
                                uri
                            ),
                        );
                        Json(ApiResult::error(
                            400,
                            &format!("Could not convert  to compose service configuration {uri}"),
                        ))
                    }
                } else {
                    SchemaRegistry::send_invoke_err(
                        &mut ict,
                        &anyhow!("Not-Found for plugin-service {}", uri),
                    );
                    Json(ApiResult::error(
                        404,
                        &format!("Not-Found for plugin-service configuration {uri}"),
                    ))
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
        };

        let xmlrsp = ctx.lock().unwrap().get_response_xml();
        let retraw = ctx.lock().unwrap().get_return_rawdata();
        release_context(&ctx).await;
        drop(ctx);
        (resdata, xmlrsp, retraw)
    } else {
        (
            Json(ApiResult::error(404, &format!("Could not parse URI {uri}"))),
            false,
            false,
        )
    };

    if rawdata {
        if xmlresp {
            let text = to_xml_response(&respdata.0.data).unwrap_or("<xml>ERR</xml>".to_owned());
            log::warn!("text xml: {text}");
            resp.render(Text::Xml(text));
        } else {
            resp.render(Json(respdata.0.data));
        }
    } else if xmlresp {
        let text = to_xml_response(&respdata.0).unwrap_or("<xml>ERR</xml>".to_owned());
        log::warn!("text2 xml: {text}");
        resp.render(Text::Xml(text));
    } else {
        resp.render(respdata);
    }
}
