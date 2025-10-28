use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    utils::{AsyncQuery, AsyncRequest, ChimesPerformanceInfo, ManageApiResult},
    Args,
};
use chimes_store_core::{
    config::auth::AuthorizationConfig,
    service::{
        invoker::{InvocationContext, JwtFromDepot},
        sched::SchedulerHolder,
        starter::MxStoreService,
    },
    utils::{redis::redis_get, ApiResult, ApiResult2},
};
use clap::{CommandFactory, Parser};
use rbatis::Page;
use salvo::{
    fs::NamedFile, handler, http::StatusCode, oapi::endpoint, writing::Json, Depot, Request,
    Response,
};
use serde_json::{json, Value};
use substring::Substring;
use sysinfo::System;
use uuid::Uuid;

#[handler]
pub async fn about_info(
    _depot: &mut Depot,
    _req: &mut Request,
) -> Json<ManageApiResult<Option<Value>>> {
    let mut aboutmap = HashMap::new();
    let cmd = Args::command();
    aboutmap.insert(
        "bin_name".to_owned(),
        cmd.get_bin_name().unwrap_or_default().to_owned(),
    );
    aboutmap.insert("name".to_owned(), cmd.get_name().to_owned());
    aboutmap.insert(
        "version".to_owned(),
        cmd.get_version().unwrap_or_default().to_owned(),
    );
    aboutmap.insert(
        "long_version".to_owned(),
        cmd.get_long_version().unwrap_or_default().to_owned(),
    );
    aboutmap.insert(
        "author".to_owned(),
        cmd.get_author().unwrap_or_default().to_owned(),
    );
    aboutmap.insert("display_name".to_owned(), "GrowthStore".to_string());
    aboutmap.insert(
        "about".to_owned(),
        cmd.get_about().unwrap_or_default().to_string(),
    );
    aboutmap.insert(
        "long_about".to_owned(),
        cmd.get_long_about().unwrap_or_default().to_string(),
    );

    // os, arch, memory, cores
    let mut sys = System::new_all();
    sys.refresh_all();

    aboutmap.insert("arch".to_owned(), System::cpu_arch());
    aboutmap.insert("os".to_owned(), std::env::consts::OS.to_string());
    aboutmap.insert("family".to_owned(), std::env::consts::FAMILY.to_string());
    aboutmap.insert(
        "os_version".to_owned(),
        System::os_version().unwrap_or_default(),
    );
    aboutmap.insert(
        "os_long_version".to_owned(),
        System::long_os_version().unwrap_or_default(),
    );
    aboutmap.insert("home".to_owned(), System::host_name().unwrap_or_default());
    aboutmap.insert("distribution_id".to_owned(), System::distribution_id());
    aboutmap.insert(
        "kernel_version".to_owned(),
        System::kernel_version().unwrap_or_default(),
    );
    aboutmap.insert(
        "kernel_long_version".to_owned(),
        System::kernel_long_version(),
    );

    aboutmap.insert(
        "open_file_limits".to_owned(),
        format!("{}", System::open_files_limit().unwrap_or_default()),
    );

    if let Ok(perf) = ChimesPerformanceInfo::get_performance_info() {
        aboutmap.insert("cpu_cores".to_owned(), format!("{}", perf.cpu_cores));
        aboutmap.insert("total_memory".to_owned(), format!("{}", perf.memory_total));
        aboutmap.insert(
            "virtual_memory".to_owned(),
            format!("{}", perf.memory_virtual),
        );
    }

    let args = Args::parse();
    let features = args.features.features.join(",");
    aboutmap.insert("features".to_owned(), features);
    Json(ManageApiResult::ok(Some(json!(aboutmap))))
}

#[endpoint]
pub async fn common_invoke_option(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ManageApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let cond = req.parse_body::<Value>().await.unwrap_or_default();

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
                    Err(err) => Json(ManageApiResult::error(500, format!("{err}").as_str())),
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
    let cond = req.parse_body::<Value>().await.unwrap_or_default();

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
                log::info!("full_invoke_uri: {full_invoke_uri}");
                let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));

                match MxStoreService::invoke_return_vec(full_invoke_uri, ctx, vecparams).await {
                    Ok(rs) => Json(ManageApiResult::ok(rs)),
                    Err(err) => Json(ManageApiResult::error(500, format!("{err}").as_str())),
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
    let cond = req.parse_body::<Value>().await.unwrap_or_default();

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

                match MxStoreService::invoke_return_page(full_invoke_uri, ctx, vecparams).await {
                    Ok(rs) => Json(ApiResult2::ok(rs)),
                    Err(err) => Json(ApiResult2::error(500, format!("{err}").as_str())),
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
pub async fn raw_file_send(_depot: &mut Depot, req: &mut Request, res: &mut Response) {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let _token = req.query::<String>("_token").unwrap_or_default();
    let ns = req.param::<String>("ns").unwrap_or_default();
    let path = req.uri().path().to_string();

    let prefix = format!("/api/file/{}/raw/", ns.clone());

    // log::warn!("Prefix: {prefix}");
    // log::warn!("Fullpath: {path}");

    let str_endpath = if path.starts_with(&prefix) {
        path.substring(prefix.len(), path.len())
    } else {
        res.status_code(StatusCode::BAD_REQUEST);
        let err = Json(ApiResult::<String>::error(
            400,
            "Invoke URI of Download Query".to_string().as_str(),
        ));
        res.render(err);
        return;
    };

    // log::warn!("Filepath: {str_endpath}");

    // should check the token by give query
    if let Some(mss) = MxStoreService::get(&ns) {
        // let conf = mss.get_config();
        let fm = mss.get_filestore();
        let fullpath = fm.combine_fullpath(str_endpath);
        // log::warn!("Combined Fullpath: {:?}", fullpath.clone());
        NamedFile::builder(fullpath).send(req.headers(), res).await;
        return;
    }

    res.status_code(StatusCode::BAD_REQUEST);
    let err = Json(ApiResult::<String>::error(
        400,
        "Invoke URI of Download Query".to_string().as_str(),
    ));
    res.render(err);
}

#[handler]
pub async fn common_file_send(depot: &mut Depot, req: &mut Request, res: &mut Response) {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let file_id = req.param::<String>("file_id").unwrap_or_default();
    let ns = req.param::<String>("ns").unwrap_or_default();

    if let Some(mss) = MxStoreService::get(&ns) {
        let conf = mss.get_config();
        let fm = mss.get_filestore();
        if let Some(query) = conf.download_query {
            let file_name_field = conf.download_file_name.unwrap_or("file_name".to_owned());
            let file_path_field = conf.download_file_path.unwrap_or("file_path".to_owned());
            let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
            let ret: Option<Value> =
                match MxStoreService::invoke_return_one(query, ctx, vec![Value::String(file_id)])
                    .await
                {
                    Ok(rs) => rs,
                    Err(err) => {
                        log::info!("find file error {err}");
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

#[endpoint]
pub async fn fetch_mcp_tools(
    _depot: &mut Depot,
    req: &mut Request,
) -> Json<ManageApiResult<Vec<Value>>> {
    let ns = req.query::<String>("ns").unwrap_or_default();
    if let Some(mx) = MxStoreService::get(&ns) {
        match mx.get_mcp_tools() {
            Ok(rs) => Json(ManageApiResult::ok(rs)),
            Err(err) => Json(ManageApiResult::error(500, format!("{err}").as_str())),
        }
    } else {
        Json(ManageApiResult::error(404, "Not Found"))
    }
}

#[endpoint]
pub async fn async_invoke_option(
    _depot: &mut Depot,
    req: &mut Request,
) -> Json<ManageApiResult<Option<Value>>> {
    // let params = req.parse_params::<Value>().expect("unexpect format");
    let mut cond = match req.parse_body::<AsyncRequest>().await {
        Ok(v) => v,
        Err(err) => {
            return Json(ManageApiResult::error(
                500,
                &format!("deserialize the request body with error {err}"),
            ));
        }
    };

    let jobid = if let Some(uid) = cond.uuid.clone() {
        if uid.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            uid.clone()
        }
    } else {
        Uuid::new_v4().to_string()
    };

    // TODO: 需要增加调用该接口的权限处理
    //       首先是调用该接口（async_invoke_option）必须要登录
    //       其次是执行cond.uri需要限定在对应的用户权限上
    //       最后是，用户信息需要向下传递
    cond.uuid = Some(jobid.clone());
    SchedulerHolder::get().addjob_delay(&jobid, "", None, Some(5), Box::new(cond));

    Json(ManageApiResult::ok(Some(Value::String(jobid))))
}

#[endpoint]
pub async fn async_query_result(
    _depot: &mut Depot,
    req: &mut Request,
) -> Json<ManageApiResult<Option<Value>>> {
    let method = req.method().as_str().to_uppercase();

    let cond = match method.as_str() {
        "GET" => match req.parse_queries::<AsyncQuery>() {
            Ok(v) => v,
            Err(err) => {
                return Json(ManageApiResult::error(
                    500,
                    &format!("could not parse the request query with error {err}"),
                ));
            }
        },
        "POST" | "PUT" => match req.parse_body::<AsyncQuery>().await {
            Ok(v) => v,
            Err(err) => {
                return Json(ManageApiResult::error(
                    500,
                    &format!("deserialize the request body with error {err}"),
                ));
            }
        },
        _ => {
            return Json(ManageApiResult::error(400, "Unsupport request"));
        }
    };

    let key = format!("{}-{}", cond.namespace, cond.uuid);
    match redis_get(&cond.namespace, &key) {
        Ok(res) => {
            match res {
                Some(rt) => match serde_json::from_str::<Value>(&rt) {
                    Ok(val) => Json(ManageApiResult::ok(Some(val))),
                    Err(err) => Json(ManageApiResult::error(
                        500,
                        &format!("could not deserialize the result {err}"),
                    )),
                },
                None => {
                    // check the result was generated
                    let cond_uuid = cond.uuid.clone();
                    if let Some(sched_state_uri) = AuthorizationConfig::get().schedule_state_uri {
                        let cond_arg = json!({
                            "and": vec![json!({"field": "uuid",  "op": "=", "value": cond_uuid})]
                        });
                        let full_uri = format!("{sched_state_uri}#find_one");
                        let ctx = Arc::new(Mutex::new(InvocationContext::new()));
                        match MxStoreService::invoke_return_one(
                            full_uri.clone(),
                            ctx,
                            vec![cond_arg],
                        )
                        .await
                        {
                            Ok(ts) => {
                                if ts.is_some() {
                                    Json(ManageApiResult::ok(None))
                                } else {
                                    Json(ManageApiResult::error(
                                        404,
                                        &format!("Task {cond_uuid} have not completed."),
                                    ))
                                }
                            }
                            Err(err) => {
                                log::info!(
                                    "Could not execute the {full_uri} . By the error is {err}"
                                );
                                Json(ManageApiResult::ok(None))
                            }
                        }
                    } else {
                        Json(ManageApiResult::ok(None))
                    }
                }
            }
        }
        Err(err) => Json(ManageApiResult::error(
            500,
            &format!("could not get the result {err}"),
        )),
    }
}
