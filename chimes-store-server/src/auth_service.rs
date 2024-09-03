use crate::api::{ChangePasswordRequest, UserAuthRequest};
use anyhow::{anyhow, Result};
use chimes_store_core::{
    config::{
        auth::{AppSecretPair, AuthorizationConfig, JwtUserClaims},
        ConditionItem, QueryCondition,
    },
    service::{
        invoker::InvocationContext, registry::SchemaRegistry, sdk::InvokeUri,
        starter::MxStoreService,
    },
    utils::{
        copy_to_slice,
        global_data::{
            copy_value_excluded, rsa_decrypt_with_private_key, rsa_encrypt_with_public_key,
        },
        ApiResult,
    },
};
use chimes_store_dbs::utils::{ase_encrypt_to_text, md5_hash, sha1_256_hash, sha2_256_hash};
use itertools::Itertools;
use rbatis::rbdc;
use salvo::{
    async_trait,
    jwt_auth::{JwtAuthDepotExt, JwtAuthState},
    writing::Json,
    Depot, FlowCtrl, Handler, Request, Response, Writer,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use substring::Substring;

pub struct AuthorizationService(pub(crate) AuthorizationConfig);
unsafe impl Send for AuthorizationService {}
unsafe impl Sync for AuthorizationService {}

impl AuthorizationService {
    /**
     * 对密码进行相应的处理
     */
    pub fn credential_preprocess(&self, user: &UserAuthRequest) -> UserAuthRequest {
        let credential = user.credential.clone().unwrap_or_default();
        if let Some(l) = credential.find(':') {
            log::info!("found : at {}", l);
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
                ase_encrypt_to_text(&org_pwd.as_bytes(), &key, &iv)
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
                let enc = ase_encrypt_to_text(&org_pwd.as_bytes(), &key, &iv);
                md5_hash(&format!("{enc}_{}", captha_code.unwrap_or_default()))
            }
            "rsa" => rsa_encrypt_with_public_key(
                org_pwd,
                &conf.credential_key.clone().unwrap_or_default(),
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
    ) -> Result<Option<Value>> {
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
    ) -> Result<Option<Value>> {

        let cpv = match user.as_object() {
            Some(tm) => {
                let mut cps = vec![];
                if self.0.enable_organization && organization.is_some() {
                    cps.push(ConditionItem {
                        field: self.0.organization_field.clone().unwrap_or("organization".to_owned()),
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
            },
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
                .await {
                Ok(us) => {
                    if us.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(us[0].clone()))
                    }
                },
                Err(err) => Err(err)
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
    ) -> Result<Vec<String>> {
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

        log::info!("rolesearch: {rolesearch}");

        let roles = if rolesearch.starts_with("query://") {
            let qcond = if self.0.enable_organization {
                json!({&username_field: username, org_field: json!(organization)})
            } else {
                json!({&username_field: username})
            };

            MxStoreService::invoke_return_vec(
                rolesearch,
                ctx,
                vec![qcond],
            )
            .await?
        } else {
            let qs = if self.0.enable_organization {
                QueryCondition {
                    and: vec![ConditionItem {
                        field: username_field.to_owned(),
                        op: "=".to_owned(),
                        value: Value::String(username.to_owned()),
                        ..Default::default()
                    }, ConditionItem {
                        field: org_field.to_owned(),
                        op: "=".to_owned(),
                        value: json!(organization),
                        ..Default::default()
                    }],
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
                MxStoreService::invoke_return_vec(
                    rolesearch,
                    ctx,
                    vec![serde_json::to_value(qs)?],
                )
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
    pub async fn find_appid_secret(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        appid: &str,
        secret: &str,
    ) -> Result<Vec<AppSecretPair>> {
        let asprov = self.0.appsecret_provider.clone().unwrap_or_default();
        if asprov.is_empty() {
            let appids: Vec<AppSecretPair> = self.0.app_secret_keys.clone().iter().filter(|p| p.app_id == *appid && p.app_secret == *secret).map(|t| t.to_owned()).collect();
            Ok(appids)
        } else {
            let ci = ConditionItem {
                field: "app_id".to_owned(),
                op: "=".to_owned(),
                value: json!(appid),
                ..Default::default()
            };

            let scrt = ConditionItem {
                field: "app_secret".to_owned(),
                op: "=".to_owned(),
                value: json!(secret),
                ..Default::default()
            };

            let qs = QueryCondition {
                and: vec![ci, scrt],
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
                    let tvxs: Vec<AppSecretPair> =  tvs.iter().map(|t| serde_json::from_value::<AppSecretPair>(t.to_owned()).unwrap_or(AppSecretPair::default()))
                                                              .filter(|f| !f.app_id.is_empty() && !f.app_secret.is_empty())
                                                              .collect();
                    Ok(tvxs)
                },
                Err(err) => {
                    Err(err)
                }
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
    pub fn credential_validate(&self, user: &Value, auth: &UserAuthRequest) -> Result<bool> {
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
            if pwd_text.is_none() {
                return Err(anyhow!("error.password.store.empty"));
            } else {
                let authcp = self.credential_preprocess(auth);
                let authcp_credential = authcp.credential.unwrap_or_default(); 
                let strpwd = pwd_text.unwrap().to_string();
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
                    let prikey = &self.0.credential_solt.clone().unwrap_or_default();
                    if let Some(decpwd) = rsa_decrypt_with_private_key(&strpwd, prikey) {
                        if let Some(decenc) = rsa_decrypt_with_private_key(&authcp_credential, prikey)
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

        let uri_act = format!("{}#update", uri_r);

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
    #[must_use = "handle future must be used"]
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
        if self.0 {
            log::info!("Should check the permission for each request.");
        } else {
            log::info!("Bypass this request.");
        }
        if api == "api" {
            if self.0 {
                let jwt = depot
                    .jwt_auth_data::<JwtUserClaims>()
                    .map(|f| f.claims.clone())
                    .unwrap_or(JwtUserClaims::anonymous());
                let user_roles = match depot.get::<Vec<String>>("_USER_ROLES") {
                    Ok(ts) => ts.to_owned(),
                    Err(err) => {
                        log::debug!("error on get  _USER_ROLES {:?}", err);
                        vec![]
                    }
                };

                let schema = sppath[1];

                if matches!(schema, "file" | "auth" | "execute") || sppath.len() < 5 {
                    if depot.jwt_auth_state() == JwtAuthState::Authorized {
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
                };

                log::info!("check permission {schema}");

                //
                // 匿名用户，如果jwt中是一个匿名用户，则只添加role_anonymous角色，
                // 如果是登录用户，则它必须是一个role_commonuser和role_anonymous
                //
                let uroles = if !jwt.is_anonymous() && user_roles.is_empty() {
                    let username = jwt.username.clone();
                    let auth_service = AuthorizationService(AuthorizationConfig::get());
                    let org = if auth_service.0.enable_organization {
                        Some(jwt.domain.clone())
                    } else {
                        None
                    };

                    if auth_service.0.enable {
                        let ctx = Arc::new(Mutex::new(InvocationContext::new()));
                        let ret = match auth_service.find_user_roles(ctx, &username, &org).await {
                            Ok(ts) => ts,
                            Err(_) => {
                                vec![]
                            }
                        };

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
                        let mut us = vec![];
                        us.push("ROLE_COMMONUSER".to_owned());
                        us.push("ROLE_API_CALLER".to_owned());
                        us.push("ROLE_ANONYMOUS".to_owned());
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

                log::debug!("Current user's role: {:?}", uroles);

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
                };

                let jwt = JwtUserClaims::anonymous();
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
