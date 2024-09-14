use anyhow::anyhow;
use chimes_store_core::config::auth::JwtUserClaims;
use chimes_store_core::config::{MethodHook, PluginConfig};
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::sdk::{
    InvokeUri, MethodDescription, RxHookInvoker, RxPluginService,
};
use chimes_store_core::service::starter::{load_config, MxStoreService};
use chimes_store_core::utils::global_data::global_app_data_insert_with_expire;
use chimes_store_core::utils::redis::{redis_get, redis_set_expire};
use rbatis::Page;
use reqwest::Method;
use salvo::oapi::{
    schema, Content, Object, OpenApi, Operation, PathItem, RefOr, RequestBody, Response, Schema,
    ToArray,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use crate::proc::template::{json_path_get, template_eval};

use super::reqwest::RestHttpClient;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RestApiServiceInfo {
    pub name: String,
    pub rest_url: String,
    pub rest_method: Option<String>,
    pub rest_desc: Option<String>,

    #[serde(default)]
    pub rest_api: bool,
    #[serde(default)]
    pub no_access_token: bool, // 为true则表示不附加AccessToken

    pub rest_body: String,
    pub rest_content_type: Option<String>,
    pub return_validate: Option<String>,
    pub return_data: Option<String>,

    /**
     * 该接口的返回值可以用于本系统的登录
     * 如果设置了该接口的返回值可以用于登录后，需要将参数中的某个值，作为 captcha_id，同时，返回值中的某个值作为captcha_code
     */
    #[serde(default)]
    pub use_auth: bool,
    pub captcha_id_express: Option<String>,
    pub captcha_code_express: Option<String>,

    #[serde(default)]
    pub perm_roles: Vec<String>,

    #[serde(default)]
    pub bypass_permission: bool, // 允许匿名访问，只有在允许匿名访问的时候，才能通过passoff调用

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<MethodHook>,
}

unsafe impl Sync for RestApiServiceInfo {}

unsafe impl Send for RestApiServiceInfo {}

fn to_api_result_schema(t: RefOr<Schema>, array: bool) -> schema::Schema {
    let mut apiresult = Object::new();
    apiresult = apiresult.property(
        "status",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "message",
        Object::new().schema_type(schema::BasicType::String),
    );
    if array {
        apiresult = apiresult.property("data", t.to_array());
    } else {
        apiresult = apiresult.property("data", t);
    }
    apiresult = apiresult.property(
        "timestamp",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    schema::Schema::Object(apiresult)
}

impl RestApiServiceInfo {
    pub(crate) fn to_operation(&self, array: bool) -> Operation {
        let mut ins_op = Operation::new();
        ins_op = ins_op.request_body(
            RequestBody::new()
                .add_content("application/json", RefOr::Type(Schema::Object(Object::new()))),
        );
        ins_op = ins_op.summary(self.rest_desc.clone().unwrap_or_default());

        let mut description = "方法在执行的时候会根据URL中所传递的Query参数，组装成第一个参数（根据参数名转成JSON Object），从Request Body接收第二个参数（必须为JSON对象），模板中须按这个规则来处理对应的参数。".to_string();
        description.push_str("在请求模板中需要使用args[0]来访问Query String传递过来的参数，使用args[1]来访问Request Body传递过来的参数。");
        description.push_str(&format!("该方法将请求{}接口。", self.rest_url));
        if self.bypass_permission {
            description.push_str("该方法可以被匿名调用。");
        }

        ins_op = ins_op.description(description);

        let mut resp = Response::new(format!("返回{}接口请求成功后处理的数据。", self.rest_url));
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(
                RefOr::Type(schema::Schema::Object(Object::new())),
                array,
            )),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));
        ins_op
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RestapiPluginConfig {
    pub app_id: Option<String>,
    pub app_secret: Option<String>,
    pub api_server: Option<String>,
    #[serde(default)]
    pub enable_oauth2: bool,
    pub oauth2_validate_url: Option<String>,
    pub oauth2_request_method: Option<String>,
    pub oauth_request_type: Option<String>,
    pub oauth2_request_body: Option<String>,
    pub oauth2_token_express: Option<String>,
    pub oauth2_expired_express: Option<String>,
    pub token_pass_style: Option<String>,
    pub token_identifier: Option<String>,
    pub custom_headers: Option<String>,
    #[serde(default)]
    pub accept_invalid_certs: bool,    
    pub services: Vec<RestApiServiceInfo>,
}

unsafe impl Sync for RestapiPluginConfig {}

unsafe impl Send for RestapiPluginConfig {}

impl RestapiPluginConfig {
    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<RestApiServiceInfo> {
        self.services
            .clone()
            .into_iter()
            .filter(|p| p.name == *name)
            .last()
    }
}

#[allow(dead_code)]
pub struct RestapiPluginService {
    namespace: String,
    conf: PluginConfig,
    restconf: Mutex<Option<RestapiPluginConfig>>,
    service_map: HashMap<String, RestApiServiceInfo>,
}

impl RestapiPluginService {
    #[allow(dead_code)]
    pub fn new(ns: &str, conf: &PluginConfig) -> Result<Self, anyhow::Error> {
        log::debug!("Plugin config load from {}", conf.config.clone());
        let t = match load_config(conf.config.clone()) {
            Ok(r) => r,
            Err(err) => {
                log::debug!("Could not load the config file: {:?}", err);
                Some(RestapiPluginConfig::default())
            }
        };

        let mut map = HashMap::new();

        if let Some(tcplc) = t.clone() {
            tcplc.services.into_iter().for_each(|f| {
                map.insert(f.name.clone(), f);
            });
        }

        Ok(Self {
            namespace: ns.to_owned(),
            conf: conf.to_owned(),
            restconf: Mutex::new(t),
            service_map: map,
        })
    }

    #[allow(dead_code)]
    pub fn get_named_service(&self, name: &str) -> Option<RestApiServiceInfo> {
        self.service_map.get(name).cloned()
    }

    #[allow(dead_code)]
    pub fn get_names(&self) -> Vec<String> {
        self.service_map.keys().cloned().collect()
    }

    #[allow(dead_code)]
    pub fn get_services(&self) -> Vec<RestApiServiceInfo> {
        self.service_map.values().cloned().collect()
    }

    fn to_openapi_doc(&self, ns: &str) -> OpenApi {
        let mut openapi = OpenApi::new(self.conf.name.clone(), "0.1.0");
        let tconf = self.restconf.lock().unwrap().clone();
        if let Some(restconf) = tconf {
            for tsvc in restconf
                .services
                .iter()
                .filter(|f| !f.bypass_permission)
                .cloned()
            {
                let opt = tsvc.to_operation(false);
                let one_path = format!(
                    "/api/restapi/{}/{}/{}/single",
                    ns,
                    self.conf.name.clone(),
                    tsvc.name.clone()
                );
                openapi = openapi.add_path(
                    one_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                );
                openapi = openapi.add_path(
                    one_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Get, opt.clone()),
                );

                let opt = tsvc.to_operation(true);
                let list_path = format!(
                    "/api/restapi/{}/{}/{}/list",
                    ns,
                    self.conf.name.clone(),
                    tsvc.name.clone()
                );
                openapi = openapi.add_path(
                    list_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                );
                openapi = openapi.add_path(
                    list_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Get, opt.clone()),
                );
            }
            for tsvc in restconf
                .services
                .iter()
                .filter(|f| f.bypass_permission)
                .cloned()
            {
                let opt = tsvc.to_operation(false);
                let one_path = format!(
                    "/api/passoff/restapi/{}/{}/{}/single",
                    ns,
                    self.conf.name.clone(),
                    tsvc.name.clone()
                );
                openapi = openapi.add_path(
                    one_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                );
                openapi = openapi.add_path(
                    one_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Get, opt.clone()),
                );

                let opt = tsvc.to_operation(true);
                let list_path = format!(
                    "/api/passoff/restapi/{}/{}/{}/list",
                    ns,
                    self.conf.name.clone(),
                    tsvc.name.clone()
                );
                openapi = openapi.add_path(
                    list_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                );
                openapi = openapi.add_path(
                    list_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Get, opt.clone()),
                );
            }
        }
        openapi
    }

    /**
     * 执行获取access_token
     */
    #[allow(dead_code)]
    pub async fn get_access_token(
        ns: &str,
        name: &str,
        restconf: &RestapiPluginConfig,
    ) -> Option<String> {
        let redis_key = format!("restapi://{}/{}-access-token", ns, name);
        if let Ok(Some(t)) = redis_get(ns, &redis_key) {
            return Some(t);
        }

        log::info!("get access_token by oauth2 request.");

        let opt = restconf
            .custom_headers
            .clone()
            .map(|v| serde_json::from_str::<Value>(&v).map(|f| {
                match f {
                    Value::Object(mut mt) => {
                       if restconf.accept_invalid_certs {
                         mt.insert("accept_invalid_certs".to_string(), Value::Bool(true));
                       }
                       Value::Object(mt) 
                    },
                    _ => {
                        json!({"accept_invalid_certs": restconf.accept_invalid_certs })
                    }
                }
            }).unwrap_or(Value::Null));
        let req_url = format!(
            "{}{}",
            restconf.api_server.clone().unwrap_or_default(),
            restconf.oauth2_validate_url.clone().unwrap_or_default()
        );
        let method = Method::from_str(
            &restconf
                .oauth2_request_method
                .clone()
                .unwrap_or("GET".to_string()),
        )
        .unwrap_or(Method::GET);
        let data = restconf.oauth2_request_body.clone().unwrap_or_default();
        let fmt = restconf.oauth_request_type.clone().unwrap_or_default();
        // let app_id = restconf.app_id.clone().unwrap_or_default();
        // let app_secret = restconf.app_secret.clone().unwrap_or_default();
        let text = template_eval(&data, json!({"config": restconf.clone()}))
            .unwrap_or_default();
        match RestHttpClient::send_http_request(&req_url, method, &text, &fmt, &opt).await {
            Ok(t) => {
                if let Some(t) = t {
                    log::info!("respose: {t}");
                    let expire_path = restconf
                        .oauth2_expired_express
                        .clone()
                        .unwrap_or("$.expire_in".to_owned());
                    let token_path = restconf
                        .oauth2_token_express
                        .clone()
                        .unwrap_or("$.access_token".to_owned());

                    let expire = json_path_get(&t, &expire_path)
                        .unwrap_or(json!(60))
                        .as_u64()
                        .unwrap_or(60u64);

                    if let Some(Value::String(tv)) = json_path_get(&t, &token_path) {
                        if let Err(err) = redis_set_expire(ns, &redis_key, &tv, expire - 30) {
                            log::debug!("error for set expire {err}");
                        }
                        Some(tv.to_owned())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Err(err) => {
                log::info!("request {} with err {}", req_url, err);
                None
            }
        }
    }
}

/**
 * 调用函数
 * 通过这种方式可以解除&self的引用，从而可以将有些引用打散
 */
pub async fn call_rest_api(
    restconf: &RestapiPluginConfig,
    plc: &RestApiServiceInfo,
    _ctx: Arc<Mutex<InvocationContext>>,
    access_token: Option<String>,
    args: &[Value],
) -> Result<Option<Value>, anyhow::Error> {
    log::info!("calling rest api {}", plc.rest_url);
    let no_access_token = plc.no_access_token;

    let mut opt = restconf
        .custom_headers
        .clone()
        .map(|v| serde_json::from_str::<Value>(&v).unwrap_or(Value::Null));
    let mut req_url = format!(
        "{}{}",
        restconf.api_server.clone().unwrap_or_default(),
        plc.rest_url.clone()
    );
    if !no_access_token {
        if restconf.token_pass_style == Some("Query".to_owned()) {
            if req_url.contains('?') {
                req_url.push_str(&format!(
                    "&{}={}",
                    restconf
                        .token_identifier
                        .clone()
                        .unwrap_or("access_token".to_owned()),
                    access_token.clone().unwrap_or_default()
                ));
            } else {
                req_url.push_str(&format!(
                    "?{}={}",
                    restconf
                        .token_identifier
                        .clone()
                        .unwrap_or("access_token".to_owned()),
                    access_token.clone().unwrap_or_default()
                ));
            }
        } else {
            let optc = opt.unwrap_or(Value::Null);
            let mut optmap = optc.as_object().map(|f| f.to_owned()).unwrap_or_default();
            if restconf.token_pass_style == Some("Cookie".to_owned()) {
                let cookie = format!(
                    "&{}={}",
                    restconf
                        .token_identifier
                        .clone()
                        .unwrap_or("access_token".to_owned()),
                    access_token.clone().unwrap_or_default()
                );
                optmap.insert("cookie".to_owned(), Value::String(cookie));
            } else if let Some(headermap) = optmap.get_mut("header") {
                if let Some(hmap) = headermap.as_object_mut() {
                    hmap.insert(
                        restconf
                            .token_identifier
                            .clone()
                            .unwrap_or("access_token".to_owned()),
                        Value::String(access_token.unwrap_or_default()),
                    );
                } else {
                    let json = json!({restconf.token_identifier.clone().unwrap_or("access_token".to_owned()): access_token.unwrap_or_default()});
                    optmap.insert("header".to_string(), json);
                }
            } else {
                let json = json!({restconf.token_identifier.clone().unwrap_or("access_token".to_owned()): access_token.unwrap_or_default()});
                optmap.insert("header".to_string(), json);
            }
            if restconf.accept_invalid_certs {
                optmap.insert("accept_invalid_certs".to_string(), Value::Bool(true));
            }
            opt = Some(Value::Object(optmap));
        }
    }

    let method = Method::from_str(&plc.rest_method.clone().unwrap_or("GET".to_string()))
        .unwrap_or(Method::GET);
    let data = plc.rest_body.clone();
    let fmt = plc
        .rest_content_type
        .clone()
        .unwrap_or("json".to_owned())
        .to_lowercase();

    //let app_id = restconf.app_id.clone().unwrap_or_default();
    //let app_secret = restconf.app_secret.clone().unwrap_or_default();
    let ctx_val = json!({ "config": restconf.clone(), "args": args });
    let text = template_eval(
        &data,
        ctx_val.clone(),
    )
    .unwrap_or_default();


    match RestHttpClient::send_http_request(&req_url, method, &text, &fmt, &opt).await {
        Ok(t) => {
            if let Some(t) = t {
                log::info!("respose: {t}");
                if let Some(ret_validate) = plc.return_validate.clone() {
                    let ret_val = json_path_get(&t, &ret_validate);
                    if ret_val.is_none() {
                        return Err(anyhow!("return data was not validated"));
                    }
                }

                let ret_ctx_val = json!({ "config": restconf.clone(), "args": args, "ret": t.clone() });

                let captcha_id = if plc.use_auth {
                    let res = template_eval(&plc.captcha_id_express.clone().unwrap_or_default(), ret_ctx_val.clone()).unwrap_or_default();
                    Some(res)
                } else {
                    None
                };

                let captcha_code = if plc.use_auth {
                    let res = template_eval(&plc.captcha_code_express.clone().unwrap_or_default(), ret_ctx_val).unwrap_or_default();
                    Some(res)
                } else {
                    None
                };

                if let Some(rt) = plc.return_data.clone() {
                    let ret = json_path_get(&t, &rt);
                    if plc.use_auth && captcha_code.is_some() && captcha_id.is_some() {
                        global_app_data_insert_with_expire(&captcha_id.unwrap_or_default(), &captcha_code.unwrap_or_default(), 6000);
                    }
                    Ok(ret)
                } else {
                    if plc.use_auth && captcha_code.is_some() && captcha_id.is_some() {
                        global_app_data_insert_with_expire(&captcha_id.unwrap_or_default(), &captcha_code.unwrap_or_default(), 6000);
                    }
                    Ok(Some(t))
                }
            } else {
                Ok(None)
            }
        }
        Err(err) => {
            log::info!("request {} with err {}", req_url, err);
            Err(anyhow!(err))
        }
    }
}

fn option_value_to_vec_value(ret: &Option<Value>) -> Vec<Value> {
    match ret {
        None => vec![],
        Some(tt) => {
            if tt.is_array() {
                if let Some(tvec) = tt.as_array() {
                    tvec.to_owned()
                } else {
                    vec![]
                }
            } else {
                vec![tt.to_owned()]
            }
        }
    }
}

impl RxPluginService for RestapiPluginService {
    fn invoke_return_option(
        &'_ self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, anyhow::Error>> + Send>> {
        // Compose
        let ns = self.namespace.clone();
        let name = self.conf.name.clone();
        let full_uri = uri.url();
        let st_uri = uri.clone();
        let restconf = self.restconf.lock().unwrap().clone();
        let named_service = self.get_named_service(&st_uri.method);
        Box::pin(async move {
            if let Some(restconf) = restconf {
                let access_token = Self::get_access_token(&ns, &name, &restconf).await;
                if let Some(plc) = named_service {
                    let mix_args = MxStoreService::invoke_pre_hook_(
                        full_uri.clone(),
                        plc.hooks.clone(),
                        ctx.clone(),
                        args.clone(),
                    )
                    .await?;
                    match call_rest_api(&restconf, &plc, ctx.clone(), access_token, &mix_args).await
                    {
                        Ok(ret) => {
                            if plc.hooks.is_empty() {
                                Ok(ret)
                            } else {
                                ctx.lock().unwrap().insert("RETURN_VALUE", ret.clone());
                                MxStoreService::invoke_post_hook_(
                                    full_uri.clone(),
                                    plc.hooks.clone(),
                                    ctx.clone(),
                                    mix_args,
                                )
                                .await?;
                                match ctx.lock().unwrap().get::<Option<Value>>("RETURN_VALUE") {
                                    Ok(mt) => Ok(mt.to_owned()),
                                    Err(_) => Ok(ret),
                                }
                            }
                        }
                        Err(err) => {
                            if !plc.hooks.is_empty() {
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                MxStoreService::invoke_post_hook_(
                                    full_uri.clone(),
                                    plc.hooks.clone(),
                                    ctx.clone(),
                                    mix_args,
                                )
                                .await?;
                            }
                            Err(err)
                        }
                    }
                } else {
                    Err(anyhow!("Not Found {:?}", st_uri.url()))
                }
            } else {
                Err(anyhow!("Not Found {:?}", st_uri.url()))
            }
        })
    }

    fn invoke_return_vec(
        &self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, anyhow::Error>> + Send>> {
        // Compose
        let ns = self.namespace.clone();
        let name = self.conf.name.clone();
        let full_uri = uri.url();
        let st_uri = uri.clone();
        let restconf = self.restconf.lock().unwrap().clone();
        let named_service = self.get_named_service(&st_uri.method);
        Box::pin(async move {
            if let Some(restconf) = restconf {
                let access_token = Self::get_access_token(&ns, &name, &restconf).await;
                if let Some(plc) = named_service {
                    let mix_args = MxStoreService::invoke_pre_hook_(
                        full_uri.clone(),
                        plc.hooks.clone(),
                        ctx.clone(),
                        args.clone(),
                    )
                    .await?;
                    match call_rest_api(&restconf, &plc, ctx.clone(), access_token, &mix_args).await
                    {
                        Ok(ret) => {
                            if plc.hooks.is_empty() {
                                Ok(option_value_to_vec_value(&ret))
                            } else {
                                ctx.lock().unwrap().insert("RETURN_VALUE", ret.clone());
                                MxStoreService::invoke_post_hook_(
                                    full_uri.clone(),
                                    plc.hooks.clone(),
                                    ctx.clone(),
                                    mix_args,
                                )
                                .await?;
                                match ctx.lock().unwrap().get::<Option<Value>>("RETURN_VALUE") {
                                    Ok(mt) => Ok(option_value_to_vec_value(mt)),
                                    Err(_) => Ok(option_value_to_vec_value(&ret)),
                                }
                            }
                        }
                        Err(err) => {
                            if !plc.hooks.is_empty() {
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                MxStoreService::invoke_post_hook_(
                                    full_uri.clone(),
                                    plc.hooks.clone(),
                                    ctx.clone(),
                                    mix_args,
                                )
                                .await?;
                            }
                            Err(err)
                        }
                    }
                } else {
                    Err(anyhow!("Not Found {:?}", st_uri.url()))
                }
            } else {
                Err(anyhow!("Not Found {:?}", st_uri.url()))
            }
        })
    }

    fn invoke_return_page(
        &self,
        uri: InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, anyhow::Error>> + Send>> {
        let st_uri = uri.clone();
        Box::pin(async move { Err(anyhow!("Not Found {:?}", st_uri.url())) })
    }

    fn get_config(&self) -> Option<Value> {
        match serde_json::to_value(self.restconf.lock().unwrap().clone()) {
            Ok(t) => Some(t),
            Err(err) => {
                log::debug!("Convert to json with error: {:?}", err);
                None
            }
        }
    }

    fn parse_config(&self, val: &Value) -> Result<(), anyhow::Error> {
        match serde_json::from_value::<RestapiPluginConfig>(val.to_owned()) {
            Ok(t) => {
                self.restconf.lock().unwrap().replace(t);
                // self.compose.replace(Some(t));
                Ok(())
            }
            Err(err) => {
                log::warn!("Parse JSON value to config with error: {:?}", err);
                Err(anyhow!(err))
            }
        }
    }

    fn save_config(&self, conf: &PluginConfig) -> Result<(), anyhow::Error> {
        let path: PathBuf = conf.config.clone().into();
        chimes_store_core::service::starter::save_config(
            &self.restconf.lock().unwrap().clone(),
            path,
        )
    }

    fn get_metadata(&self) -> Vec<chimes_store_core::service::sdk::MethodDescription> {
        let mut desc = vec![];
        if let Some(compse) = self.restconf.lock().unwrap().clone() {
            for svc in compse.services.clone() {
                desc.push(MethodDescription {
                    uri: format!(
                        "{}://{}/{}",
                        self.conf.protocol.clone(),
                        self.namespace.clone(),
                        self.conf.name
                    ),
                    name: svc.name.clone(),
                    func: None,
                    params_vec: true,
                    params1: vec![],
                    params2: None,
                    response: vec![],
                    return_page: false,
                    return_vec: true,
                });
            }
        }
        desc
    }

    fn get_openapi(&self, ns: &str) -> Box<dyn std::any::Any> {
        Box::new(self.to_openapi_doc(ns))
    }

    fn has_permission(
        &self,
        uri: &InvokeUri,
        _jwt: &JwtUserClaims,
        roles: &[String],
        bypass: bool,
    ) -> bool {
        log::info!("{bypass}, {roles:?}, {}", uri.method);
        if let Some(restconf) = self.restconf.lock().unwrap().to_owned() {
            for it in restconf.services.iter().filter(|p| p.name == uri.method) {
                if bypass && it.bypass_permission {
                    return true;
                } else {
                    log::info!("{:?}", it.perm_roles);
                    if it.perm_roles.is_empty() {
                        return true;
                    }

                    if it
                        .perm_roles
                        .clone()
                        .into_iter()
                        .any(|f| roles.contains(&f))
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
}
