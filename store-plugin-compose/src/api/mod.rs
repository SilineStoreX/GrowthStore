use std::sync::{Arc, Mutex};

use chimes_store_core::service::files::UploadFileInfo;
use chimes_store_core::service::invoker::JwtFromDepot;
use chimes_store_core::service::{
    invoker::InvocationContext, sdk::InvokeUri, starter::MxStoreService,
};
use chimes_store_core::utils::ApiResult;
use rbatis::Page;
use salvo::handler;
use salvo::{writing::Json, Depot, Request};
use serde_json::{Map, Value};

use crate::proc::fileparts::process_filepart;
use crate::proc::ComposePluginConfig;

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
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let method = req.param::<String>("method").unwrap();
    let uri = format!("compose://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::info!("Could not parse the body as json value, {:?}", err);
            args.push(Value::Null);
        }
    }

    match req.parse_body::<Value>().await {
        Ok(tt) => {
            args.push(tt);
        }
        Err(err) => {
            log::info!("Could not parse the body as json value {:?}", err);
            args.push(Value::Null);
        }
    }

    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
                match pls.invoke_return_option(invoke_uri, ctx, args).await {
                    Ok(ret) => Json(ApiResult::ok(ret)),
                    Err(err) => Json(ApiResult::error(
                        500,
                        &format!("Runtime exception: {:?}", err),
                    )),
                }
            }
            None => Json(ApiResult::error(
                404,
                &format!("Not-Found for plugin-service {}", uri),
            )),
        }
    } else {
        Json(ApiResult::error(
            404,
            &format!("Could not parse URI {}", uri),
        ))
    }
}

#[handler]
pub async fn execute_vec_request(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Vec<Value>>> {
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let method = req.param::<String>("method").unwrap();
    let uri = format!("compose://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::info!("Could not parse the body as json value, {:?}", err);
            args.push(Value::Null);
        }
    }

    match req.parse_body::<Value>().await {
        Ok(tt) => {
            args.push(tt);
        }
        Err(err) => {
            log::info!("Could not parse the body as json value {:?}", err);
            args.push(Value::Null);
        }
    }

    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
                match pls.invoke_return_vec(invoke_uri, ctx, args).await {
                    Ok(ret) => Json(ApiResult::ok(ret)),
                    Err(err) => Json(ApiResult::error(
                        500,
                        &format!("Runtime exception: {:?}", err),
                    )),
                }
            }
            None => Json(ApiResult::error(
                404,
                &format!("Not-Found for plugin-service {}", uri),
            )),
        }
    } else {
        Json(ApiResult::error(
            404,
            &format!("Could not parse URI {}", uri),
        ))
    }
}

#[handler]
pub async fn execute_paged_request(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Page<Value>>> {
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let method = req.param::<String>("method").unwrap();
    let uri = format!("compose://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::info!("Could not parse the body as json value, {:?}", err);
            args.push(Value::Null);
        }
    }

    match req.parse_body::<Value>().await {
        Ok(tt) => {
            args.push(tt);
        }
        Err(err) => {
            log::info!("Could not parse the body as json value {:?}", err);
            args.push(Value::Null);
        }
    }

    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
                match pls.invoke_return_page(invoke_uri, ctx, args).await {
                    Ok(ret) => Json(ApiResult::ok(ret)),
                    Err(err) => Json(ApiResult::error(
                        500,
                        &format!("Runtime exception: {:?}", err),
                    )),
                }
            }
            None => Json(ApiResult::error(
                404,
                &format!("Not-Found for plugin-service {}", uri),
            )),
        }
    } else {
        Json(ApiResult::error(
            404,
            &format!("Could not parse URI {}", uri),
        ))
    }
}

#[handler]
pub async fn execute_upload_request(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Vec<Value>>> {
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let method = req.param::<String>("method").unwrap();
    let uri = format!("compose://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_queries::<Value>() {
        Ok(tt) => args.push(tt),
        Err(err) => {
            log::info!("Could not parse the body as json value, {:?}", err);
            args.push(Value::Null);
        }
    }

    match req.parse_body::<Value>().await {
        Ok(tt) => {
            args.push(tt);
        }
        Err(err) => {
            log::info!("Could not parse the body as json value {:?}", err);
            args.push(Value::Null);
        }
    }

    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
                if let Some(val) = pls.get_config() {
                    if let Ok(tconf) = serde_json::from_value::<ComposePluginConfig>(val) {
                        if let Some(css) = tconf.get(&invoke_uri.method) {
                            if !css.fileupload {
                                return Json(ApiResult::error(
                                    400,
                                    &format!(
                                        "Could not execute none-upload function by this {}",
                                        uri
                                    ),
                                ));
                            }

                            if let Ok(form_data) = req.form_data().await {
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
                                        return Json(ApiResult::error(
                                            413,
                                            &format!("Size exceed {}", fm.max_filesize()),
                                        ));
                                    }
                                    let ufls_args = ufls
                                        .iter()
                                        .filter_map(|f| {
                                            if let Ok(t) = serde_json::to_value(f.clone()) {
                                                Some(t)
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<Vec<Value>>();

                                    match pls.invoke_return_vec(invoke_uri, ctx, ufls_args).await {
                                        Ok(ret) => Json(ApiResult::ok(ret)),
                                        Err(err) => Json(ApiResult::error(
                                            500,
                                            &format!("Runtime exception: {:?}", err),
                                        )),
                                    }
                                } else {
                                    Json(ApiResult::error(
                                        400,
                                        &format!("No file parts provided {}", uri),
                                    ))
                                }
                            } else {
                                Json(ApiResult::error(
                                    400,
                                    &format!("No file parts provided {}", uri),
                                ))
                            }
                        } else {
                            Json(ApiResult::error(
                                400,
                                &format!("Could not found compose service defined by this {}", uri),
                            ))
                        }
                    } else {
                        Json(ApiResult::error(
                            400,
                            &format!(
                                "Could not convert  to compose service configuration {}",
                                uri
                            ),
                        ))
                    }
                } else {
                    Json(ApiResult::error(
                        404,
                        &format!("Not-Found for plugin-service configuration {}", uri),
                    ))
                }
            }
            None => Json(ApiResult::error(
                404,
                &format!("Not-Found for plugin-service {}", uri),
            )),
        }
    } else {
        Json(ApiResult::error(
            404,
            &format!("Could not parse URI {}", uri),
        ))
    }
}
