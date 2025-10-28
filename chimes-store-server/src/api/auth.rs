use crate::auth_service::{find_app_secret_by_app_id, AuthAppSecuretLoadJob, AuthorizationService};
use crate::{config::Config, salvo_main::JwtClaims, utils::generate_rand_string};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use chimes_store_core::config::auth::{AuthorizationConfig, JwtUserClaims};
use chimes_store_core::service::invoker::{InvocationContext, JwtFromDepot};
use chimes_store_core::service::sched::SchedulerHolder;
use chimes_store_core::utils::{get_local_timestamp, GlobalConfig};
use chimes_store_core::utils::global_data::global_app_data_insert;
use chimes_store_core::utils::global_data::global_app_data_remove;
use chimes_store_core::utils::global_data::{global_app_data_get, rsa_decrypt_with_private_key};
use chimes_store_core::utils::ApiResult;
use chimes_store_dbs::utils::{aes_encrypt_text};
use chimes_store_utils::crypto::{hex_encode_string, hmac_sha256};
use itertools::Itertools;
use jsonwebtoken::EncodingKey;
use salvo::{http::cookie::time::OffsetDateTime, oapi::extract::JsonBody, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Debug, ToParameters, ToSchema)]
#[salvo(extract(
    default_source(from = "query"),
    default_source(from = "param"),
    default_source(from = "body"),
))]
struct AppKeyRequest {
    pub app_id: String,
    pub app_secret: String,
}

#[derive(Serialize, Deserialize, Debug, ToSchema, ToResponse)]
pub struct AuthResponse {
    pub token: Option<String>,
    pub expired: u32,
}


#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, ToSchema, ToResponse)]
struct InfoResponse {}

/// exchange the appkey and app_secret to authorization token.
#[handler]
pub fn exchange(depot: &mut Depot, req: JsonBody<AppKeyRequest>, res: &mut Response) {
    let config: &Config = depot.get::<Config>("config").unwrap();
    let auth = req.0;
    log::warn!("receive to {}", auth.app_secret.clone());
    let (app_secret, _time) = if let Some(secret) = rsa_decrypt_with_private_key(
        &auth.app_secret,
        &config.rsa_private_key.clone().unwrap_or_default(),
    ) {
        log::warn!("decode to {}", secret.clone());
        let twsecs = secret.split("##").collect_vec();
        if twsecs.len() > 1 {
            let sec = twsecs[0].to_string();
            let time = twsecs[1].to_string();
            match time.parse::<i64>() {
                Ok(t) => {
                    let now = get_local_timestamp() as i64;
                    let tm = now - t;
                    if tm.abs() > 30000 {
                        res.status_code(StatusCode::FORBIDDEN)
                            .render(Json(ApiResult::<AuthResponse>::error(
                                StatusCode::FORBIDDEN.as_u16() as i32,
                                "Invalid Timestamp",
                            )));
                        return;
                    }

                    (sec, t)
                }
                Err(_) => {
                    res.status_code(StatusCode::FORBIDDEN).render(Json(
                        ApiResult::<AuthResponse>::error(
                            StatusCode::FORBIDDEN.as_u16() as i32,
                            "Invalid AppId with AppSecret Pair",
                        ),
                    ));
                    return;
                }
            }
        } else {
            res.status_code(StatusCode::FORBIDDEN)
                .render(Json(ApiResult::<AuthResponse>::error(
                    StatusCode::FORBIDDEN.as_u16() as i32,
                    "Invalid AppId with AppSecret Pair",
                )));
            return;
        }
    } else {
        res.status_code(StatusCode::FORBIDDEN)
            .render(Json(ApiResult::<AuthResponse>::error(
                StatusCode::FORBIDDEN.as_u16() as i32,
                "Not Found AppId with AppSecret Pair",
            )));
        return;
    };

    if config
        .app_keys
        .clone()
        .into_iter()
        .any(|p| p.app_id == auth.app_id.clone() && p.secret_key == app_secret.clone())
    {
        let exp =
            OffsetDateTime::now_utc() + salvo::http::cookie::time::Duration::seconds(30000i64);
        let claim = JwtClaims {
            sid: auth.app_id,
            exp: exp.unix_timestamp(),
        };
        match jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claim,
            &EncodingKey::from_secret(config.listen.slot.clone().as_bytes()),
        ) {
            Ok(token) => {
                res.render(Json(ApiResult::ok(AuthResponse {
                    token: Some(token),
                    expired: 30000,
                })));
            }
            Err(err) => {
                log::info!("Unable to encoding jwt_token: {err}");
                res.status_code(StatusCode::BAD_REQUEST).render(Json(
                    ApiResult::<AuthResponse>::error(
                        StatusCode::BAD_REQUEST.as_u16() as i32,
                        "Bad Request",
                    ),
                ));
            }
        }
    } else {
        res.status_code(StatusCode::FORBIDDEN)
            .render(Json(ApiResult::<AuthResponse>::error(
                StatusCode::FORBIDDEN.as_u16() as i32,
                "Not Found AppId with AppSecret Pair",
            )));
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Debug, ToParameters, ToSchema)]
#[salvo(extract(
    default_source(from = "query"),
    default_source(from = "param"),
    default_source(from = "body"),
))]
pub struct UserAuthRequest {
    pub organization: Option<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
    pub extends: Option<Value>,
    pub captcha_code: Option<String>,
    pub captcha_id: Option<String>,
    pub sso_token: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug, ToParameters, ToSchema)]
pub struct UserAuthResponse {
    pub username: String,
    pub organization: Option<String>,
    pub detail: Value,
    pub state: i32,
    pub token: Option<String>,
    pub roles: Vec<String>,
}

unsafe impl Send for UserAuthRequest {}

unsafe impl Send for UserAuthResponse {}

unsafe impl Sync for UserAuthRequest {}

unsafe impl Sync for UserAuthResponse {}

#[derive(Clone, Default, Serialize, Deserialize, Debug, ToParameters, ToSchema)]
#[salvo(extract(
    default_source(from = "query"),
    default_source(from = "param"),
    default_source(from = "body"),
))]
pub struct ChangePasswordRequest {
    pub organization: Option<String>,
    pub username: String,
    pub credential: String,
    pub new_credential: String,
    pub captcha_code: Option<String>,
    pub captcha_id: Option<String>,
}

pub fn get_user_id_from_user(user: &Option<Value>, conf: &AuthorizationConfig) -> String {
    if let Some(user) = user {
        let user_id_field = conf.userid_field.clone().unwrap_or("user_id".to_owned());
        if let Some(mp) = user.as_object() {
            if let Some(t) = mp.get(&user_id_field) {
                return t.to_string();
            }
        }
    }
    String::new()
}


/**
 * 用户登录时的Captcha
 * 返回image，Base64编码
 */
#[handler]
#[fastrace::trace]
pub async fn get_auth_config(_depot: &mut Depot) -> Json<ApiResult<Value>> {
    let conf = GlobalConfig::config();
    let pkey = conf.rsa_password_public_key.clone().unwrap_or_default();
    let aeskey = conf.aes_encryption_key.clone().unwrap_or(generate_rand_string(32));
    let aessolt = conf.aes_encryption_solt.clone().unwrap_or(generate_rand_string(16));
    
    let encpkey = aes_encrypt_text(&pkey, &aeskey, &aessolt);
    Json(ApiResult::ok(json!({"private_key": encpkey})))
}

/**
 * 用户登录时的Captcha
 * 返回image，Base64编码
 */
#[handler]
#[fastrace::trace]
pub async fn user_auth_auth_code(_depot: &mut Depot) -> Json<ApiResult<Value>> {
    let png = captcha::Captcha::new()
        .add_chars(5)
        .apply_filter(captcha::filters::Noise::new(0.4))
        .view(180, 80)
        .as_tuple();
    match png {
        Some(st) => {
            let basestr = STANDARD_NO_PAD.encode(st.1);
            let keyid = generate_rand_string(18);
            global_app_data_insert(&keyid.clone(), &st.0);
            Json(ApiResult::ok(
                json!({"code_id": keyid, "image_url": basestr, "expire": get_local_timestamp() + 100000 }),
            ))
        }
        None => Json(ApiResult::error(5010, "FAILED")),
    }
}

/**
 * 用户单点登录接口
 * 提供Token返回
 */
#[handler]
#[fastrace::trace]
pub async fn user_auth_ssologin(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<UserAuthResponse>> {
    let code = match req.query::<String>("token") {
        Some(t) => t,
        None => {
            return Json(ApiResult::error(400, "error.not.provided.sso.token"));
        }
    };

    let authreq = UserAuthRequest {
        sso_token: Some(code),
        ..Default::default()
    };

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
    // let authreq = req;
    _user_auth_login(ctx, authreq).await
}

/**
 * 用户登录接口
 * 提供用户名，密码，以及验证码来进行登录
 */
#[handler]
#[fastrace::trace]
pub async fn user_auth_login(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<UserAuthResponse>> {
    let authreq = match req.parse_json::<UserAuthRequest>().await {
        Ok(auth) => auth,
        Err(err) => {
            log::info!("Parse request body failed. {err}");
            return Json(ApiResult::error(400, "error.parse.user.auth.request"));
        }
    };

    log::error!("AuthReq: {authreq:?}");

    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
    // let authreq = req;
    _user_auth_login(ctx, authreq).await
}

#[fastrace::trace]
pub async fn _user_auth_login(
    ctx: Arc<Mutex<InvocationContext>>,
    reqs: UserAuthRequest,
) -> Json<ApiResult<UserAuthResponse>> {
    let authconf = AuthorizationConfig::get();
    let auth_service = AuthorizationService(authconf.clone());
    let organization = if authconf.enable_organization {
        reqs.organization.clone()
    } else {
        None
    };

    let checking_sso_token = !reqs.sso_token.clone().unwrap_or_default().trim().is_empty(); // not empty

    let req = if checking_sso_token {
        match auth_service.sso_check(ctx.clone(), &reqs).await {
            Ok(ts) => ts,
            Err(err) => {
                log::error!("error to check sso {err}");
                return Json(ApiResult::error(401, "error.for.sso.checking"));
            }
        }
    } else {
        reqs.clone()
    };

    if authconf.enable_captcha || req.extends.is_some() {
        let captcha_id = req.captcha_id.clone().unwrap();
        let codeval = global_app_data_get(&captcha_id);
        if codeval.is_none() {
            return Json(ApiResult::error(400, "missing.verify.code.key"));
        }
        global_app_data_remove(&captcha_id);

        if codeval.unwrap().to_lowercase()
            != req.captcha_code.clone().unwrap_or_default().to_lowercase()
        {
            return Json(ApiResult::error(400, "error.verify.code"));
        }
    }

    log::error!("Auth Request {req:?}");

    if req.extends.is_none()
        && (req.credential.is_none() || req.credential.clone().unwrap_or_default().is_empty())
        && req.sso_token.is_none()
    {
        return Json(ApiResult::error(401, "error.auth.password.empty"));
    }

    let username_field = auth_service
        .0
        .username_field
        .clone()
        .unwrap_or("username".to_owned());

    let result = match req.extends.clone() {
        Some(st) => {
            match auth_service
                .find_user_opts(ctx.clone(), &req.organization.clone(), &st)
                .await
            {
                Ok(user) => match user {
                    Some(user) => {
                        let user_name = user.get(username_field).unwrap_or(&Value::Null).clone();
                        let org = if authconf.enable_organization {
                            user.get(
                                authconf
                                    .organization_field
                                    .clone()
                                    .unwrap_or("organization".to_owned()),
                            )
                            .map(|f| match f {
                                Value::String(st) => st.clone(),
                                _ => f.to_string(),
                            })
                        } else {
                            None
                        };

                        Ok((
                            Some(user),
                            user_name.as_str().map(|f| f.to_owned()).unwrap_or_default(),
                            org,
                        ))
                    }
                    None => Ok((None, String::new(), None)),
                },
                Err(err) => Err(anyhow!(err)),
            }
        }
        None => {
            let username = req.username.clone().unwrap_or_default();
            let org = organization.clone();
            match auth_service.find_user(ctx.clone(), &username, &org).await {
                Ok(user) => {
                    // should extra the username and org name
                    let (us, org_name) = match user.clone() {
                        Some(us) => {
                            let user_name = match us.get(username_field).unwrap().clone() {
                                Value::String(tm) => tm,
                                _ => "".to_string(),
                            };
                            let org = if authconf.enable_organization {
                                us.get(
                                    authconf
                                        .organization_field
                                        .clone()
                                        .unwrap_or("organization".to_owned()),
                                )
                                .map(|f| match f {
                                    Value::String(st) => st.clone(),
                                    _ => f.to_string(),
                                })
                            } else {
                                None
                            };
                            (user_name, org)
                        }
                        None => (String::new(), None),
                    };
                    Ok((user, us, org_name))
                }
                Err(err) => Err(anyhow!(err)),
            }
        }
    };

    match result {
        Ok((user, username, org)) => {
            if user.is_none() {
                return Json(ApiResult::error(404, "User Not Found"));
            }

            if req.extends.is_none() && req.sso_token.is_none() {
                if let Err(err) =
                    auth_service.credential_validate(&user.clone().unwrap_or(Value::Null), &req)
                {
                    return Json(ApiResult::error(401, &err.to_string()));
                }
            }

            match auth_service
                .find_user_roles(ctx, &username, &org.clone())
                .await
            {
                Ok(roles) => {
                    let exp = OffsetDateTime::now_utc()
                        + salvo::http::cookie::time::Duration::seconds(
                            authconf.token_expire.unwrap_or(7200i64),
                        );
                    let juc = JwtUserClaims {
                        username: username.clone(),
                        userid: get_user_id_from_user(&user, &authconf),
                        superadmin: roles.contains(&"ROLE_SUPERADMIN".to_string()),
                        domain: org
                            .clone()
                            .unwrap_or(authconf.app_name.clone().unwrap_or("default".to_owned())),
                        exp: exp.unix_timestamp(),
                    };

                    log::info!("juc: {juc:?}");

                    match jsonwebtoken::encode(
                        &jsonwebtoken::Header::default(),
                        &juc,
                        &EncodingKey::from_secret(
                            authconf
                                .token_solt
                                .clone()
                                .unwrap_or("AuthorizationJWTToken".to_owned())
                                .as_bytes(),
                        ),
                    ) {
                        Ok(tk) => {
                            let authresp = UserAuthResponse {
                                organization: org.clone(),
                                username: username.clone(),
                                detail: auth_service.desensitize_user(&user.unwrap_or(Value::Null)),
                                state: 0,
                                token: Some(tk),
                                roles,
                            };
                            Json(ApiResult::ok(authresp))
                        }
                        Err(err) => {
                            log::error!("Error on generate jwt token {err}");
                            Json(ApiResult::error(500, "error.token.generation"))
                        }
                    }
                }
                Err(err) => {
                    log::error!("Error on invoke role_search {err}");
                    Json(ApiResult::error(500, "invoke.role_search.error"))
                }
            }
        }
        Err(err) => {
            log::error!("Error on invoke user_search {err}");
            Json(ApiResult::error(500, "invoke.user_search.error"))
        }
    }
}

/**
 * 用户Token验证
 * 在请求过程中按正常方式加入Authorization的Token
 * 返回用户信息，包括用户的角色（被授权的资源）
 */
#[handler]
#[fastrace::trace]
pub async fn user_auth_info(depot: &mut Depot) -> Json<ApiResult<UserAuthResponse>> {
    match depot.jwt_auth_state() {
        JwtAuthState::Authorized => {
            let data = depot.jwt_auth_data::<JwtUserClaims>().unwrap();
            let username = data.claims.username.clone();
            let org = data.claims.domain.clone();
            let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
            // let authreq = req;
            _user_auth_info(ctx, &username, Some(org), false, false).await
        }
        JwtAuthState::Unauthorized => Json(ApiResult::<UserAuthResponse>::error(
            StatusCode::UNAUTHORIZED.as_u16() as i32,
            "Unauthorized",
        )),
        JwtAuthState::Forbidden => Json(ApiResult::<UserAuthResponse>::error(
            StatusCode::FORBIDDEN.as_u16() as i32,
            "Forbidden",
        )),
    }
}

#[fastrace::trace]
async fn _user_auth_info(
    ctx: Arc<Mutex<InvocationContext>>,
    username: &str,
    org_: Option<String>,
    retoken: bool,
    without_detail: bool,
) -> Json<ApiResult<UserAuthResponse>> {
    let authconf = AuthorizationConfig::get();
    let auth_service = AuthorizationService(authconf.clone());
    let organization = if authconf.enable_organization {
        org_.clone()
    } else {
        None
    };

    match auth_service
        .find_user(ctx.clone(), username, &organization)
        .await
    {
        Ok(user) => match auth_service
            .find_user_roles(ctx, username, &organization)
            .await
        {
            Ok(roles) => {
                if !retoken {
                    let authresp = UserAuthResponse {
                        organization: organization.clone(),
                        username: username.to_string(),
                        detail: if without_detail {
                            Value::Null
                        } else {
                            auth_service.desensitize_user(&user.unwrap_or(Value::Null))
                        },
                        state: 0,
                        token: None,
                        roles: if without_detail { vec![] } else { roles },
                    };

                    Json(ApiResult::ok(authresp))
                } else {
                    let exp = OffsetDateTime::now_utc()
                        + salvo::http::cookie::time::Duration::seconds(if without_detail {
                            100000000i64
                        } else {
                            authconf.token_expire.unwrap_or(7200i64)
                        });
                    let juc = JwtUserClaims {
                        username: username.to_string(),
                        userid: get_user_id_from_user(&user, &authconf),
                        superadmin: roles.contains(&"ROLE_SUPERADMIN".to_string()),
                        domain: organization
                            .clone()
                            .unwrap_or(authconf.app_name.clone().unwrap_or("default".to_owned())),
                        exp: exp.unix_timestamp(),
                    };

                    match jsonwebtoken::encode(
                        &jsonwebtoken::Header::default(),
                        &juc,
                        &EncodingKey::from_secret(
                            authconf
                                .token_solt
                                .clone()
                                .unwrap_or("AuthorizationJWTToken".to_owned())
                                .as_bytes(),
                        ),
                    ) {
                        Ok(tk) => {
                            let authresp = UserAuthResponse {
                                organization: organization.clone(),
                                username: username.to_string(),
                                detail: if without_detail {
                                    Value::Null
                                } else {
                                    auth_service.desensitize_user(&user.unwrap_or(Value::Null))
                                },
                                state: 0,
                                token: Some(tk),
                                roles: if without_detail { vec![] } else { roles },
                            };
                            Json(ApiResult::ok(authresp))
                        }
                        Err(err) => {
                            log::error!("Error on generate jwt token {err}");
                            Json(ApiResult::error(500, "error.token.generation"))
                        }
                    }
                }
            }
            Err(err) => {
                log::error!("Error on invoke role_search {err}");
                Json(ApiResult::error(500, "invoke.role_search.error"))
            }
        },
        Err(err) => {
            log::error!("Error on invoke user_search {err}");
            Json(ApiResult::error(500, "invoke.user_search.error"))
        }
    }
}

/**
 * 用户Token的刷新
 * 在请求过程中按正常方式加入Authorization的Token
 * 返回用户信息，包括用户的角色（被授权的资源），同时重新一个新的Token。新的Token会延展JWTToken的过期时间
 */
#[handler]
#[fastrace::trace]
pub async fn user_auth_refresh(depot: &mut Depot) -> Json<ApiResult<UserAuthResponse>> {
    match depot.jwt_auth_state() {
        JwtAuthState::Authorized => {
            let data = depot.jwt_auth_data::<JwtUserClaims>().unwrap();
            let username = data.claims.username.clone();
            let org = data.claims.domain.clone();
            let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
            // let authreq = req;
            _user_auth_info(ctx, &username, Some(org), true, false).await
        }
        JwtAuthState::Unauthorized => Json(ApiResult::<UserAuthResponse>::error(
            StatusCode::UNAUTHORIZED.as_u16() as i32,
            "Unauthorized",
        )),
        JwtAuthState::Forbidden => Json(ApiResult::<UserAuthResponse>::error(
            StatusCode::FORBIDDEN.as_u16() as i32,
            "Forbidden",
        )),
    }
}

/**
 * 用户Token的刷新
 * 在请求过程中按正常方式加入Authorization的Token
 * 返回用户信息，包括用户的角色（被授权的资源），同时重新一个新的Token。新的Token会延展JWTToken的过期时间
 */
#[handler]
#[fastrace::trace]
pub async fn user_auth_logout(_depot: &mut Depot) -> Json<ApiResult<UserAuthResponse>> {
    Json(ApiResult::<UserAuthResponse>::error(
        StatusCode::OK.as_u16() as i32,
        "SIGNOUT",
    ))
}

/**
 * 修改用户密码
 * 修改用户密码，根据相应的来验证
 **/
#[handler]
#[fastrace::trace]
pub async fn user_auth_change_pwd(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<UserAuthResponse>> {
    let cpwdreq = match req.parse_json::<ChangePasswordRequest>().await {
        Ok(auth) => auth,
        Err(err) => {
            log::info!("Parse request body failed. {err}");
            return Json(ApiResult::error(400, "error.parse.user.changepwd.request"));
        }
    };

    let authreq = UserAuthRequest {
        organization: cpwdreq.organization.clone(),
        username: Some(cpwdreq.username.clone()),
        credential: Some(cpwdreq.credential.clone()),
        captcha_code: cpwdreq.captcha_code.clone(),
        captcha_id: cpwdreq.captcha_id.clone(),
        ..Default::default()
    };

    // let authreq = req;
    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
    let ret = _user_auth_login(ctx.clone(), authreq).await;
    if ret.0.status == 200 || ret.0.status == 0 {
        let user = ret.0.data.unwrap();
        log::info!("got login user: {user:?}");
        _user_auth_do_change_pwd(ctx, &user.detail, &cpwdreq).await
    } else {
        ret
    }
}

#[fastrace::trace]
async fn _user_auth_do_change_pwd(
    ctx: Arc<Mutex<InvocationContext>>,
    user: &Value,
    req: &ChangePasswordRequest,
) -> Json<ApiResult<UserAuthResponse>> {
    let authconf = AuthorizationConfig::get();
    let auth_service = AuthorizationService(authconf.clone());
    match auth_service.change_passwd(ctx, user, req).await {
        Ok(_) => Json(ApiResult::ok(UserAuthResponse::default())),
        Err(err) => {
            log::info!("Error for change password {err}");
            Json(ApiResult::error(400, "error.change.password"))
        }
    }
}

/**
 * 用户登录接口
 * 提供用户名，密码，以及验证码来进行登录
 */
#[handler]
#[fastrace::trace]
pub async fn user_app_exchange(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<UserAuthResponse>> {
    let authreq = match req.parse_queries::<AppKeyRequest>() {
        Ok(auth) => auth,
        Err(_) => match req.parse_json::<AppKeyRequest>().await {
            Ok(authbody) => authbody,
            Err(err) => {
                log::info!("Parse request body failed. {err}");
                return Json(ApiResult::error(400, "error.parse.user.appkey.request"));
            }
        },
    };

    let authconf = AuthorizationConfig::get();
    let auth_service = AuthorizationService(authconf.clone());
    let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
    // let authreq = req;
    match auth_service
        .find_appid_secret(ctx.clone(), &authreq.app_id)
        .await
    {
        Ok(ts) => {
            if ts.is_empty() {
                Json(ApiResult::error(404, "error.find.appsecret.not_found"))
            } else if ts.len() > 1 {
                Json(ApiResult::error(405, "error.find.appsecret.duplicated"))
            } else {
                let apppar = ts[0].clone();
                let encryption = apppar.encryption.unwrap_or_default();

                let (app_secret, _time) = if encryption {
                    if let Some(secret) = rsa_decrypt_with_private_key(
                        &authreq.app_secret,
                        &authconf.rsa_private_key.clone().unwrap_or_default(),
                    ) {
                        log::warn!("decode to {}", secret.clone());
                        let twsecs = secret.split("##").collect_vec();
                        if twsecs.len() > 1 {
                            let sec = twsecs[0].to_string();
                            let time = twsecs[1].to_string();
                            match time.parse::<i64>() {
                                Ok(t) => {
                                    let now = get_local_timestamp() as i64;
                                    log::info!("now timestamp: {now}, {}", now - t);
                                    let tm = now - t;
                                    if tm.abs() > 30000 {
                                        return Json(ApiResult::<UserAuthResponse>::error(
                                            StatusCode::FORBIDDEN.as_u16() as i32,
                                            "Invalid Timestamp",
                                        ));
                                    }
                                    (sec, t)
                                }
                                Err(_) => {
                                    return Json(ApiResult::<UserAuthResponse>::error(
                                        StatusCode::FORBIDDEN.as_u16() as i32,
                                        "Invalid AppId with AppSecret Pair",
                                    ));
                                }
                            }
                        } else {
                            return Json(ApiResult::<UserAuthResponse>::error(
                                StatusCode::FORBIDDEN.as_u16() as i32,
                                "Invalid AppId with AppSecret Pair",
                            ));
                        }
                    } else {
                        return Json(ApiResult::<UserAuthResponse>::error(
                            StatusCode::FORBIDDEN.as_u16() as i32,
                            "Invalid AppId with AppSecret Pair",
                        ));
                    }
                } else {
                    (authreq.app_secret.clone(), 0)
                };

                if app_secret != apppar.app_secret {
                    return Json(ApiResult::<UserAuthResponse>::error(
                        StatusCode::FORBIDDEN.as_u16() as i32,
                        "Invalid AppSecret for this AppId",
                    ));
                }

                let username = apppar.username.clone().unwrap_or_default();
                let orgname = apppar.orgname.clone();
                if authconf.check_relative_user {
                    _user_auth_info(ctx, &username, orgname, true, true).await
                } else {
                    let exp = OffsetDateTime::now_utc()
                        + salvo::http::cookie::time::Duration::seconds(100000000i64);
                    let juc = JwtUserClaims {
                        username: username.to_string(),
                        userid: "0".to_owned(),
                        superadmin: false,
                        domain: authreq.app_id.clone(),
                        exp: exp.unix_timestamp(),
                    };

                    match jsonwebtoken::encode(
                        &jsonwebtoken::Header::default(),
                        &juc,
                        &EncodingKey::from_secret(
                            authconf
                                .token_solt
                                .clone()
                                .unwrap_or("AuthorizationJWTToken".to_owned())
                                .as_bytes(),
                        ),
                    ) {
                        Ok(tk) => {
                            let authresp = UserAuthResponse {
                                organization: Some(authreq.app_id.clone()),
                                username: username.to_string(),
                                detail: Value::Null,
                                state: 0,
                                token: Some(tk),
                                roles: vec![],
                            };
                            Json(ApiResult::ok(authresp))
                        }
                        Err(err) => {
                            log::error!("Error on generate jwt token {err}");
                            Json(ApiResult::error(500, "error.token.generation"))
                        }
                    }
                }
            }
        }
        Err(err) => {
            log::error!("error to find appid/secret {err}");
            Json(ApiResult::error(400, "error.find.appsecret.error"))
        }
    }
}

#[handler]
pub async fn refresh_app_secrets(
    depot: &mut Depot,
    _req: &mut Request,
) -> Json<ApiResult<String>> {

    match depot.jwt_auth_state() {
        JwtAuthState::Authorized => {
            if let Some(claim) = depot.jwt_auth_data::<JwtUserClaims>() {
                if claim.claims.superadmin {
                    SchedulerHolder::get().addjob_delay(
                        &Uuid::new_v4().to_string(),
                        "",
                        None,
                        Some(1),
                        Box::new(AuthAppSecuretLoadJob),
                    );
                    return Json(ApiResult::ok("SUCCESS".to_string()));
                }
            }
        },
        _ => {

        }
    }


    Json(ApiResult::error(403, "Forbidden"))
}


#[derive(Debug, Clone, Deserialize, Serialize)]
struct GrowthAccessToken {
    pub appid: Option<String>,
    pub timestamp: u64,             // UTC Timestamp, millese-second, 
    pub code: Option<String>        // code should be encryption by given app_secret with method hmac_sha256,
                                    // The auth_code should be hmac_sha256(app_secret, appid={appid}&secret={app_secret}&timestamp={timestamp})
}

#[handler]
pub async fn access_token(
    _depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<AuthResponse>> {
    let gat = req.parse_body::<GrowthAccessToken>().await.map(Some).unwrap_or(req.parse_queries::<GrowthAccessToken>().map(Some).unwrap_or(None));

    match gat {
        None => {
            return Json(ApiResult::error(401, "Unable to parse the request"));
        },
        Some(ts) => {
            if ts.appid.is_none() {
                return Json(ApiResult::error(403, "Forbidden: AppId was not provided"));
            }
            if ts.code.is_none() {
                return Json(ApiResult::error(403, "Forbidden: Encrypted code for this app_id was not provided"));
            }

            let app_id = ts.appid.unwrap_or_default();
            let tstamp = ts.timestamp;
            let code = ts.code.unwrap_or_default();
            let utcnow = OffsetDateTime::now_utc();
            let minutc = utcnow.unix_timestamp();
            let givutc = tstamp as i64 / 1000;
            if  (minutc - givutc).abs() > 180  { // timestamp out of 3 minus, return an error 
                log::error!("Timestamp ERROR. {} compare to {}", utcnow.unix_timestamp(), tstamp);
                return Json(ApiResult::error(400, "Timestamp Error. The request timestamp is older/newer than current timestamp. Please make sure the time difference between servers."));
            }

            if let Some(apppair) = find_app_secret_by_app_id(&app_id).await {
                let app_secret = apppair.app_secret.clone();
                let text = format!("appid={app_id}&secret={app_secret}&timestamp={tstamp}");
                
                let hmax = hmac_sha256(&app_secret, &text);
                let hex = hex_encode_string(&hmax);
                log::warn!("code={hex}");
                if code == hex {
                    let authconf = AuthorizationConfig::get();
                    let token_expr = authconf.token_expire.unwrap_or(7200i64);
                    
                    let exp = utcnow + salvo::http::cookie::time::Duration::seconds(token_expr);

                    let username = apppair.username.unwrap_or_default();
                    let userid = apppair.userid.unwrap_or_default();
                    let orgname = match apppair.orgname {
                        Some(org) => {
                            if org.is_empty() {
                                authconf.app_name.unwrap_or("api-zone".to_string())
                            } else {
                                org
                            }
                        },
                        None => {
                            authconf.app_name.unwrap_or("api-zone".to_string())
                        }
                    };

                    let juc = JwtUserClaims {
                        username: username,
                        userid: userid,
                        superadmin: false,
                        domain: format!("{app_id}@{orgname}"),
                        exp: exp.unix_timestamp(),
                    };

                    match jsonwebtoken::encode(
                        &jsonwebtoken::Header::default(),
                        &juc,
                        &EncodingKey::from_secret(
                            authconf
                                .token_solt
                                .clone()
                                .unwrap_or("AuthorizationJWTToken".to_owned())
                                .as_bytes(),
                        ),
                    ) {
                        Ok(tk) => {
                            let authresp = AuthResponse { 
                                token: Some(tk), 
                                expired: token_expr as u32 
                            };
                            Json(ApiResult::ok(authresp))
                        }
                        Err(err) => {
                            log::error!("Error on generate jwt token {err}");
                            Json(ApiResult::error(500, "error.token.generation"))
                        }
                    }
                } else {
                    return Json(ApiResult::error(403, "Invalid code"));
                }
            } else {
                return Json(ApiResult::error(404, "Invalid appId"));
            }
        }
    }
}
