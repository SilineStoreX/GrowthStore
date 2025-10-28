use anyhow::anyhow;
use chimes_store_core::config::auth::JwtUserClaims;
use chimes_store_core::config::{MethodHook, PluginConfig};
use chimes_store_core::pin_blockon_async_v2;
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::sdk::{
    InvokeUri, MethodDescription, RxHookInvoker, RxPluginService, RxSseRequest,
};
use chimes_store_core::service::starter::{load_config, MxStoreService};
use chimes_store_core::utils::global_data::global_app_data_insert_with_expire;
use chimes_store_core::utils::redis::{redis_get, redis_set_expire};
use chimes_store_utils::common::option_value_to_vec_value;
use futures_core::Stream;
use futures_lite::StreamExt;
use rbatis::Page;
use reqwest::Method;
use reqwest_eventsource::retry::Never;
use reqwest_eventsource::{Event, EventSource};
use salvo::oapi::{
    schema, Array, Content, Object, OpenApi, Operation, PathItem, RefOr, RequestBody, Response,
    Schema,
};
use salvo::sse::SseEvent;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use substring::Substring;

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
    pub mcp_tool: bool,
    pub mcp_schema: Option<String>,

    pub response_schema: Option<String>,

    #[serde(default)]
    pub no_access_token: bool, // 为true则表示不附加AccessToken

    pub rest_body: String,
    pub rest_content_type: Option<String>,
    pub return_validate: Option<String>,
    pub return_data: Option<String>,

    #[serde(default)]
    pub return_bytes: bool, // 为true表示，返回的是二进制数据，但接口返回会将其进行BASE64

    #[serde(default)]
    pub return_xml: bool,
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

    #[serde(default)]
    pub verify_param_sign: bool,

    #[serde(default)]
    pub encryption_body: bool,

    #[serde(default)]
    pub validate_params: bool,

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
        apiresult = apiresult.property("data", Array::new().items(t));
    } else {
        apiresult = apiresult.property("data", t);
    }
    apiresult = apiresult.property(
        "timestamp",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    schema::Schema::Object(Box::new(apiresult))
}

impl RestApiServiceInfo {
    pub(crate) fn to_operation(&self, array: bool) -> Operation {
        let mut ins_op = Operation::new();
        let param_schema = if let Some(mcpschema) = self.mcp_schema.clone() {
            // log::warn!("Restapi schema: {}", mcpschema);
            if let Ok(schema) = serde_json::from_str::<Object>(&mcpschema) {
                schema::Schema::Object(Box::new(schema))
            } else {
                schema::Schema::Object(Box::new(Object::new()))
            }
        } else {
            schema::Schema::Object(Box::new(Object::new()))
        };

        ins_op = ins_op.request_body(
            RequestBody::new().add_content("application/json", RefOr::Type(param_schema)),
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
                RefOr::Type(schema::Schema::Object(Box::new(Object::new()))),
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
    pub token_present: Option<String>,
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
            .next_back()
    }
}

#[allow(dead_code)]
pub struct RestapiPluginService {
    namespace: String,
    conf: PluginConfig,
    restconf: Mutex<Option<RestapiPluginConfig>>,
    service_map: HashMap<String, RestApiServiceInfo>,
}

unsafe impl Send for RestapiPluginService {}

unsafe impl Sync for RestapiPluginService {}

impl RestapiPluginService {
    #[allow(dead_code)]
    pub fn new(ns: &str, conf: &PluginConfig) -> Result<Self, anyhow::Error> {
        log::debug!("Plugin config load from {}", conf.config.clone());
        let t = match load_config(conf.config.clone()) {
            Ok(r) => r,
            Err(err) => {
                log::debug!("Could not load the config file: {err:?}");
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

    fn to_mcp_tool(&self) -> Vec<Value> {
        let mut tools = vec![];
        let tconf = self.restconf.lock().unwrap().clone();
        if let Some(restconf) = tconf {
            let conf_name = self.conf.name.clone();
            let ns_ = self.namespace.clone().replace(".", "_");
            for svc in restconf.services.iter() {
                if svc.mcp_tool {
                    let uri = format!("restapi://{}/{}#{}", self.namespace, &conf_name, svc.name);
                    let ret_type = "optional";
                    let func_name = format!("restapi_{}_{}_{}", ns_, &conf_name, svc.name.clone());
                    if let Some(schema_desc) = svc.mcp_schema.clone() {
                        if let Ok(param) = serde_json::from_str::<Object>(&schema_desc) {
                            let val = json!({
                                "name": func_name,
                                "invoke_uri": uri,
                                "return_type": ret_type,
                                "description": svc.rest_desc.clone().unwrap_or_default(),
                                "parameters": param
                            });
                            tools.push(val);
                        } else {
                            let val = json!({
                                "name": func_name,
                                "invoke_uri": uri,
                                "return_type": ret_type,
                                "description": svc.rest_desc.clone().unwrap_or_default(),
                                "parameters": json!({"type": "object", "properties": {}})
                            });
                            tools.push(val);
                        }
                    } else {
                        let val = json!({
                            "name": func_name,
                            "invoke_uri": uri,
                            "return_type": ret_type,
                            "description": svc.rest_desc.clone().unwrap_or_default(),
                            "parameters": json!({"type": "object", "properties": {}})
                        });
                        tools.push(val);
                    }
                }
            }
        }
        tools
    }

    fn to_schema_operation(&self) -> Operation {
        let mut ins_op = Operation::new();
        ins_op = ins_op.description("获取该方法的请求参数与返回值的JSONSchema表示。");
        let mut resp = Response::new("返回该方法的JSONSchema。");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(RefOr::Type(
                schema::Schema::Object(Box::new(Object::new())),
            ), false)),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));
        ins_op
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
                let schpath = format!(
                            "/api/restapi/{}/{}/{}/schema",
                            ns,
                            self.conf.name.clone(),
                            tsvc.name
                );

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
                openapi = openapi.add_path(schpath, PathItem::new(salvo::oapi::PathItemType::Get, self.to_schema_operation()));
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

                if let Some(mcpschema) = tsvc.mcp_schema.clone() {
                    if let Ok(schema) = serde_json::from_str::<Object>(&mcpschema) {
                        openapi =
                            openapi.add_schema("params", schema::Schema::Object(Box::new(schema)));
                    }
                }
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
        let redis_key = format!("restapi://{ns}/{name}-access-token");
        if let Ok(Some(t)) = redis_get(ns, &redis_key) {
            return Some(t);
        }

        log::info!("get access_token by oauth2 request.");

        let opt = restconf.custom_headers.clone().map(|v| {
            let t = template_eval(&v, json!({"config": restconf.clone()})).unwrap_or(v);
            serde_json::from_str::<Value>(&t)
                .map(|f| match f {
                    Value::Object(mut mt) => {
                        if restconf.accept_invalid_certs {
                            mt.insert("accept_invalid_certs".to_string(), Value::Bool(true));
                        }
                        Value::Object(mt)
                    }
                    _ => {
                        json!({"accept_invalid_certs": restconf.accept_invalid_certs })
                    }
                })
                .unwrap_or(Value::Null)
        });
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
        log::info!("method call: {req_url} == {fmt}, {data}");
        let text = template_eval(&data, json!({"config": restconf.clone()})).unwrap_or_default();
        match RestHttpClient::send_http_request(&req_url, method, &text, &fmt, &opt, false, false)
            .await
        {
            Ok((t, headers)) => {
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
                    log::warn!("jsonpath to get token -- {expire_path} / {token_path}");
                    if let Some(Value::String(tv)) = json_path_get(&t, &token_path) {
                        log::warn!("Got it: {tv}");
                        if let Err(err) = redis_set_expire(ns, &redis_key, &tv, expire - 30) {
                            log::debug!("error for set expire {err}");
                        }
                        Some(tv.to_owned())
                    } else if let Some(headermap) = headers {
                        if let Some(token_pass) = restconf.token_pass_style.clone() {
                            let token = restconf.token_identifier.clone().unwrap_or_default();
                            headermap.iter().for_each(|(h, v)| {
                                log::info!("header {h:?}, value: {v:?}");
                            });
                            if token_pass.to_lowercase() == "cookie" {
                                if let Some(hv) = headermap.get("set-cookie") {
                                    if let Ok(cookievalue) = hv.to_str() {
                                        let prefix = format!("{token}=");
                                        log::warn!("Got the cookie: {prefix} /// {cookievalue}");
                                        if cookievalue.starts_with(&prefix) {
                                            let val = if let Some(endp) = cookievalue.find(";") {
                                                cookievalue.substring(prefix.len(), endp)
                                            } else {
                                                cookievalue
                                                    .substring(prefix.len(), cookievalue.len())
                                            };

                                            log::warn!("found the value of cookie {val}");
                                            return Some(val.to_string());
                                        }
                                    }
                                }
                            } else if token_pass.to_lowercase() == "header" {
                                // get the header value
                                if let Some(hv) = headermap.get(token) {
                                    return Some(hv.to_str().unwrap_or_default().to_string());
                                }
                            }
                        }
                        None
                    } else {
                        None
                    }
                } else {
                    // 如果是采用Cookie传递，则从Cookie中查询相应的Token;
                    if let Some(headermap) = headers {
                        if let Some(token_pass) = restconf.token_pass_style.clone() {
                            let token = restconf.token_identifier.clone().unwrap_or_default();
                            headermap.iter().for_each(|(h, v)| {
                                log::info!("header {h:?}, value: {v:?}");
                            });
                            if token_pass.to_lowercase() == "cookie" {
                                if let Some(hv) = headermap.get("set-cookie") {
                                    if let Ok(cookievalue) = hv.to_str() {
                                        let prefix = format!("{token}=");
                                        log::warn!("Got the cookie: {prefix} /// {cookievalue}");
                                        if cookievalue.starts_with(&prefix) {
                                            let val = if let Some(endp) = cookievalue.find(";") {
                                                cookievalue.substring(prefix.len(), endp)
                                            } else {
                                                cookievalue
                                                    .substring(prefix.len(), cookievalue.len())
                                            };

                                            log::warn!("found the value of cookie {val}");
                                            return Some(val.to_string());
                                        }
                                    }
                                }
                            } else if token_pass.to_lowercase() == "header" {
                                // get the header value
                                if let Some(hv) = headermap.get(token) {
                                    return Some(hv.to_str().unwrap_or_default().to_string());
                                }
                            }
                        }
                        None
                    } else {
                        None
                    }
                }
            }
            Err(err) => {
                log::info!("request {req_url} with err {err}");
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
    log::info!("calling rest api {}, AK: {access_token:?}", plc.rest_url);
    let no_access_token = plc.no_access_token;

    let mut opt = restconf.custom_headers.clone().map(|v| {
        let t = template_eval(
            &v,
            json!({"config": restconf.clone(), "args": args.to_vec()}),
        )
        .unwrap_or(v);
        serde_json::from_str::<Value>(&t).unwrap_or(Value::Null)
    });
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
                    "{}={}",
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
                    let acctoken = if let Some(tp) = restconf.token_present.clone() {
                        if tp.is_empty() {
                            access_token.clone().unwrap_or_default()
                        } else {
                            template_eval(
                                &tp,
                                json!({"token": access_token.clone().unwrap_or_default()}),
                            )
                            .unwrap_or(access_token.clone().unwrap_or_default())
                        }
                    } else {
                        access_token.clone().unwrap_or_default()
                    };
                    let json = json!({restconf.token_identifier.clone().unwrap_or("access_token".to_owned()): acctoken});
                    optmap.insert("header".to_string(), json);
                }
            } else {
                let acctoken = if let Some(tp) = restconf.token_present.clone() {
                    if tp.is_empty() {
                        access_token.clone().unwrap_or_default()
                    } else {
                        template_eval(
                            &tp,
                            json!({"token": access_token.clone().unwrap_or_default()}),
                        )
                        .unwrap_or(access_token.clone().unwrap_or_default())
                    }
                } else {
                    access_token.clone().unwrap_or_default()
                };
                let json = json!({restconf.token_identifier.clone().unwrap_or("access_token".to_owned()): acctoken});
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
    log::info!("Template Args: {args:?}");

    let ctx_val = json!({ "config": restconf.clone(), "args": args });
    let text = template_eval(&data, ctx_val.clone()).unwrap_or_default();

    log::warn!("Request Headers: {opt:?}");

    let retbytes = plc.return_bytes;
    let retxml = plc.return_xml;

    match RestHttpClient::send_http_request(&req_url, method, &text, &fmt, &opt, retbytes, retxml)
        .await
    {
        Ok((t, _)) => {
            if let Some(t) = t {
                log::debug!("respose: {t}");
                if let Some(ret_validate) = plc.return_validate.clone() {
                    if !ret_validate.trim().is_empty() {
                        let ret_val = json_path_get(&t, ret_validate.trim());
                        if ret_val.is_none() {
                            return Err(anyhow!("return data was not validated"));
                        }
                    }
                }

                let ret_ctx_val =
                    json!({ "config": restconf.clone(), "args": args, "ret": t.clone() });

                let captcha_id = if plc.use_auth {
                    let res = template_eval(
                        &plc.captcha_id_express.clone().unwrap_or_default(),
                        ret_ctx_val.clone(),
                    )
                    .unwrap_or_default();
                    Some(res)
                } else {
                    None
                };

                let captcha_code = if plc.use_auth {
                    let res = template_eval(
                        &plc.captcha_code_express.clone().unwrap_or_default(),
                        ret_ctx_val,
                    )
                    .unwrap_or_default();
                    Some(res)
                } else {
                    None
                };

                if let Some(rt) = plc.return_data.clone() {
                    let ret = json_path_get(&t, rt.trim());
                    if plc.use_auth && captcha_code.is_some() && captcha_id.is_some() {
                        global_app_data_insert_with_expire(
                            &captcha_id.unwrap_or_default(),
                            &captcha_code.unwrap_or_default(),
                            6000,
                        );
                    }
                    Ok(ret)
                } else {
                    if plc.use_auth && captcha_code.is_some() && captcha_id.is_some() {
                        global_app_data_insert_with_expire(
                            &captcha_id.unwrap_or_default(),
                            &captcha_code.unwrap_or_default(),
                            6000,
                        );
                    }
                    Ok(Some(t))
                }
            } else {
                Ok(None)
            }
        }
        Err(err) => {
            log::info!("request {req_url} with err {err}");
            Err(anyhow!(err))
        }
    }
}

/**
 * 调用函数
 * 通过这种方式可以解除&self的引用，从而可以将有些引用打散
 */
pub async fn call_sse_api(
    restconf: &RestapiPluginConfig,
    plc: &RestApiServiceInfo,
    _ctx: Arc<Mutex<InvocationContext>>,
    access_token: Option<String>,
    args: &[Value],
) -> Result<EventSource, anyhow::Error> {
    log::info!("calling sse api {}", plc.rest_url);
    let no_access_token = plc.no_access_token;

    let mut opt = restconf.custom_headers.clone().map(|v| {
        let t = template_eval(
            &v,
            json!({"config": restconf.clone(), "args": args.to_vec()}),
        )
        .unwrap_or(v);
        serde_json::from_str::<Value>(&t).unwrap_or(Value::Null)
    });
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
                    let acctoken = if let Some(tp) = restconf.token_present.clone() {
                        if tp.is_empty() {
                            access_token.clone().unwrap_or_default()
                        } else {
                            template_eval(
                                &tp,
                                json!({"token": access_token.clone().unwrap_or_default()}),
                            )
                            .unwrap_or(access_token.clone().unwrap_or_default())
                        }
                    } else {
                        access_token.clone().unwrap_or_default()
                    };
                    let json = json!({restconf.token_identifier.clone().unwrap_or("access_token".to_owned()): acctoken});
                    optmap.insert("header".to_string(), json);
                }
            } else {
                let acctoken = if let Some(tp) = restconf.token_present.clone() {
                    if tp.is_empty() {
                        access_token.clone().unwrap_or_default()
                    } else {
                        template_eval(
                            &tp,
                            json!({"token": access_token.clone().unwrap_or_default()}),
                        )
                        .unwrap_or(access_token.clone().unwrap_or_default())
                    }
                } else {
                    access_token.clone().unwrap_or_default()
                };
                let json = json!({restconf.token_identifier.clone().unwrap_or("access_token".to_owned()): acctoken});
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
    log::info!("Template Args: {args:?}");

    let ctx_val = json!({ "config": restconf.clone(), "args": args });
    let text = template_eval(&data, ctx_val.clone()).unwrap_or_default();

    log::warn!("Request Headers: {opt:?}");

    // let retbytes = plc.return_bytes;
    // let retxml = plc.return_xml;

    RestHttpClient::send_sse_request(&req_url, method, &text, &fmt, &opt).await
}

impl RxPluginService for RestapiPluginService {
    
    fn should_verify_params(&self, uri: &InvokeUri) -> bool {
        if let Some(named_service) = self.get_named_service(&uri.method) {
            named_service.validate_params
        } else {
            false
        }
    }

    fn should_decrypt_body(&self, uri: &InvokeUri) -> bool {
        if let Some(named_service) = self.get_named_service(&uri.method) {
            named_service.encryption_body
        } else {
            false
        }
    }
        
    fn should_verify_sign(&self, 
        uri: &InvokeUri,
        jwt: &Option<JwtUserClaims>) -> bool {
        if let Some(named_service) = self.get_named_service(&uri.method) {
            if named_service.verify_param_sign {
                if let Some(jwtst) = jwt {
                    return jwtst.domain.contains("@");   // domain中包含有@符号代表当前访问者是使用AppId/AppSecret方式进行访问
                } else {
                    return true; // 必须校验，即一定会失败
                }
            }
        }
        false
    }

    fn invoke_direct_query(
        &self,
        config: Value,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, anyhow::Error>> + Send>> {
        // Compose
        let ns = self.namespace.clone();
        let name = self.conf.name.clone();
        let restconf = self.restconf.lock().unwrap().clone();
        // self.get_named_service(&st_uri.method);
        Box::pin(async move {
            let named_service = serde_json::from_value::<RestApiServiceInfo>(config)
                .map_err(|e| {
                    log::warn!("parse error {e}");
                    e
                })
                .map(Some)
                .unwrap_or(None);
            if let Some(restconf) = restconf {
                let access_token = Self::get_access_token(&ns, &name, &restconf).await;
                if let Some(plc) = named_service {
                    match call_rest_api(&restconf, &plc, ctx.clone(), access_token, &args).await {
                        Ok(ret) => Ok(option_value_to_vec_value(&ret)),
                        Err(err) => Err(err),
                    }
                } else {
                    Err(anyhow!(
                        "Custom config of this direct request was not be parsed"
                    ))
                }
            } else {
                Err(anyhow!("RESTful Service was not config porperly."))
            }
        })
    }

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
        let verify_result= self.verify_params(&st_uri, args.clone());

        Box::pin(async move {
            if let Err(err) = verify_result {
                return Err(err);
            }
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
        let verify_result= self.verify_params(&st_uri, args.clone());

        Box::pin(async move {
            if let Err(err) = verify_result {
                return Err(err);
            }

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

    fn to_sse_request(&self) -> Option<Box<&dyn RxSseRequest>> {
        Some(Box::new(self))
    }

    fn get_config(&self) -> Option<Value> {
        match serde_json::to_value(self.restconf.lock().unwrap().clone()) {
            Ok(t) => Some(t),
            Err(err) => {
                log::debug!("Convert to json with error: {err:?}");
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
                log::warn!("Parse JSON value to config with error: {err:?}");
                Err(anyhow!(err))
            }
        }
    }

    fn add_service(&self, services: Vec<Value>) -> Result<(), anyhow::Error> {
        services
            .iter()
            .map(|f| serde_json::from_value::<RestApiServiceInfo>(f.to_owned()).unwrap_or_default())
            .filter(|p| !p.name.is_empty())
            .for_each(|t| {
                if let Some(rtconf) = self.restconf.lock().unwrap().as_mut() {
                    match rtconf.services.iter().position(|cp| cp.name == t.name) {
                        Some(idx) => {
                            if idx < rtconf.services.len() {
                                rtconf.services.remove(idx);
                                rtconf.services.push(t);
                            }
                        }
                        None => {
                            rtconf.services.push(t);
                        }
                    }
                }
            });
        Ok(())
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

    fn get_method_metadata(
        &self,
        name: &str,
    ) -> Option<chimes_store_core::service::sdk::MethodDescription> {
        if let Some(compse) = self.restconf.lock().unwrap().clone() {
            compse
                .services
                .clone()
                .iter()
                .filter(|p| p.name == name)
                .map(|svc| MethodDescription {
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
                })
                .last()
        } else {
            None
        }
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
                        .any(|f| f.is_empty() || roles.contains(&f))
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn get_mcp_tools(&self) -> Result<Vec<Value>, anyhow::Error> {
        Ok(self.to_mcp_tool())
    }

    fn get_invoke_param_schema(&self, invk: &InvokeUri) -> Option<Value> {
        if let Some(ts) = self.get_named_service(&invk.method) {
            // ts.mcp_schema.map(|s| serde_json::from_str(&s).map(|t| Some(t)).unwrap_or(None)).unwrap_or(None)
            let resp = ts.mcp_schema.map(|s| serde_json::from_str::<Value>(&s).map(|t| Some(t)).unwrap_or(None)).unwrap_or(None);
            if let Some(val) = &resp {
                if let Some(Value::String(refval)) = val.get("$ref") {
                    if let Ok(ivkref) = InvokeUri::parse(&refval) {
                        if let Some(mx) = MxStoreService::get(&ivkref.namespace) {
                            if let Some(st) = mx.get_object(&ivkref.object) {
                                if let Ok(sch) = st.to_validate_schema(&mx.get_config(), false, false) {
                                    return serde_json::to_value(sch).map(Some).unwrap_or(None);
                                }
                            }
                        }
                    } 
                }
            }
            
            return resp;
        } else {
            None
        }
    }


    fn get_response_schema(&self, invk: &InvokeUri) -> Option<Value> {
        if let Some(ts) = self.get_named_service(&invk.method) {
            let resp = ts.response_schema.map(|s| serde_json::from_str::<Value>(&s).map(|t| Some(t)).unwrap_or(None)).unwrap_or(None);
            if let Some(val) = &resp {
                if let Some(Value::String(refval)) = val.get("$ref") {
                    if let Ok(ivkref) = InvokeUri::parse(&refval) {
                        if let Some(mx) = MxStoreService::get(&ivkref.namespace) {
                            if let Some(st) = mx.get_object(&ivkref.object) {
                                if let Ok(sch) = st.to_validate_schema(&mx.get_config(), false, false) {
                                    return serde_json::to_value(sch).map(Some).unwrap_or(None);
                                }
                            }
                        }
                    } 
                }
            }
            return resp;
        } else {
            None
        }
    }    
}

impl RxSseRequest for RestapiPluginService {
    fn invoke_sse_request(
        &self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Pin<Box<dyn Stream<Item = Result<SseEvent, salvo::Error>> + Send>>,
                        anyhow::Error,
                    >,
                > + Send,
        >,
    > {
        // Compose
        let ns = self.namespace.clone();
        let name = self.conf.name.clone();
        let full_uri = uri.url();
        let st_uri = uri.clone();
        let restconf = self.restconf.lock().unwrap().clone();
        let named_service = self.get_named_service(&st_uri.method);
        let verify_result= self.verify_params(&st_uri, args.clone());
        
        Box::pin(async move {
            if let Err(err) = verify_result {
                return Err(err);
            }

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
                    match call_sse_api(&restconf, &plc, ctx.clone(), access_token, &mix_args).await
                    {
                        Ok(mut ret) => {
                            // Ok(Box::new(ret))
                            ret.set_retry_policy(Box::new(Never));
                            let ss = ret.map(|s| {
                                let  sse = match s {
                                    Ok(evt) => {
                                        match evt {
                                            Event::Open => {
                                                Ok(SseEvent::default().name("OPEN").text("[OK]"))
                                            },
                                            Event::Message(msg) => {
                                                Ok(SseEvent::default().name(msg.event).id(msg.id).text(msg.data))
                                            }
                                        }
                                    },
                                    Err(err) => {
                                        // SseEvent::default().name("ERROR").text(format!("error {err}"))
                                        match err {
                                            reqwest_eventsource::Error::StreamEnded => {
                                                // Ok(SseEvent::default())
                                                // We need StreamEnded to finish this request
                                                Ok(SseEvent::default().name("end").text(err.to_string()))
                                            },
                                            reqwest_eventsource::Error::InvalidContentType(_, resp ) => {
                                                let response =  pin_blockon_async_v2!(async move {
                                                    match resp.text().await {
                                                        Ok(text) => {
                                                            text
                                                        },
                                                        Err(err) => {
                                                            format!("Could not extract response body {err}")
                                                        }
                                                    }
                                                });

                                                match response {
                                                    Ok(text) => {
                                                        Ok(SseEvent::default().name("end").text(text))
                                                    },
                                                    Err(err) => {
                                                        Err(salvo::Error::Other(err.into()))
                                                    }
                                                }
                                            },
                                            _ => {
                                                Err(salvo::Error::Other(err.into()))
                                            }
                                        }
                                        // Err(salvo::Error::Other(err.into()))
                                    }
                                };

                                // Ok::<_, salvo::Error>(sse)
                                sse
                            });

                            Ok(ss.boxed())
                            // Ok(Box::new(ss))
                            // Err(anyhow!("Unhandle"))
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
}
