use std::{fs, path::Path, str::FromStr, time::Duration};

use super::{AuthResponse, FunctionRegistry};
use crate::{
    config::{Config, ManagerAccountConfig, Plugin, WebConfig},
    manager::{ManagementRequest, ManagementState},
    salvo_main::JwtClaims,
    utils::{
        naming_property, zip::{check_zip_match_archive, create_zip_file, extract_zip_file}, AppConfig, ManageApiResult
    },
};
use async_std::{path::PathBuf, stream::StreamExt};
use chimes_store_core::{service::{invoker::JwtFromDepot, sdk::{InvokeUri, MxProbeService}, starter::save_config}, utils::global_data::{rsa_decrypt_with_private_key, rsa_encrypt_with_public_key}};
use chimes_store_core::service::starter::MxStoreService;
use chimes_store_core::utils::ApiResult;
use chimes_store_core::{
    config::{
        auth::AuthorizationConfig, Column, PluginConfig, QueryObject, ServerConfig, StoreObject,
        StoreServiceConfig,
    },
    service::{invoker::InvocationContext, script::ExtensionRegistry}
};
use chimes_store_dbs::docs::ToOpenApiDoc;
use itertools::Itertools;
use jsonwebtoken::EncodingKey;
use salvo::{
    fs::NamedFile, http::cookie::time::OffsetDateTime, oapi::extract::JsonBody, prelude::*,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use substring::Substring;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Debug, ToParameters, ToSchema)]
#[salvo(extract(
    default_source(from = "query"),
    default_source(from = "param"),
    default_source(from = "body"),
))]
struct SigninRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, ToParameters, ToSchema)]
#[salvo(extract(
    default_source(from = "query"),
    default_source(from = "param"),
    default_source(from = "body"),
))]
struct ChangePasswordRequest {
    pub username: String,
    pub password: String,
    pub new_password: String,
}


#[derive(Serialize, Deserialize, Debug, Default, ToParameters, ToSchema)]
#[salvo(extract(
    default_source(from = "query"),
    default_source(from = "param"),
    default_source(from = "body"),
))]
struct UserInfoResponse {
    pub username: String,
    pub fullname: Option<String>,
    pub avatar: Option<String>,
    pub id: String,
}

fn password_validate(username: &str, pt: &str, conf: &Config) -> bool {
    let maes = ManagerAccountConfig::get_managers();
    let pure = if pt.starts_with("rsa:") {
        let m = pt.substring(4, pt.len());
        rsa_decrypt_with_private_key(m, &conf.rsa_private_key.clone().unwrap_or_default())
    } else {
        Some(pt.to_owned())
    };
    
    if pure.is_none() {
        return false;
    }

    if let Some(ma) = maes.into_iter().filter(|p| p.username.to_lowercase() == username.to_owned().to_lowercase()).last() {
        let mapwd = ma.credentials.clone();
        let mapure = if mapwd.starts_with("rsa:") {
            let mc = mapwd.substring(4, mapwd.len());
            rsa_decrypt_with_private_key(mc, &conf.rsa_private_key.clone().unwrap_or_default())
        } else {
            Some(mapwd)
        };

        if mapure.is_none() {
            false
        } else {
            pure == mapure
        }
    } else {
        false
    }
}

/**
 * summary: 执行开发者的登录操作
 * description: 开发者需要提供其用户名以及密码进行登录
 */
#[endpoint]
pub async fn signin(
    depot: &mut Depot,
    req: JsonBody<SigninRequest>,
) -> Json<ManageApiResult<AuthResponse>> {
    let config: &Config = depot.get::<Config>("config").unwrap();

    let auth = req.0;
    if password_validate(&auth.username, &auth.password, config) {
        let exp = OffsetDateTime::now_utc()
            + Duration::from_secs(config.listen.expire_sec.unwrap_or(86400u32) as u64);
        let claim = JwtClaims {
            sid: auth.username,
            exp: exp.unix_timestamp(),
        };
        match jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claim,
            &EncodingKey::from_secret(config.listen.slot.clone().as_bytes()),
        ) {
            Ok(token) => Json(ManageApiResult::ok(AuthResponse {
                token: Some(token),
                expired: config.listen.expire_sec.unwrap_or(1800000),
            })),
            Err(err) => {
                log::info!("Unable to encoding jwt_token: {}", err);
                Json(ManageApiResult::<AuthResponse>::error(
                    StatusCode::BAD_REQUEST.as_u16() as i32,
                    "Bad Request",
                ))
            }
        }
    } else {
        Json(ManageApiResult::<AuthResponse>::error(
            StatusCode::FORBIDDEN.as_u16() as i32,
            "Username was not found or Password wrong",
        ))
    }
}

#[endpoint]
pub async fn change_pwd(
    depot: &mut Depot,
    req: JsonBody<ChangePasswordRequest>,
) -> Json<ManageApiResult<AuthResponse>> {
    let config: &Config = depot.get::<Config>("config").unwrap();
    let auth = req.0;
    if password_validate(&auth.username, &auth.password, config) {
        // update the config's manageraccount
        let mtx = ManagerAccountConfig::get_managers();
        if  let Some(key) = config.rsa_public_key.clone() {
            let cp = mtx.iter().map(|p| {
                let mut f = p.clone();
                if f.username.to_lowercase() == auth.username.to_lowercase() {
                    if let Some(encpwd) = rsa_encrypt_with_public_key(&auth.new_password.clone(), &key) {
                        f.credentials = format!("rsa:{}", encpwd);
                    } else {
                        f.credentials.clone_from(&auth.new_password);
                    }
                }
                f
            }).collect_vec();
            ManagerAccountConfig::update(cp.clone());
            let mut conf = config.clone();
            conf.managers = cp;
            // 
            if let Ok(conf_path) = PathBuf::from_str(&MxStoreService::get_config_path()) {
                if let Err(err) = save_config(&conf, conf_path.join("Config.toml")) {
                    log::info!("Update configuration file failed {err}");
                    Json(ManageApiResult::<AuthResponse>::error(
                        StatusCode::FORBIDDEN.as_u16() as i32,
                        "Username was not found or Password wrong",
                    ))
                } else {
                    Json(ManageApiResult::<AuthResponse>::ok(
                        AuthResponse {
                            token: None,
                            expired: 0
                        }
                    ))
                }
            } else {
                Json(ManageApiResult::<AuthResponse>::error(
                    StatusCode::SERVICE_UNAVAILABLE.as_u16() as i32,
                    "config_path error",
                ))                
            }
        } else {
            Json(ManageApiResult::<AuthResponse>::error(
                StatusCode::FORBIDDEN.as_u16() as i32,
                "Username was not found or Password wrong",
            ))
        }
    } else {
        Json(ManageApiResult::<AuthResponse>::error(
            StatusCode::FORBIDDEN.as_u16() as i32,
            "Username was not found or Password wrong",
        ))
    }
}

#[handler]
pub async fn userinfo(depot: &mut Depot, _: &mut Request) -> Json<ApiResult<UserInfoResponse>> {
    let config: &Config = depot.get::<Config>("config").unwrap();

    match depot.jwt_auth_state() {
        JwtAuthState::Authorized => {
            let data = depot.jwt_auth_data::<JwtClaims>().unwrap();
            let m = config
                .managers
                .clone()
                .into_iter()
                .filter(|p| p.username == data.claims.sid)
                .map(|f| UserInfoResponse {
                    username: f.username.clone(),
                    avatar: f.avatar.clone(),
                    fullname: f.full_name.clone(),
                    id: f.username.clone(),
                })
                .last()
                .unwrap_or(UserInfoResponse::default());

            Json(ApiResult::<UserInfoResponse>::ok(m))
        }
        JwtAuthState::Unauthorized => Json(ApiResult::<UserInfoResponse>::error(
            StatusCode::UNAUTHORIZED.as_u16() as i32,
            "Unauthorized",
        )),
        JwtAuthState::Forbidden => Json(ApiResult::<UserInfoResponse>::error(
            StatusCode::FORBIDDEN.as_u16() as i32,
            "Forbidden",
        )),
    }
}

#[handler]
pub async fn reload(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Value>> {
    let ManagementState { sender, .. } = depot.obtain::<ManagementState>().unwrap();

    let reload_path = req.query("_reload").unwrap_or("*");
    let (mngr_sender, mngr_receiver) = flume::unbounded();
    // let request = Box::new(request.into());
    let _ = sender.send(ManagementRequest::Reload(
        mngr_sender,
        vec![reload_path.to_owned()],
    ));

    let res: Option<Value> = mngr_receiver.into_stream().next().await;

    Json(ApiResult::ok(json!({"result": res})))
}

#[handler]
pub async fn save(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Value>> {
    let ManagementState { sender, .. } = depot.obtain::<ManagementState>().unwrap();

    let save_ = req.query("_save").unwrap_or("*");
    let (mngr_sender, mngr_receiver) = flume::unbounded();
    // let request = Box::new(request.into());
    let _ = sender.send(ManagementRequest::Save(mngr_sender, vec![save_.to_owned()]));

    let res: Option<Value> = mngr_receiver.into_stream().next().await;

    Json(ApiResult::ok(json!({"result": res})))
}

#[handler]
pub async fn auth_conf(
    depot: &mut Depot,
    _req: &mut Request,
) -> Json<ApiResult<AuthorizationConfig>> {
    let config: &Config = depot.get::<Config>("config").unwrap();
    let path = config.web.config_path.clone().join("Authorization.toml");
    log::info!("Loads authorization: {:?}", path.clone());
    match AuthorizationConfig::load(path) {
        Ok(auth) => Json(ApiResult::ok(auth)),
        Err(err) => Json(ApiResult::error(400, &format!("error on {}", err))),
    }
}

#[handler]
pub async fn save_auth_conf(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<AuthorizationConfig>> {
    let config: &Config = depot.get::<Config>("config").unwrap();
    let path = config.web.config_path.clone().join("Authorization.toml");
    log::info!("Save authorization: {:?}", path.clone());
    match req.parse_body::<AuthorizationConfig>().await {
        Ok(authconf) => match authconf.save(path.clone()) {
            Ok(_) => {
                if let Err(err) = AuthorizationConfig::load(path) {
                    log::info!("Reload the Authorization.toml failed after saved. {}", err);
                }
                Json(ApiResult::ok(authconf))
            }
            Err(err) => Json(ApiResult::error(500, &format!("error on {}", err))),
        },
        Err(err) => Json(ApiResult::error(
            400,
            &format!("error on parse body {}", err),
        )),
    }
}

#[handler]
pub async fn probe_schema(_depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Value>> {
    let schema_ = req.query("schema").unwrap_or("");
    let ns_ = req.query("ns").unwrap_or("");
    match MxStoreService::get(ns_) {
        Some(ts) => match ts.probe_schema(schema_).await {
            Ok(rs) => Json(ApiResult::ok(json!(rs))),
            Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
        },
        None => Json(ApiResult::error(404, "Service Not-Found ")),
    }
}

#[handler]
pub async fn probe_table(_depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Value>> {
    // let ManagementState { sender, .. } = depot.obtain::<ManagementState>().unwrap();

    let schema_ = req.query("schema").unwrap_or("");
    let table_ = req.query("table").unwrap_or("");
    let ns_ = req.query("ns").unwrap_or("");
    let rule_ = req.query("rule").unwrap_or("none");

    match MxStoreService::get(ns_) {
        Some(ts) => match ts.probe_table(schema_, table_).await {
            Ok(rs) => {
                let crs = rs.iter().map(|f| {
                    let mut c = f.clone();
                    c.prop_name = naming_property(f, rule_);
                    c
                }).collect_vec();
                Json(ApiResult::ok(json!(crs)))
            },
            Err(err) => Json(ApiResult::error(500, format!("{}", err).as_str())),
        },
        None => Json(ApiResult::error(404, "Service Not-Found ")),
    }
}

/**
 * 根据前端传入的参数，来进行StoreService的生成
 * 目前，只支持基于数据库的CRUD
 * 未来，可以根据插件的方式，来生成多种多样的数据格式
 * - ES
 * - Redis
 * - MonogoDB
 */
#[handler]
pub async fn generate(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Value>> {
    let ManagementState { sender, .. } = depot.obtain::<ManagementState>().unwrap();

    let ns_ = req.query::<String>("ns").unwrap_or_default();
    let sch = req.query::<String>("schema").unwrap_or_default();
    let rule = req.query::<String>("rule").unwrap_or("none".to_owned());

    let sto = req
        .parse_body::<Vec<StoreObject>>()
        .await
        .expect("Body format unexcept");

    log::info!("Rec: {:?}", sto.clone());

    let newsto = if let Some(tss) = MxStoreService::get(&ns_) {
        let mut sxsto = vec![];
        for mut s in sto {
            if s.object_type.is_empty() {
                if let Ok(Some(tbl)) = tss.probe_one_table(&sch, &s.object_name).await {
                    s.object_type = tbl.table_type.clone().unwrap_or_default();
                }
            }
            let mut keys = s.get_key_columns().iter().map(|f| f.field_name.clone()).collect_vec();
            
            if keys.is_empty() {
                if let Ok(kss) = tss.probe_table_keys(&sch, &s.object_name).await {
                    keys = kss
                        .into_iter()
                        .map(|f| f.column_name.unwrap())
                        .collect_vec();
                }
            }
            if s.fields.is_empty() {
                if let Ok(vss) = tss.probe_table(&sch, &s.object_name).await {
                    s.fields = vss
                        .into_iter()
                        .map(|f| {
                            let prop = naming_property(&f, &rule);
                            let mut c: Column = f.into();
                            c.pkey = keys.contains(&c.field_name);
                            c.prop_name = prop;
                            c
                        })
                        .collect::<Vec<Column>>();
                };
            }

            sxsto.push(s);
        }
        sxsto
    } else {
        sto
    };

    MxStoreService::update_service_add_objects(&ns_, &newsto);

    let (mngr_sender, mngr_receiver) = flume::unbounded();

    let _ = sender.send(ManagementRequest::Save(mngr_sender, vec![ns_]));

    let res: Option<Value> = mngr_receiver.into_stream().next().await;

    Json(ApiResult::ok(json!({"result": res})))
}

#[handler]
pub async fn fetch_namespaces(_depot: &mut Depot, _req: &mut Request) -> Json<ApiResult<Value>> {
    let nss = MxStoreService::get_namespaces();

    // let vt = FunctionRegistry::get_all_functions();

    let ret = nss.into_iter().map(|f| json!({"id": f, "label": f, "icon": "Present", "children": FunctionRegistry::get_active_functions(&f).into_iter().map(|l| {
        let mut cl = l.clone();
        cl.id = format!("{}:{}", f, l.id);

        let children: Vec<Value> = if let Some(tss) = MxStoreService::get(&f) {
            let conf = tss.get_config();
            match cl.name.as_str() {
                "Object" => {
                    conf.objects.into_iter().map(|c| json!({"id": format!("{}:{}", cl.id.clone(), c.name.clone()), "label": c.name.clone(), "icon": cl.icon.clone(), "leaf": true})).collect_vec()
                },
                "Query" => {
                    conf.querys.into_iter().map(|c| json!({"id": format!("{}:{}", cl.id.clone(), c.name.clone()), "label": c.name.clone(), "icon": cl.icon.clone(), "leaf": true})).collect_vec()
                },
                "Plugin" => {
                    let cxid =  cl.id.clone().replace(":_plugin", "");
                    conf.plugins.into_iter().map(|c| json!({"id": format!("{}:{}:{}", cxid, c.protocol.clone(), c.name.clone()), "label": c.name.clone(), "icon": cl.icon.clone(), "leaf": true})).collect_vec()
                },
                _ => {
                    vec![]
                }
            }
        } else {
            vec![]
        };

        json!({"id": cl.id, "label": cl.label, "icon": cl.icon.clone(), "children": children })
    }).collect_vec()})).collect_vec();

    Json(ApiResult::ok(json!(ret)))
}

#[handler]
pub async fn namespace_config(_depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Value>> {
    let ns_ = req.query::<String>("ns").unwrap_or_default();

    if ns_.is_empty() {
        return Json(ApiResult::error(404, "Parameter ns was not suppliered."));
    }

    match MxStoreService::get(&ns_) {
        Some(tss) => {
            let conf = tss.get_config();
            Json(ApiResult::ok(json!(conf)))
        }
        None => Json(ApiResult::error(
            404,
            format!("{ns_} is not found.").as_str(),
        )),
    }
}

#[handler]
pub async fn update(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Value>> {
    let ns_ = req.query::<String>("ns").unwrap_or_default();
    let ty_ = req.query::<String>("type").unwrap_or_default();

    if ns_.is_empty() {
        return Json(ApiResult::error(404, "Parameter ns was not suppliered."));
    }

    match MxStoreService::get(&ns_) {
        Some(_) => {
            match ty_.as_str() {
                "object" => match req.parse_body::<Vec<StoreObject>>().await {
                    Ok(sts) => {
                        MxStoreService::update_service_add_objects(&ns_, &sts);
                    }
                    Err(err) => {
                        log::info!("Parse the request body error: {:?}", err);
                        return Json(ApiResult::error(
                            400,
                            "Could not parse request body as StoreObject."
                                .to_string()
                                .as_str(),
                        ));
                    }
                },
                "query" => {
                    if let Ok(sts) = req.parse_body::<Vec<QueryObject>>().await {
                        MxStoreService::update_service_add_query(&ns_, &sts);
                    } else {
                        return Json(ApiResult::error(
                            400,
                            "Could not parse request body as QueryObject."
                                .to_string()
                                .as_str(),
                        ));
                    }
                }
                "plugin" => match req.parse_body::<Vec<PluginConfig>>().await {
                    Ok(sts) => {
                        MxStoreService::update_service_add_plugin(&ns_, &sts);
                    }
                    Err(err) => {
                        log::info!("Could not parse to plugin-clonfig. {:?}", err);
                        return Json(ApiResult::error(
                            400,
                            "Could not parse request body as PluginConfig.",
                        ));
                    }
                },
                "config" => match req.parse_body::<StoreServiceConfig>().await {
                    Ok(sts) => {
                        let path = MxStoreService::get_model_path();
                        if let Err(err) = MxStoreService::update_and_save_namespace(&sts, path) {
                            return Json(ApiResult::error(
                                500,
                                &format!("Save the Namesapce config failed. {:?}", err),
                            ));
                        }
                    }
                    Err(err) => {
                        log::info!("Could not parse to plugin-clonfig. {:?}", err);
                        return Json(ApiResult::error(
                            400,
                            "Could not parse request body as PluginConfig.",
                        ));
                    }
                },
                _ => {
                    return Json(ApiResult::error(
                        405,
                        format!("{ty_} is support now.").as_str(),
                    ));
                }
            }

            let ManagementState { sender, .. } = depot.obtain::<ManagementState>().unwrap();
            let (mngr_sender, mngr_receiver) = flume::unbounded();

            let _ = sender.send(ManagementRequest::Save(mngr_sender, vec![ns_]));

            let res: Option<Value> = mngr_receiver.into_stream().next().await;

            Json(ApiResult::ok(json!({"result": res})))
        }
        None => Json(ApiResult::error(
            404,
            format!("{ns_} is not found.").as_str(),
        )),
    }
}

#[handler]
pub async fn create_namespace(_depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Value>> {
    let ns_ = req.query::<String>("ns").unwrap_or_default();
    let ty_ = req.query::<String>("type").unwrap_or_default();

    if ns_.is_empty() {
        return Json(ApiResult::error(404, "Parameter ns was not suppliered."));
    }

    match MxStoreService::get(&ns_) {
        Some(_) => Json(ApiResult::error(450, format!("{ns_} exists.").as_str())),
        None => {
            // Save the config into new file
            match ty_.as_str() {
                "config" => match req.parse_body::<StoreServiceConfig>().await {
                    Ok(sts) => {
                        let path = MxStoreService::get_model_path();
                        let sts = if sts.filename.ends_with(".toml") {
                            sts
                        } else {
                            let mut cts = sts.clone();
                            cts.filename.push_str(".toml");
                            cts
                        };
                        log::info!("Save to {path} with {}", sts.filename);
                        match MxStoreService::update_and_save_namespace(&sts, path) {
                            Ok(_) => Json(ApiResult::ok(Value::String("OK".to_string()))),
                            Err(err) => Json(ApiResult::error(
                                500,
                                &format!("error to save file {:?}", err),
                            )),
                        }
                    }
                    Err(err) => {
                        log::info!("Could not parse to plugin-clonfig. {:?}", err);
                        Json(ApiResult::error(
                            400,
                            "Could not parse request body as PluginConfig.",
                        ))
                    }
                },
                _ => Json(ApiResult::error(
                    400,
                    "Could not parse request body as StoreServiceConfig.",
                )),
            }
        }
    }
}

/**
 * 删除一个指定我的对象
 * 参数： ns,指定的命名空间
 *       type,被删除的类型
 */
#[handler]
pub async fn delete(depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Value>> {
    let ns_: String = req.query::<String>("ns").unwrap_or_default();
    let ty_ = req.query::<String>("type").unwrap_or_default();

    if ns_.is_empty() {
        return Json(ApiResult::error(404, "Parameter ns was not suppliered."));
    }

    match MxStoreService::get(&ns_) {
        Some(_) => {
            match ty_.as_str() {
                "object" => {
                    if let Ok(sts) = req.parse_body::<Vec<String>>().await {
                        MxStoreService::update_service_delete_objects(&ns_, &sts);
                    } else {
                        return Json(ApiResult::error(
                            400,
                            "Could not parse request body as StoreObject."
                                .to_string()
                                .as_str(),
                        ));
                    }
                }
                "query" => {
                    if let Ok(sts) = req.parse_body::<Vec<String>>().await {
                        MxStoreService::update_service_delete_query(&ns_, &sts);
                    } else {
                        return Json(ApiResult::error(
                            400,
                            "Could not parse request body as QueryObject."
                                .to_string()
                                .as_str(),
                        ));
                    }
                }
                "plugin" => {
                    if let Ok(sts) = req.parse_body::<Vec<String>>().await {
                        MxStoreService::update_service_delete_plugin(&ns_, &sts);
                    } else {
                        return Json(ApiResult::error(
                            400,
                            "Could not parse request body as Plugin."
                                .to_string()
                                .as_str(),
                        ));
                    }
                }
                "namespace" => {
                    if let Ok(_) = req.parse_body::<Vec<String>>().await {
                        MxStoreService::remove_namespace(&ns_);
                        // MxStoreService::update_service_delete_plugin(&ns_, &sts);
                    } else {
                        return Json(ApiResult::error(
                            400,
                            "Could not parse request body as Plugin."
                                .to_string()
                                .as_str(),
                        ));
                    }
                }
                _ => {
                    return Json(ApiResult::error(
                        405,
                        format!("{ty_} is support now.").as_str(),
                    ));
                }
            }

            let ManagementState { sender, .. } = depot.obtain::<ManagementState>().unwrap();
            let (mngr_sender, mngr_receiver) = flume::unbounded();

            let _ = sender.send(ManagementRequest::Save(mngr_sender, vec![ns_]));

            let res: Option<Value> = mngr_receiver.into_stream().next().await;

            Json(ApiResult::ok(json!({"result": res})))
        }
        None => Json(ApiResult::error(
            404,
            format!("{ns_} is not found.").as_str(),
        )),
    }
}

#[handler]
pub async fn plugin_list(depot: &mut Depot, _req: &mut Request) -> Json<ApiResult<Vec<Plugin>>> {
    match depot.get::<Config>("config") {
        Ok(conf) => Json(ApiResult::ok(conf.plugins.clone())),
        Err(_) => Json(ApiResult::error(
            404,
            "Plugin was found.".to_string().as_str(),
        )),
    }
}

#[endpoint]
pub async fn extension_list(
    _depot: &mut Depot,
    _req: &mut Request,
) -> Json<ManageApiResult<Vec<Value>>> {
    Json(ManageApiResult::ok(
        ExtensionRegistry::get_extensions()
            .into_iter()
            .map(|(lang, fullname)| json!({"lang": lang, "description": fullname}))
            .collect_vec(),
    ))
}

#[handler]
pub async fn plugin_add(depot: &mut Depot, _req: &mut Request) -> Json<ApiResult<Vec<Plugin>>> {
    match depot.get::<Config>("config") {
        Ok(conf) => Json(ApiResult::ok(conf.plugins.clone())),
        Err(_) => Json(ApiResult::error(
            404,
            "Plugin was found.".to_string().as_str(),
        )),
    }
}

#[handler]
pub async fn config_get(_depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Option<Value>>> {
    let schema_ = req.query("schema").unwrap_or("").to_string(); // 对象协议
    let ns_ = req.query("ns").unwrap_or("").to_string(); // namesapace
    let name_ = req.query("name").unwrap_or("").to_string(); // 对象名称0

    let ctx = Arc::new(Mutex::new(InvocationContext::new()));

    match MxStoreService::invoke_return_one(
        format!("{}://{}/{}#get_config", schema_, ns_, name_),
        ctx,
        vec![],
    )
    .await
    {
        Ok(val) => Json(ApiResult::ok(val)),
        Err(_err) => Json(ApiResult::error(404, "Service Not-Found ")),
    }

}

#[endpoint]
pub async fn config_save(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ManageApiResult<Option<Value>>> {
    // let ManagementState { sender, .. } = depot.obtain::<ManagementState>().unwrap();
    let webconfig: WebConfig = match depot.get::<Config>("config") {
        Ok(conf) => conf.web.clone(),
        Err(_) => AppConfig::config(),
    };

    let body_val = match req.parse_body::<Value>().await {
        Ok(v) => v,
        Err(err) => {
            log::debug!("Parse the body for save_config with errror {:?}", err);
            return Json(ManageApiResult::error(500, "Could not parse the body"));
        }
    };
    let schema_ = req.query("schema").unwrap_or("").to_string();
    let name_ = req.query("name").unwrap_or("").to_string();
    let ns_ = req.query("ns").unwrap_or("").to_string();
    let config_path = webconfig.model_path.to_string_lossy().to_string();

    let ctx = Arc::new(Mutex::new(InvocationContext::new()));

    match MxStoreService::invoke_return_one(
        format!("{}://{}/{}#save_config", schema_, ns_, name_),
        ctx,
        vec![body_val, json!({"model_path": config_path })],
    )
    .await
    {
        Ok(val) => Json(ManageApiResult::ok(val)),
        Err(err) => {
            log::warn!("Could not service this method {:?}", err);
            Json(ManageApiResult::error(500, "Service Not-Found"))
        }
    }

}

#[handler]
pub async fn metadata_get(depot: &mut Depot, req: &mut Request) -> Text<String> {
    let webconfig: WebConfig = match depot.get::<Config>("config") {
        Ok(conf) => conf.web.clone(),
        Err(_) => AppConfig::config(),
    };

    let ns = req.param::<String>("ns");
    let name = req.param::<String>("name");
    let schema = req.param::<String>("schema").unwrap();

    // TODO: should to replace to assets path
    let meta_path = match webconfig.config_path.canonicalize() {
        Ok(mp) => mp.join("../").join("metadata/").canonicalize().unwrap(),
        Err(_) => std::env::current_exe()
            .map(|p| p.join("../").canonicalize().unwrap())
            .unwrap_or(std::env::current_dir().unwrap())
            .join("metadata/"),
    };

    // 两套逻辑
    // object and query放在 metadata/object/<ns> 或 metadta/query/<ns> 目录下，
    // plugin的放在 metadata/<plugin>/schema.json 下，这个目录的schema.json文件是用于在配置界面中进行表单的显示
    let meta_file = if ns.is_some() && name.is_some() {
        meta_path
            .join(schema)
            .join(ns.unwrap_or_default())
            .join(format!("{}.json", name.unwrap_or_default()))
    } else {
        meta_path.join(schema).join("schema.json")
    };

    log::debug!("Read the file {} as metadata", meta_file.to_string_lossy());

    match std::fs::read_to_string(meta_file) {
        Ok(jt) => Text::Json(jt),
        Err(err) => {
            log::debug!("Could not read the file {:?}", err);
            Text::Json(json!(ApiResult::<Value>::ok(json!({}))).to_string())
        }
    }
}

#[handler]
pub async fn metadata_generate(
    _depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Option<Value>>> {
    let schema_ = req.query("schema").unwrap_or("").to_string(); // 对象协议
    let ns_ = req.query("ns").unwrap_or("").to_string(); // namesapace
    let name_ = req.query("name").unwrap_or("").to_string(); // 对象名称0

    let ctx = Arc::new(Mutex::new(InvocationContext::new()));

    match MxStoreService::invoke_return_one(
        format!("{}://{}/{}#get_config", schema_, ns_, name_),
        ctx,
        vec![],
    )
    .await
    {
        Ok(val) => Json(ApiResult::ok(val)),
        Err(_err) => Json(ApiResult::error(404, "Service Not-Found ")),
    }

}

#[handler]
pub async fn export_namespace(_depot: &mut Depot, req: &mut Request, res: &mut Response) {
    let ns = req.query("ns").unwrap_or("").to_string(); // 对象协议
    match MxStoreService::get(&ns) {
        Some(t) => {
            let mut files = vec![];
            let conf = t.get_config();

            files.push("models/".to_string());
            files.push(format!("models/{}", conf.filename));
            files.push(format!("models/{}/", conf.namespace));

            for pl in t.get_plugins() {
                files.push(format!("models/{}/{}", conf.namespace, pl.config));
            }
            let asset_path = MxStoreService::get_assets_path();
            let script_path = Path::new(&asset_path);
            let ns_scriptpath = script_path.join("scripts").join(conf.namespace);

            let walkdirs = walkdir::WalkDir::new(ns_scriptpath);
            let it = walkdirs.into_iter();
            it.into_iter().filter_map(|f| f.ok()).for_each(|e| {
                let path = e.path();
                if let Ok(strip) = path.strip_prefix(script_path) {
                    if let Some(strippath) = strip.to_str() {
                        if path.is_dir() {
                            files.push(format!("{}/", strippath.replace('\\', "/")));
                        } else {
                            files.push(strippath.replace('\\', "/"));
                        }
                    }
                }
            });

            let archive_path = Path::new(&asset_path).join("archives/");

            if let Err(err) = fs::create_dir_all(&archive_path) {
                log::debug!("create dir all failed {err}");
            }

            let archive_file = archive_path
                .join("archives/")
                .with_file_name(format!("{}.zip", ns));

            match create_zip_file(archive_file.clone(), &asset_path, &files) {
                Ok(_) => {
                    NamedFile::builder(archive_file)
                        .attached_name(format!("{}.zip", ns))
                        .send(req.headers(), res)
                        .await;
                }
                Err(err) => {
                    res.render(Json(ApiResult::<String>::error(500, &format!("{}", err))));
                }
            }
        }
        None => res.render(Json(ApiResult::<String>::error(404, "Namespace Not-Found"))),
    }
}

#[handler]
pub async fn restore_namespace(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Option<Value>>> {
    let ns = req.query::<String>("force").unwrap_or_default();
    let force = ns.to_lowercase() == *"true";
    if let Ok(form_data) = req.form_data().await {
        let files_vec = form_data
            .files
            .get_vec("file")
            .map(|f| f.to_owned())
            .unwrap_or(vec![]);
        for file in files_vec.clone() {
            match check_zip_match_archive(file.path(), MxStoreService::get_assets_path(), force) {
                Ok(t) => {
                    if !t {
                        return Json(ApiResult::<Option<Value>>::error(
                            506,
                            "Archive does not contains a valid format",
                        ));
                    }
                }
                Err(err) => {
                    log::info!("error on open archive {err}");
                    return Json(ApiResult::<Option<Value>>::error(
                        500,
                        &format!("Could not parse or extract the archive file err is {err}"),
                    ));
                }
            }
        }

        for file in files_vec {
            if let Err(err) =
                extract_zip_file(file.path(), MxStoreService::get_assets_path(), force)
            {
                log::info!("error on open archive {err}");
                return Json(ApiResult::<Option<Value>>::error(
                    500,
                    &format!(
                        "Could not parse or extract the archive file for extract, err is {}",
                        err
                    ),
                ));
            }
        }

        // reload all models
        let ManagementState { sender, .. } = depot.obtain::<ManagementState>().unwrap();

        let reload_path = req.query("_reload").unwrap_or("*");
        let (mngr_sender, mngr_receiver) = flume::unbounded();
        // let request = Box::new(request.into());
        let _ = sender.send(ManagementRequest::Reload(
            mngr_sender,
            vec![reload_path.to_owned()],
        ));

        let res: Option<Value> = mngr_receiver.into_stream().next().await;

        Json(ApiResult::ok(res))
    } else {
        return Json(ApiResult::<Option<Value>>::error(
            500,
            "Could not parse upload files",
        ));
    }
}

#[handler]
pub async fn metadata_openapi(depot: &mut Depot, req: &mut Request) -> Text<String> {
    let ns = req.param::<String>("ns").unwrap();
    let config: &Config = depot.get::<Config>("config").unwrap();

    let conf = ServerConfig {
        address: config.listen.ip.to_string(),
        port: config.listen.port as u64,
        version: Some("0.2.0".to_owned()),
    };

    match MxStoreService::get(&ns) {
        Some(tss) => {
            let doc = tss.to_openapi_doc(&conf);
            match serde_json::to_string(&doc) {
                Ok(text) => Text::Json(text),
                Err(err) => {
                    log::info!("could not generate api-docs {}", err);
                    Text::Json(json!({}).to_string())
                }
            }
        }
        None => Text::Json(json!({}).to_string()),
    }
}

#[handler]
pub async fn auth_roles(_depot: &mut Depot, _req: &mut Request) -> Json<ApiResult<Vec<String>>> {
    let authconf = AuthorizationConfig::get();
    let roles = authconf.get_role_name_presets();
    Json(ApiResult::ok(roles))
}

#[handler]
pub async fn fetch_plugin_name(_depot: &mut Depot, req: &mut Request) -> Json<ApiResult<Vec<String>>> {
    if let Ok(query) = req.parse_queries::<Value>() {
        let protocol = query.get("protocol").map(|f| f.as_str().unwrap_or_default().to_owned()).unwrap_or_default();
        let ns = query.get("ns").map(|f| f.as_str().unwrap_or_default().to_owned()).unwrap_or_default();
        if let Some(stt) = MxStoreService::get(&ns) {
            let plss = stt.get_plugin_config_by_protocol(&protocol);
            return Json(ApiResult::ok(plss.iter().map(|f| f.name.clone()).collect::<Vec<String>>()));
        }
    }

    Json(ApiResult::ok(vec![]))
}


#[handler]
pub async fn execute_common_management(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Option<Value>>> {
    let protocol = req.param::<String>("protocol").unwrap();
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let method = req.param::<String>("method").unwrap();
    let uri = format!("{protocol}://{ns}/{name}#{method}");
    let mut args = vec![];
    match req.parse_body::<Value>().await {
        Ok(tt) => {
            match tt.clone() {
                Value::Array(mut tms) => {
                    args.append(&mut tms);
                },
                Value::Object(_tm) => {
                    args.push(tt);
                }
                _ => {
                    return Json(ApiResult::error(
                        400,
                        "No payload provided",
                    ));
                }
            };
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
