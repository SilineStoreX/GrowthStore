use std::sync::{Arc, Mutex};

use chimes_store_core::{
    service::{
        invoker::{InvocationContext, JwtFromDepot},
        starter::MxStoreService,
    },
    utils::{ApiResult, ApiResult2},
};
use rbatis::Page;
use salvo::{
    fs::NamedFile, handler, http::StatusCode, oapi::endpoint, writing::Json, Depot, Request,
    Response,
};
use serde_json::Value;

use crate::utils::ManageApiResult;

#[endpoint]
pub async fn common_invoke_option(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ManageApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let cond = req.parse_body::<Value>().await.unwrap();

    if let Some(uri) = cond.get("uri") {
        if let Some(invoke_uri) = uri.as_str() {
            if let Some(params) = cond.get("params") {
                let vecparams = if params.is_array() {
                    if let Some(vecparams) = params.as_array() {
                        vecparams.clone()
                    } else {
                        vec![params.clone()]
                    }
                } else {
                    vec![params.clone()]
                };
                let full_invoke_uri = invoke_uri.to_owned();
                let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

                match MxStoreService::invoke_return_one(full_invoke_uri, ctx, vecparams).await {
                    Ok(rs) => Json(ManageApiResult::ok(rs)),
                    Err(err) => Json(ManageApiResult::error(500, format!("{}", err).as_str())),
                }
            } else {
                Json(ManageApiResult::error(
                    405,
                    "No params provided".to_string().as_str(),
                ))
            }
        } else {
            Json(ManageApiResult::error(
                405,
                "Invoke URI invalid".to_string().as_str(),
            ))
        }
    } else {
        Json(ManageApiResult::error(
            405,
            "No invoke URI provided".to_string().as_str(),
        ))
    }
}

#[endpoint]
pub async fn common_invoke_vec(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ManageApiResult<Vec<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let cond = req.parse_body::<Value>().await.unwrap();

    if let Some(uri) = cond.get("uri") {
        if let Some(invoke_uri) = uri.as_str() {
            if let Some(params) = cond.get("params") {
                let vecparams = if params.is_array() {
                    if let Some(vecparams) = params.as_array() {
                        vecparams.clone()
                    } else {
                        vec![params.clone()]
                    }
                } else {
                    vec![params.clone()]
                };
                let full_invoke_uri = invoke_uri.to_owned();
                log::info!("full_invoke_uri: {}", full_invoke_uri);
                let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

                match MxStoreService::invoke_return_vec(full_invoke_uri, ctx, vecparams).await {
                    Ok(rs) => Json(ManageApiResult::ok(rs)),
                    Err(err) => Json(ManageApiResult::error(500, format!("{}", err).as_str())),
                }
            } else {
                Json(ManageApiResult::error(
                    405,
                    "No params provided".to_string().as_str(),
                ))
            }
        } else {
            Json(ManageApiResult::error(
                405,
                "Invoke URI invalid".to_string().as_str(),
            ))
        }
    } else {
        Json(ManageApiResult::error(
            405,
            "No invoke URI provided".to_string().as_str(),
        ))
    }
}

#[handler]
pub async fn common_invoke_page(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult2<Page<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let cond = req.parse_body::<Value>().await.unwrap();

    if let Some(uri) = cond.get("uri") {
        if let Some(invoke_uri) = uri.as_str() {
            if let Some(params) = cond.get("params") {
                let vecparams = if params.is_array() {
                    if let Some(vecparams) = params.as_array() {
                        vecparams.clone()
                    } else {
                        vec![params.clone()]
                    }
                } else {
                    vec![params.clone()]
                };
                let full_invoke_uri = invoke_uri.to_owned();
                let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

                match MxStoreService::invoke_return_page(full_invoke_uri, ctx, vecparams).await
                {
                    Ok(rs) => Json(ApiResult2::ok(rs)),
                    Err(err) => Json(ApiResult2::error(500, format!("{}", err).as_str())),
                }
            } else {
                Json(ApiResult2::error(
                    405,
                    "No params provided".to_string().as_str(),
                ))
            }
        } else {
            Json(ApiResult2::error(
                405,
                "Invoke URI invalid".to_string().as_str(),
            ))
        }
    } else {
        Json(ApiResult2::error(
            405,
            "No invoke URI provided".to_string().as_str(),
        ))
    }
}

#[handler]
pub async fn common_file_send(depot: &mut Depot, req: &mut Request, res: &mut Response) {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let file_id = req.param::<String>("file_id").unwrap();
    let ns = req.param::<String>("ns").unwrap();

    if let Some(mss) = MxStoreService::get(&ns) {
        let conf = mss.get_config();
        let fm = mss.get_filestore();
        if let Some(query) = conf.download_query {
            let file_name_field = conf.download_file_name.unwrap_or("file_name".to_owned());
            let file_path_field = conf.download_file_path.unwrap_or("file_path".to_owned());
            let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
            let ret: Option<Value> = match MxStoreService::invoke_return_one(query, ctx, vec![Value::String(file_id)]).await {
                    Ok(rs) => rs,
                    Err(err) => {
                        log::info!("find file error {}", err);
                        None
                    }
                };

            if let Some(tv) = ret {
                if let Some(path) = tv.get(&file_path_field) {
                    let filename = tv.get(&file_name_field);
                    if let Value::String(pathtext) = path {
                        let fullpath = fm.combine_fullpath(pathtext);
                        if filename.is_some() {
                            NamedFile::builder(fullpath)
                                .attached_name(filename.unwrap().to_string())
                                .send(req.headers(), res)
                                .await;
                        } else {
                            NamedFile::builder(fullpath).send(req.headers(), res).await;
                        }
                        return;
                    }
                }
            }
        }
    }

    res.status_code(StatusCode::BAD_REQUEST);
    let err = Json(ApiResult::<String>::error(
        400,
        "Invoke URI of Download Query".to_string().as_str(),
    ));
    res.render(err);
}
