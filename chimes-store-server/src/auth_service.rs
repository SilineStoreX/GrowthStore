use crate::api::{ChangePasswordRequest, UserAuthRequest};
use anyhow::{anyhow};
use chimes_store_core::{
    config::{
        auth::{AppSecretPair, AuthorizationConfig, JwtUserClaims},
        ConditionItem, QueryCondition,
    },
    service::{
        invoker::InvocationContext, registry::SchemaRegistry, sched::JobInvoker, sdk::InvokeUri,
        starter::MxStoreService,
    },
    utils::{
        get_local_timestamp, global_data::{
            copy_value_excluded, rsa_decrypt_with_private_key, rsa_encrypt_with_public_key,
        }, redis::{redis_get, redis_set}, ApiResult
    },
};
use chimes_store_dbs::utils::{aes_encrypt_to_text, md5_hash, sha1_256_hash, sha2_256_hash};
use chimes_store_utils::common::copy_to_slice;
use itertools::Itertools;
use jsonwebtoken::{errors::ErrorKind, Algorithm, EncodingKey, Header, TokenData};
use rbatis::rbdc;
use salvo::{
    async_trait, http::cookie::time::OffsetDateTime, jwt_auth::{ConstDecoder, JwtAuthDepotExt, JwtAuthState, JwtError}, writing::Json, Depot, FlowCtrl, Handler, Request, Response, Writer
};
use salvo::jwt_auth::JwtAuthDecoder;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap, future::Future, sync::{Arc, Mutex, OnceLock}
};
use substring::Substring;

static APP_SECRET_KEYS_MANAGER: OnceLock<Arc<Mutex<HashMap<String, AppSecretPair>>>> =
    OnceLock::new();

pub async fn find_app_secret_by_app_id(app_id: &str) -> Option<AppSecretPair> {
    // let keymap = APP_SECRET_KEYS_MANAGER.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
    let conf = AuthorizationConfig::get();
    let auth_service = AuthorizationService(conf);
    let ctx = InvocationContext::arcnew();
    if let Ok(rs) = auth_service.find_appid_secret(ctx, app_id).await {
       rs.first().map(|t| t.to_owned())
    } else {
        None
    }
}

fn find_app_secret_by_token(token: &str) -> Option<AppSecretPair> {
    if let Some(manager) = APP_SECRET_KEYS_MANAGER.get() {
        let conf = AuthorizationConfig::get();
        let appscret_redis = conf.appsecret_redis.clone().unwrap_or_default();
        let appsecret_prov = conf.appsecret_provider.clone().unwrap_or_default();
        if appscret_redis.is_empty() || appsecret_prov.is_empty() {
            manager.lock().unwrap().get(token).map(|t| t.to_owned())
        } else {
            let ns = appscret_redis;
            if let Ok(Some(keytext)) = redis_get(&ns, token) {
                serde_json::from_str::<AppSecretPair>(&keytext).map(Some).unwrap_or(None)
            } else {
                None
            }
        }
    } else {
        None
    }
}

pub async fn refresh_app_secret_token_map() {
    let keymap = APP_SECRET_KEYS_MANAGER.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
    let conf = AuthorizationConfig::get();
    let appscret_redis = conf.appsecret_redis.clone().unwrap_or_default();
    let auth_service = AuthorizationService(conf);
    let ctx = InvocationContext::arcnew();
    if let Ok(rs) = auth_service.load_all_appcrets(ctx).await {
        if appscret_redis.is_empty() {
            rs.iter().filter(|p| p.token.is_some()).for_each(|s| {
                keymap
                    .lock()
                    .unwrap()
                    .insert(s.token.clone().unwrap_or_default(), s.to_owned());
            });
        } else {
            let nsp = appscret_redis.clone();
            rs.iter().filter(|p| p.token.is_some()).for_each(|s| {
                if let Ok(pt) = serde_json::to_string(s) {
                    let _ = redis_set(&nsp, &s.token.clone().unwrap_or_default(), &pt).is_ok();
                }
            });                
        }
    }
}

pub struct AuthAppSecuretLoadJob;

impl JobInvoker for AuthAppSecuretLoadJob {
    fn get_description(&self) -> String {
        "加载AppSecurets".to_string()
    }

    fn get_invoke_uri(&self) -> String {
        "".to_string()
    }

    fn get_invoke_params(&self) -> Option<Value> {
        None
    }

    fn exec(
        &self,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send + 'static>> {
        Box::pin(async move {
            refresh_app_secret_token_map().await;
            Ok(())
        })
    }
}

pub fn appkey_decode<T: DeserializeOwned + Clone>(token: &str, depot: &mut Depot) -> Result<TokenData<T>, JwtError> {
    const GROWTH_PRIFIX: &str = "growth-";

    if token.starts_with(GROWTH_PRIFIX) {
        let _ = token.substring(GROWTH_PRIFIX.len(), token.len());
        let timestamp = get_local_timestamp() as i64 + 8400000;
        if let Some(user) = find_app_secret_by_token(token) {
            let domain = if let Some(org) = user.orgname.clone() {
                format!("{}@{}", user.app_id, org)
            } else {
                format!("{}@api-zone", user.app_id)
            };

            let ct = json!({
                "username": user.username.clone().unwrap_or_default(),
                "userid": user.userid.clone().unwrap_or_default(),
                "superadmin": false,
                "domain": domain,
                "exp": timestamp,
            });
            let claims = serde_json::from_value::<T>(ct)?;
            let header = Header::new(Algorithm::RS512);
            depot.insert("GROWTH_API_SECRET", user.app_secret.clone());
            Ok(TokenData { header, claims })
        } else {
            log::debug!("Not found by the token in app_secret_keys.");
            let err = JwtError::from(ErrorKind::InvalidSubject);
            Err(err)
        }
    } else {
        log::debug!("Invalid token that is not start with growth-.");
        let err = JwtError::from(ErrorKind::InvalidToken);
        Err(err)
    }
}

pub struct ComposeDecoder {
    decoder: ConstDecoder,
}

impl ComposeDecoder {
    pub fn new(constdecoder: ConstDecoder) -> Self {
        Self {
            decoder: constdecoder,
        }
    }
}

impl JwtAuthDecoder for ComposeDecoder {
    type Error = JwtError;
    // type Error: std::error::Error + Send + Sync + 'static;


    async fn decode<C>(
        &self,
        token: &str,
        depot: &mut Depot,
    ) -> Result<TokenData<C>, Self::Error>
    where
        C: for<'de> Deserialize<'de> + Clone {
        if let Ok(jwt) = self.decoder.decode::<JwtUserClaims>(token, depot).await {
            let domain = jwt.claims.domain.clone();
            if let Some(appid) = domain.split("@").next() {
                if let Some(apppair) = find_app_secret_by_app_id(appid).await {
                    depot.insert("GROWTH_API_SECRET", apppair.app_secret.clone());
                }
            }
        };
        match self.decoder.decode::<C>(token, depot).await {
            Ok(t) => {
                Ok(t)
            },
            Err(err) => {
                log::debug!("error to do  jwttoken decode. {err}");
                appkey_decode(token, depot)
            }
        }
    }
}


pub fn generate_jwt_authorization_token(username: &str, user_id: &str, org: Option<String>) -> Result<String, anyhow::Error> {
    let authconf = AuthorizationConfig::get();
    let exp = OffsetDateTime::now_utc()
        + salvo::http::cookie::time::Duration::seconds(
            authconf.token_expire.unwrap_or(7200i64),
        );
    let juc = JwtUserClaims {
        username: username.to_string(),
        userid: user_id.to_string(),
        superadmin: false,
        domain: org
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
        Ok(token) => Ok(token),
        Err(err) => {
            Err(anyhow!("error to encode as token {err:?}"))
        },
    }
}

pub struct AuthorizationService(pub(crate) AuthorizationConfig);
unsafe impl Send for AuthorizationService {}
unsafe impl Sync for AuthorizationService {}

impl AuthorizationService {
    /**
     * 根据传递过来的Token来进行用户信息的交换
     * 首先是调用所配置的
     */
    pub async fn sso_check(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        user: &UserAuthRequest,
    ) -> Result<UserAuthRequest, anyhow::Error> {
        let sso_check_uri = match self.0.validate_url.clone() {
            Some(t) => {
                if t.trim().is_empty() {
                    return Ok(user.clone());
                } else {
                    t.trim().to_string()
                }
            }
            None => {
                return Ok(user.clone());
            }
        };

        let sso_token_text = user.sso_token.clone().unwrap_or_default();

        if sso_token_text.trim().is_empty() {
            return Ok(user.clone());
        }

        // direct to access the uri
        // the username field should be fixed as username
        match MxStoreService::invoke_return_one(
            sso_check_uri,
            ctx,
            vec![json!({"code": user.sso_token })],
        )
        .await
        {
            Ok(rt) => {
                match rt.map(|v| {
                    serde_json::from_value::<UserAuthRequest>(v)
                        .map_err(|err| anyhow!("error to parse the result {err}"))
                }) {
                    Some(mt) => match mt {
                        Ok(mut ms) => {
                            ms.sso_token = user.sso_token.clone();
                            Ok(ms)
                        }
                        Err(err) => Err(err),
                    },
                    None => Err(anyhow!("Chould not parse the result")),
                }
            }
            Err(err) => Err(anyhow!("Error to check sso {err}")),
        }
    }

    /**
     * 对密码进行相应的处理
     */
    pub fn credential_preprocess(&self, user: &UserAuthRequest) -> UserAuthRequest {
        let credential = user.credential.clone().unwrap_or_default();
        if let Some(l) = credential.find(':') {
            log::info!("found : at {l}");
            let pwd = credential.substring(l + 1, credential.len());
            let mut userclone = user.clone();
            // userclone.credential = pwd.to_owned();
            userclone.credential = Some(pwd.to_owned());
            // pwd.clone_into(&mut userclone.credential);
            userclone
        } else {
            log::info!("Not found anything");
            let mut userclone = user.clone();
            // call the method to encrypto the
            userclone.credential =
                Some(self.encrypt_password(&credential, user.captcha_code.clone()));
            userclone
        }
    }

    pub fn encrypt_password(&self, org_pwd: &str, captha_code: Option<String>) -> String {
        let conf = self.0.clone();
        match conf
            .credential_hash_method
            .clone()
            .unwrap_or("md5".to_owned())
            .to_lowercase()
            .as_str()
        {
            "aes" => {
                let key_str = conf
                    .credential_key
                    .clone()
                    .unwrap_or("defaultcredentialkey".to_string());
                let iv_str = conf
                    .credential_solt
                    .clone()
                    .unwrap_or("defaultcredentialsolt".to_string());
                let mut key = [0; 32];
                let mut iv = [0; 16];
                let key_ = key_str.as_bytes();
                let iv_ = iv_str.as_bytes();
                copy_to_slice(&mut iv, iv_);
                copy_to_slice(&mut key, key_);
                aes_encrypt_to_text(&org_pwd.as_bytes(), &key, &iv)
            }
            "sha1" => sha1_256_hash(&org_pwd.as_bytes()),
            "sha2" => sha2_256_hash(&org_pwd.as_bytes()),
            "md5" => md5_hash(&org_pwd.as_bytes()),
            "mix" => {
                let key_str = conf
                    .credential_key
                    .clone()
                    .unwrap_or("defaultcredentialkey".to_string());
                let iv_str = conf
                    .credential_solt
                    .clone()
                    .unwrap_or("defaultcredentialsolt".to_string());
                let mut key = [0; 32];
                let mut iv = [0; 16];
                let key_ = key_str.as_bytes();
                let iv_ = iv_str.as_bytes();
                copy_to_slice(&mut iv, iv_);
                copy_to_slice(&mut key, key_);
                let enc = aes_encrypt_to_text(&org_pwd.as_bytes(), &key, &iv);
                md5_hash(&format!("{enc}_{}", captha_code.unwrap_or_default()))
            }
            "rsa" => rsa_encrypt_with_public_key(
                org_pwd,
                &conf.rsa_public_key.clone().unwrap_or_default(),
            )
            .unwrap_or_default(),
            _ => org_pwd.to_owned(),
        }
    }

    pub async fn find_user(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        username: &str,
        organization: &Option<String>,
    ) -> Result<Option<Value>, anyhow::Error> {
        let username_field = self
            .0
            .username_field
            .clone()
            .unwrap_or("username".to_owned());
        let org_field = self
            .0
            .organization_field
            .clone()
            .unwrap_or("organization".to_owned());

        let ci = ConditionItem {
            field: username_field.clone(),
            op: "=".to_owned(),
            value: json!(username),
            ..Default::default()
        };

        let qs = if self.0.enable_organization {
            let ori = ConditionItem {
                field: org_field.clone(),
                op: "=".to_owned(),
                value: json!(organization),
                ..Default::default()
            };

            QueryCondition {
                and: vec![ci, ori],
                ..Default::default()
            }
        } else {
            QueryCondition {
                and: vec![ci],
                ..Default::default()
            }
        };

        let usersearch = self.0.user_search.clone().unwrap_or_default();
        // MxStoreService::invoke_return_one(usersearch.clone(), ctx, vec![json!(qs)]).await
        if usersearch.starts_with("query://") {
            let qcond = if self.0.enable_organization {
                json!({
                    org_field: json!(organization),
                    username_field: json!(username)
                })
            } else {
                json!({
                    username_field: json!(username)
                })
            };
            SchemaRegistry::get()
                .invoke_return_option(&usersearch, ctx, &[qcond])
                .await
        } else {
            SchemaRegistry::get()
                .invoke_return_option(&usersearch, ctx, &[json!(qs)])
                .await
        }
    }

    /**
     * 这个方法是根据传递过来的用户其它字段来进行验证用户信息的
     * 主要用于进行微信登录（通过手机号码、OpenID）、支付宝登录等方案。
     * 传递过来的user是JSON表示的用户信息
     * 其中的属性必须为find_one或查询的search能够接收的查询条件。
     * 另外，如果使用了扩展登录，必须将如何获得手机号码，或OpenID的Code作为验证码的一部分进行传递
     * 如：微信中，getPhoneNumber，需要通过code再通过接口来获得对应的手机号码，getOpenID，也类似。
     * 这个时候，需要将AuthInfo的captcha_id设为code，captcha_code设置为对应的手机号码或open_id。
     * 同时，在user中指定phone字段或open_id字段
     */
    pub async fn find_user_opts(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        organization: &Option<String>,
        user: &Value,
    ) -> Result<Option<Value>, anyhow::Error> {
        let cpv = match user.as_object() {
            Some(tm) => {
                let mut cps = vec![];
                if self.0.enable_organization && organization.is_some() {
                    cps.push(ConditionItem {
                        field: self
                            .0
                            .organization_field
                            .clone()
                            .unwrap_or("organization".to_owned()),
                        op: "=".to_owned(),
                        value: json!(organization.to_owned()),
                        ..Default::default()
                    });
                }
                for (key, val) in tm {
                    cps.push(ConditionItem {
                        field: key.to_owned(),
                        op: "=".to_owned(),
                        value: val.to_owned(),
                        ..Default::default()
                    });
                }
                cps
            }
            None => {
                return Err(anyhow!("No user info provided"));
            }
        };

        let qs = QueryCondition {
            and: cpv,
            ..Default::default()
        };

        let usersearch = self.0.user_search.clone().unwrap_or_default();
        // MxStoreService::invoke_return_one(usersearch.clone(), ctx, vec![json!(qs)]).await
        if usersearch.starts_with("query://") {
            match SchemaRegistry::get()
                .invoke_return_vec(&usersearch, ctx, &[user.to_owned(), json!(qs)])
                .await
            {
                Ok(us) => {
                    if us.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(us[0].clone()))
                    }
                }
                Err(err) => Err(err),
            }
        } else {
            SchemaRegistry::get()
                .invoke_return_option(&usersearch, ctx, &[json!(qs)])
                .await
        }
    }

    pub async fn find_user_roles(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        username: &str,
        organization: &Option<String>,
    ) -> Result<Vec<String>, anyhow::Error> {
        let username_field = self
            .0
            .username_field
            .clone()
            .unwrap_or("username".to_owned());
        let org_field = self
            .0
            .organization_field
            .clone()
            .unwrap_or("organization".to_owned());
        let rolename_field = self
            .0
            .role_name_field
            .clone()
            .unwrap_or("role_code".to_owned());
        let rolesearch = self.0.role_search.clone().unwrap_or_default();

        // log::info!("rolesearch: {rolesearch}");

        let roles = if rolesearch.starts_with("query://") {
            let qcond = if self.0.enable_organization {
                json!({&username_field: username, org_field: json!(organization)})
            } else {
                json!({&username_field: username})
            };

            MxStoreService::invoke_return_vec(rolesearch, ctx, vec![qcond]).await?
        } else {
            let qs = if self.0.enable_organization {
                QueryCondition {
                    and: vec![
                        ConditionItem {
                            field: username_field.to_owned(),
                            op: "=".to_owned(),
                            value: Value::String(username.to_owned()),
                            ..Default::default()
                        },
                        ConditionItem {
                            field: org_field.to_owned(),
                            op: "=".to_owned(),
                            value: json!(organization),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                }
            } else {
                QueryCondition {
                    and: vec![ConditionItem {
                        field: username_field.to_owned(),
                        op: "=".to_owned(),
                        value: Value::String(username.to_owned()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }
            };

            if rolesearch.starts_with("object://") {
                MxStoreService::invoke_return_vec(rolesearch, ctx, vec![serde_json::to_value(qs)?])
                    .await?
            } else {
                let qcond = if self.0.enable_organization {
                    json!({&username_field: username, org_field: json!(organization)})
                } else {
                    json!({&username_field: username})
                };

                MxStoreService::invoke_return_vec(
                    rolesearch,
                    ctx,
                    vec![qcond, serde_json::to_value(qs)?],
                )
                .await?
            }
        };

        let role_vec = roles
            .into_iter()
            .map(|f| {
                if let Some(t) = f.get(&rolename_field) {
                    t.as_str().map(|s| s.to_string()).unwrap_or_default()
                } else {
                    f.as_str().map(|t| t.to_string()).unwrap_or_default()
                }
            })
            .collect_vec();
        Ok(role_vec)
    }

    /**
     * find by appid/secret pair
     */
    pub async fn load_all_appcrets(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
    ) -> Result<Vec<AppSecretPair>, anyhow::Error> {
        let asprov = self.0.appsecret_provider.clone().unwrap_or_default();
        if asprov.is_empty() {
            let appids: Vec<AppSecretPair> = self.0.app_secret_keys.clone();
            Ok(appids)
        } else {
            let qs = QueryCondition {
                and: vec![],
                ..Default::default()
            };

            let appseret_provider = self.0.appsecret_provider.clone().unwrap_or_default();
            // MxStoreService::invoke_return_one(usersearch.clone(), ctx, vec![json!(qs)]).await
            let res = if appseret_provider.starts_with("query://") {
                let qcond = json!({"_cond": qs});
                SchemaRegistry::get()
                    .invoke_return_vec(&appseret_provider, ctx, &[qcond])
                    .await
            } else {
                SchemaRegistry::get()
                    .invoke_return_vec(&appseret_provider, ctx, &[json!(qs)])
                    .await
            };

            match res {
                Ok(tvs) => {
                    let tvxs: Vec<AppSecretPair> = tvs
                        .iter()
                        .map(|t| {
                            serde_json::from_value::<AppSecretPair>(t.to_owned())
                                .unwrap_or_default()
                        })
                        .filter(|f| !f.app_id.is_empty() && !f.app_secret.is_empty())
                        .collect();
                    Ok(tvxs)
                }
                Err(err) => Err(err),
            }
        }
    }

    /**
     * find by appid/secret pair
     */
    pub async fn find_appid_secret(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        appid: &str,
    ) -> Result<Vec<AppSecretPair>, anyhow::Error> {
        let asprov = self.0.appsecret_provider.clone().unwrap_or_default();
        if asprov.is_empty() {
            let appids: Vec<AppSecretPair> = self
                .0
                .app_secret_keys
                .clone()
                .iter()
                .filter(|p| p.app_id == *appid)
                .map(|t| t.to_owned())
                .collect();
            Ok(appids)
        } else {
            let ci = ConditionItem {
                field: "app_id".to_owned(),
                op: "=".to_owned(),
                value: json!(appid),
                ..Default::default()
            };

            let qs = QueryCondition {
                and: vec![ci],
                ..Default::default()
            };

            let appseret_provider = self.0.appsecret_provider.clone().unwrap_or_default();
            // MxStoreService::invoke_return_one(usersearch.clone(), ctx, vec![json!(qs)]).await
            let res = if appseret_provider.starts_with("query://") {
                let qcond = json!({"_cond": qs});
                SchemaRegistry::get()
                    .invoke_return_vec(&appseret_provider, ctx, &[qcond])
                    .await
            } else {
                SchemaRegistry::get()
                    .invoke_return_vec(&appseret_provider, ctx, &[json!(qs)])
                    .await
            };

            match res {
                Ok(tvs) => {
                    let tvxs: Vec<AppSecretPair> = tvs
                        .iter()
                        .map(|t| {
                            serde_json::from_value::<AppSecretPair>(t.to_owned())
                                .unwrap_or_default()
                        })
                        .filter(|f| !f.app_id.is_empty() && !f.app_secret.is_empty())
                        .collect();
                    Ok(tvxs)
                }
                Err(err) => Err(err),
            }
        }
    }

    /**
     * 进行登录信息与所获取的用户进行进行验证
     * 该方法
     * 1、验证用户状态
     * 2、比较用户密码
     * 3、如果用户状态的字段没有提供，则默认是通过的
     */
    pub fn credential_validate(
        &self,
        user: &Value,
        auth: &UserAuthRequest,
    ) -> Result<bool, anyhow::Error> {
        let passwd_field = self
            .0
            .user_credentials_field
            .clone()
            .unwrap_or("password".to_owned());
        let lock_field = self
            .0
            .user_lock_field
            .clone()
            .unwrap_or("locked".to_owned());
        let state_field = self
            .0
            .user_state_field
            .clone()
            .unwrap_or("user_state".to_owned());
        if let Some(lock) = user.get(lock_field) {
            if lock.is_boolean() && lock.as_bool().unwrap_or_default()
                || (lock.is_number() && lock.as_u64().unwrap_or_default() != 0)
                || (lock.is_string()
                    && (lock.as_str() == Some("yes")
                        || lock.as_str() == Some("true")
                        || lock.as_str() == Some("locked")))
            {
                return Err(anyhow!("error.user.locked"));
            }
        }

        if let Some(state) = user.get(state_field) {
            if (state.is_boolean() && !state.as_bool().unwrap_or_default())
                || (state.is_number() && state.as_u64().unwrap_or_default() == 0)
                || (state.is_string()
                    && (state.as_str() == Some("yes")
                        || state.as_str() == Some("true")
                        || state.as_str() == Some("disabled")))
            {
                return Err(anyhow!("error.user.disabled"));
            }
        }

        // Verify Password.
        // Actually, the password in the store was saved by md5 of encryted password
        if let Some(pwd) = user.get(passwd_field) {
            let pwd_text = pwd.as_str();
            if let Some(strpwd) = pwd_text {
                let authcp = self.credential_preprocess(auth);
                let authcp_credential = authcp.credential.unwrap_or_default();
                // let strpwd = pwdtext;
                // 对于RSA加密，它是包含有随机数情况的，所以每次加密所产生的结果都是不一样的，所以，对两次的结果进行解密，然后再进行比较

                if self.0.credential_hash_method == Some("mix".to_owned()) {
                    let newpwd = md5_hash(&format!(
                        "{strpwd}_{}",
                        auth.captcha_code.clone().unwrap_or_default()
                    ));
                    if authcp_credential != newpwd {
                        return Err(anyhow!("error.password.wrong"));
                    }
                } else if self.0.credential_hash_method == Some("rsa".to_owned()) {
                    let prikey = &self.0.rsa_private_key.clone().unwrap_or_default();
                    if let Some(decpwd) = rsa_decrypt_with_private_key(strpwd, prikey) {
                        if let Some(decenc) =
                            rsa_decrypt_with_private_key(&authcp_credential, prikey)
                        {
                            if decenc != decpwd {
                                return Err(anyhow!("error.password.wrong"));
                            }
                        }
                    } else {
                        log::info!("could not be decode by rsa.");
                        return Err(anyhow!("error.password.wrong"));
                    }
                } else if authcp_credential != strpwd {
                    return Err(anyhow!("error.password.wrong"));
                };
            } else {
                return Err(anyhow!("error.password.store.empty"));
            }
        } else {
            return Err(anyhow!("error.password.empty"));
        }

        Ok(true)
    }

    /**
     * 修改密码
     * 1、提供一个机制来进行密码的修改
     * 2、重点是根据配置重新生成密码
     * 3、对Value进行指定字段的复制
     */
    pub async fn change_passwd(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        user: &Value,
        pwd: &ChangePasswordRequest,
    ) -> Result<bool, anyhow::Error> {
        let passwd_field = self
            .0
            .user_credentials_field
            .clone()
            .unwrap_or("password".to_owned());
        let userid_field = self.0.userid_field.clone().unwrap_or("user_id".to_owned());
        let resetpwd_field = self
            .0
            .reset_pwd_field
            .clone()
            .unwrap_or("reset_password_time".to_owned());
        let new_passwd =
            json!(self.encrypt_password(&pwd.new_credential, pwd.captcha_code.clone()));

        let uri = self.0.user_search.clone().unwrap_or_default();
        let uri_r = if let Some(idx) = uri.find('#') {
            uri.substring(0, idx).to_owned()
        } else {
            uri
        };

        let uri_act = format!("{uri_r}#update");

        let pwdval = json!({&userid_field: user.get(&userid_field), &passwd_field: new_passwd, &resetpwd_field: rbdc::datetime::DateTime::now() });
        log::info!("pwd: {pwdval:?}");
        let _ = MxStoreService::invoke_return_one(uri_act, ctx, vec![pwdval]).await?;
        Ok(true)
    }

    pub fn desensitize_user(&self, user: &Value) -> Value {
        let pwd_field = self
            .0
            .user_credentials_field
            .clone()
            .unwrap_or("password".to_owned());
        copy_value_excluded(user, &[pwd_field])
    }
}

pub struct AuthUserRole(pub bool);

#[async_trait]
impl Handler for AuthUserRole {
    #[doc = " Handle http request."]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        let path = req.uri().path();
        let sppath = path.split('/').filter(|p| !p.is_empty()).collect_vec();
        let api = sppath[0];

        if api == "api" {
            if self.0 {
                let jwt = depot
                    .jwt_auth_data::<JwtUserClaims>()
                    .map(|f| f.claims.clone())
                    .unwrap_or(JwtUserClaims::anonymous());
                let user_roles = match depot.get::<Vec<String>>("_USER_ROLES") {
                    Ok(ts) => ts.to_owned(),
                    Err(err) => {
                        log::debug!("error on get  _USER_ROLES {err:?}");
                        vec![]
                    }
                };

                let auth_service = AuthorizationService(AuthorizationConfig::get());

                let schema = sppath[1];

                if matches!(schema, "file" | "auth" | "execute") || sppath.len() < 5 {
                    if depot.jwt_auth_state() == JwtAuthState::Authorized {
                        ctrl.call_next(req, depot, res).await;
                        return;
                    } else if sppath.len() >= 3 && (sppath[2] == "tasks" || sppath[2] == "query") {
                        if !auth_service.0.enable {
                            ctrl.call_next(req, depot, res).await;
                            return;
                        } else {
                            let _ = Json(ApiResult::<String>::error(401, "Unauthorized"))
                                .write(req, depot, res)
                                .await;
                            ctrl.skip_rest();
                            return;
                        }
                    } else if !auth_service.0.enable || auth_service.0.fail_bypass {
                        ctrl.call_next(req, depot, res).await;
                        return;
                    } else {
                        let _ = Json(ApiResult::<String>::error(401, "Unauthorized"))
                            .write(req, depot, res)
                            .await;
                        ctrl.skip_rest();
                        return;
                    }
                }

                let ns = sppath[2];
                let name = sppath[3];
                let mth = sppath[4];
                let invoke_uri = InvokeUri {
                    schema: schema.to_owned(),
                    namespace: ns.to_owned(),
                    object: name.to_owned(),
                    method: mth.to_owned(),
                    query: None,
                    query_pairs: None,
                };

                // log::info!("check permission {schema}");

                if !auth_service.0.enable {
                    // skip all permission check when the auth is disabled.
                    ctrl.call_next(req, depot, res).await;
                    return;
                }

                //
                // 匿名用户，如果jwt中是一个匿名用户，则只添加role_anonymous角色，
                // 如果是登录用户，则它必须是一个role_commonuser和role_anonymous
                //
                let uroles = if !jwt.is_anonymous() && user_roles.is_empty() {
                    let username = jwt.username.clone();

                    let org = if auth_service.0.enable_organization {
                        Some(jwt.domain.clone())
                    } else {
                        None
                    };

                    if auth_service.0.enable {
                        let ctx = Arc::new(Mutex::new(InvocationContext::new()));
                        let ret = (auth_service.find_user_roles(ctx, &username, &org).await)
                            .unwrap_or_default();

                        let mut us = ret.clone();
                        if !us.contains(&"ROLE_COMMONUSER".to_owned()) {
                            us.push("ROLE_COMMONUSER".to_owned());
                        }
                        if !us.contains(&"ROLE_ANONYMOUS".to_owned()) {
                            us.push("ROLE_ANONYMOUS".to_owned());
                        }
                        depot.insert("_USER_ROLES", us.clone());
                        us
                    } else {
                        let us = vec![
                            "ROLE_COMMONUSER".to_owned(),
                            "ROLE_API_CALLER".to_owned(),
                            "ROLE_ANONYMOUS".to_owned(),
                        ];
                        depot.insert("_USER_ROLES", us.clone());
                        us
                    }
                } else if jwt.is_anonymous() {
                    let mut us = user_roles.clone();
                    us.push("ROLE_ANONYMOUS".to_owned());
                    us
                } else {
                    user_roles
                };

                // log::warn!("Current user's role: {uroles:?}");

                // 如果没有权限，则返回并执行对应的Response
                if !MxStoreService::check_rolebase_permission(&invoke_uri, &jwt, &uroles, false) {
                    // permission denined
                    let _ = Json(ApiResult::<String>::error(403, "Permssion denined"))
                        .write(req, depot, res)
                        .await;
                    ctrl.skip_rest();
                    return;
                }
            } else {
                // the first is passoff
                let schema = sppath[2];
                let ns = sppath[3];
                let name = sppath[4];
                let mth = sppath[5];
                let invoke_uri = InvokeUri {
                    schema: schema.to_owned(),
                    namespace: ns.to_owned(),
                    object: name.to_owned(),
                    method: mth.to_owned(),
                    query: None,
                    query_pairs: None,
                };

                let jwt = JwtUserClaims::anonymous();
                // log::warn!("Current user's role is ROLE_ANONYMOUS");
                if !MxStoreService::check_rolebase_permission(
                    &invoke_uri,
                    &jwt,
                    &["ROLE_ANONYMOUS".to_owned()],
                    true,
                ) {
                    // permission denined
                    let _ = Json(ApiResult::<String>::error(403, "Permssion denined"))
                        .write(req, depot, res)
                        .await;
                    ctrl.skip_rest();
                    return;
                }
            }
        }
        ctrl.call_next(req, depot, res).await;
    }
}
