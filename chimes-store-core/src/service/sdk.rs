use anyhow::{anyhow, Error};
use chimes_store_utils::common::parse_xml;
use chimes_store_utils::crypto::{hex_encode_string, hmac_sha256};
use chimes_store_utils::template::{canonicalized_value, value_to_string, SchemaObjectType};
use futures::Stream;
use futures_lite::Future;
use rbatis::Page;
use salvo::sse::SseEvent;
use salvo::Depot;
use serde_json::{Map, Value};
use std::any::Any;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use substring::Substring;

use crate::config::auth::JwtUserClaims;
use crate::config::{Column, MethodHook, PluginConfig, QueryCondition};
use crate::dbs::probe::{ColumnInfo, KeyColumnInfo, TableInfo};
use crate::service::starter::MxStoreService;
use crate::utils::global_data::copy_value_excluded;

use super::invoker::InvocationContext;

pub trait RxStoreService {
    fn select(
        &self,
        name: &str,
        val: &Value,
    ) -> impl Future<Output = Result<Option<Value>, Error>> + Send;
    fn find_one(
        &self,
        name: &str,
        cond: &QueryCondition,
    ) -> impl Future<Output = Result<Option<Value>, Error>> + Send;
    fn query(
        &self,
        name: &str,
        cond: &QueryCondition,
    ) -> impl Future<Output = Result<Vec<Value>, Error>> + Send;
    fn paged_query(
        &self,
        name: &str,
        cond: &QueryCondition,
    ) -> impl Future<Output = Result<Page<Value>, Error>> + Send;
}

pub trait RxHookInvoker {
    fn invoke_pre_hook_(
        uri: String,
        hooks: Vec<MethodHook>,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>>;

    //impl Future<Output = Result<Vec<Value>, Error>> + Send;
    fn invoke_post_hook_(
        uri: String,
        hooks: Vec<MethodHook>,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> impl Future<Output = Result<Vec<Value>, Error>> + Send;
}

pub trait MxProbeService {
    fn probe_schema(
        &self,
        schema: &str,
    ) -> impl Future<Output = Result<Vec<TableInfo>, Error>> + Send;
    fn probe_one_table(
        &self,
        schema: &str,
        tbl: &str,
    ) -> impl Future<Output = Result<Option<crate::dbs::probe::TableInfo>, anyhow::Error>> + Send;
    fn probe_table(
        &self,
        schema: &str,
        tbl: &str,
    ) -> impl Future<Output = Result<Vec<ColumnInfo>, Error>> + Send;
    fn probe_table_keys(
        &self,
        schema: &str,
        tbl: &str,
    ) -> impl Future<Output = Result<Vec<KeyColumnInfo>, Error>> + Send;
}

pub trait RxQueryService {
    fn query(
        &self,
        name: &str,
        fix_param: &Value,
        cond: &QueryCondition,
    ) -> impl Future<Output = Result<Vec<Value>, Error>> + Send;
    fn paged_query(
        &self,
        name: &str,
        fix_param: &Value,
        cond: &QueryCondition,
    ) -> impl Future<Output = Result<Page<Value>, Error>> + Send;
}

#[derive(Clone, Debug)]
pub struct InvokeUri {
    pub schema: String,
    pub namespace: String,
    pub object: String,
    pub method: String,
    pub query: Option<String>,
    pub query_pairs: Option<Map<String, Value>>,
}

unsafe impl Send for InvokeUri {}

impl InvokeUri {
    pub fn parse(uri: &str) -> Result<Self, anyhow::Error> {
        match url::Url::parse(uri) {
            Ok(url) => {
                let pairs = url
                    .query_pairs()
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
                    .collect::<Map<String, Value>>();

                let opt_pairs = if pairs.is_empty() { None } else { Some(pairs) };
                let path = url.path();
                let frg = url.fragment().unwrap_or_default();
                let (fragement, query_at) = match frg.find("?") {
                    Some(sz) => (frg.substring(0, sz), Some(frg.substring(sz + 1, frg.len()))),
                    None => (frg, None),
                };

                let (refine_pairs, refine_query) = if let Some(qt) = query_at {
                    if opt_pairs.is_none() {
                        let temp_uri = format!("http://ts.cn/name?{qt}");
                        match url::Url::parse(&temp_uri) {
                            Ok(sturl) => {
                                let new_pairs = sturl
                                    .query_pairs()
                                    .into_iter()
                                    .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
                                    .collect::<Map<String, Value>>();
                                if new_pairs.is_empty() {
                                    (None, sturl.query().map(|s| s.to_owned()))
                                } else {
                                    (Some(new_pairs), sturl.query().map(|s| s.to_owned()))
                                }
                            }
                            Err(_) => (None, url.query().map(|c| c.to_string())),
                        }
                    } else {
                        (opt_pairs, url.query().map(|c| c.to_string()))
                    }
                } else {
                    (opt_pairs, url.query().map(|c| c.to_string()))
                };

                Ok(Self {
                    schema: url.scheme().to_owned(),
                    namespace: url.host_str().unwrap_or_default().to_owned(),
                    object: path.substring(1, path.len()).to_owned(),
                    method: fragement.to_string(),
                    query: refine_query,
                    query_pairs: refine_pairs,
                })
            }
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn url_no_method(&self) -> String {
        format!("{}://{}/{}", self.schema, self.namespace, self.object)
    }

    pub fn has_query_params(&self) -> bool {
        self.query_pairs.is_some()
    }

    pub fn get_query_params(&self) -> Option<Map<String, Value>> {
        self.query_pairs.clone()
    }

    pub fn url(&self) -> String {
        if self.schema == "http" || self.schema == "https" {
            format!("{}://{}/{}", self.schema, self.namespace, self.object)
        } else {
            match &self.query {
                Some(t) => {
                    format!(
                        "{}://{}/{}?{}#{}",
                        self.schema, self.namespace, self.object, t, self.method
                    )
                }
                None => {
                    format!(
                        "{}://{}/{}#{}",
                        self.schema, self.namespace, self.object, self.method
                    )
                }
            }
        }
    }

    pub fn is_write_method(&self) -> bool {
        matches!(
            self.method.as_str(),
            "insert" | "update" | "upsert" | "delete" | "delete_by" | "update_by"
        )
    }
}

pub trait Invocation {
    fn invoke_direct_query(
        &'static self,
        _uri: &InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        _query: String,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>> {
        Box::pin(async move { Ok(vec![]) })
    }

    fn invoke_direct_query_v2(
        &'static self,
        _uri: &InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        _query: Value,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>> {
        Box::pin(async move { Ok(vec![]) })
    }

    fn invoke_return_option(
        &'static self,
        uri: &InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, Error>> + Send>>;
    fn invoke_return_vec(
        &'static self,
        uri: &InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>>;
    fn invoke_return_page(
        &'static self,
        uri: &InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, Error>> + Send>>;
}

pub type BoxFutureSseEvent = dyn Future<
        Output = Result<
            Pin<Box<dyn Stream<Item = Result<SseEvent, salvo::Error>> + Send>>,
            anyhow::Error,
        >,
    > + Send;

pub trait RxSseRequest: Send + Sync {
    fn invoke_sse_request(
        &self,
        _uri: InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        _args: Vec<Value>,
    ) -> Pin<Box<BoxFutureSseEvent>> {
        Box::pin(async { Err(anyhow!("Unimplemented")) })
    }
}

pub trait RxPluginService: Send + Sync + Any {

    fn get_api_secret(&self, depot: &mut Depot) -> Option<String> {
        depot.get::<String>("GROWTH_API_SECRET").map(|t| Some(t.to_owned())).unwrap_or(None)
    }

    fn shutdown_plugin(&self) -> Result<(), Error> {
        Ok(())
    }

    fn get_invoke_param_schema(&self, _invk: &InvokeUri) -> Option<Value> {
        None
    }

    fn get_response_schema(&self, _invk: &InvokeUri) -> Option<Value> {
        None
    }

    /**
     * 获得该插件下的所有开放出来的MCP工具
     */
    fn get_mcp_tools(&self) -> Result<Vec<Value>, Error> {
        Ok(vec![])
    }

    fn get_config(&self) -> Option<Value>;

    fn get_description(&self) -> Option<String> {
        match self.get_config() {
            Some(Value::Object(mp)) => {
                let st = mp.get("desc").unwrap_or(
                    mp.get("rest_desc").unwrap_or(
                        mp.get("description")
                            .unwrap_or(mp.get("instructions").unwrap_or(&Value::Null)),
                    ),
                );
                Some(value_to_string(st.to_owned()))
            }
            _ => None,
        }
    }

    fn parse_config(&self, val: &Value) -> Result<(), Error>;
    fn save_config(&self, conf: &PluginConfig) -> Result<(), Error>;

    fn add_service(&self, _services: Vec<Value>) -> Result<(), Error> {
        Ok(())
    }

    fn get_metadata(&self) -> Vec<MethodDescription>;

    fn get_method_metadata(&self, _name: &str) -> Option<MethodDescription> {
        None
    }

    fn has_permission(
        &self,
        _uri: &InvokeUri,
        _jwt: &JwtUserClaims,
        roles: &[String],
        bypass: bool,
    ) -> bool {
        bypass
            || roles.contains(&"ROLE_COMMONUSER".to_owned())
            || roles.contains(&"ROLE_MINIAPP".to_owned())
            || roles.contains(&"ROLE_SUPERADMIN".to_owned())
    }

    /**
     * 根据各自的Plugin的特点，来确定是否需要对请求体解密
     */
    fn should_decrypt_body(&self, _uri: &InvokeUri) -> bool {
        false
    }

    fn decrypt_body_args(&self, uri: &InvokeUri, args: &[Value]) -> Result<Vec<Value>, anyhow::Error> {
        if self.should_decrypt_body(uri) {
            if let Some(mx) = MxStoreService::get(&uri.namespace) {
                Ok(args.iter().map(|argt| {
                    match argt {
                        Value::Object(mp) => {
                            if let Some(Value::String(bodyval)) = mp.get("encbody") {
                                let algorithm = mp.get("algorithm").map(|s| s.as_str().map(|t| t.to_string()).unwrap_or("aes".to_string())).unwrap_or("aes".to_string());
                                let fmt = mp.get("format").map(|s| s.as_str().map(|t| t.to_string()).unwrap_or("json".to_string())).unwrap_or("json".to_string());
                                let decrypt_text = match algorithm.as_str() {
                                    "rsa" => {
                                        mx.rsa_decrypt_text(bodyval)
                                    },
                                    "aes" => {
                                        mx.aes_decode_text(bodyval)
                                    }
                                    _ => {
                                        bodyval.to_owned()
                                    }
                                };
                                match fmt.as_str() {
                                    "json" => {
                                        serde_json::from_str::<Value>(&decrypt_text).unwrap_or(Value::Null)
                                    },
                                    "xml" => {
                                        parse_xml(&decrypt_text).unwrap_or(Value::Null)
                                    },
                                    _ => {
                                        Value::String(decrypt_text)
                                    }
                                }
                            } else {
                                argt.to_owned()
                            }
                        },
                        Value::String(bodytext) => {
                            let decrypt_text = if mx.0.rsa_private_key.is_some() && mx.0.rsa_public_key.is_some() {
                                mx.rsa_decrypt_text(bodytext)
                            } else if mx.0.aes_key.is_some() {
                                mx.aes_decode_text(bodytext)
                            } else {
                                bodytext.to_owned()
                            };

                            serde_json::from_str::<Value>(&decrypt_text).unwrap_or(Value::Null)
                        },
                        _ => {
                            log::warn!("Value is other");
                            argt.to_owned()
                        }
                    }
                }).collect())
            } else {
                Ok(args.to_vec())
            }
        } else {
            Ok(args.to_vec())
        }
    }

    /**
     * 是否需要校验参数签名
     * 参数签名采用
     */
    fn should_verify_sign(&self, 
        _uri: &InvokeUri,
        _jwt: &Option<JwtUserClaims>) -> bool {
        false
    }

    fn should_verify_params(&self, _uri: &InvokeUri) -> bool {
        false
    }

    fn verify_params(&self, uri: &InvokeUri, args: Vec<Value>) -> Result<(), anyhow::Error> {
        if !self.should_verify_params(uri) {
            return Ok(());
        }
        
        if let Some(sch)  = self.get_invoke_param_schema(uri) {
            let tsp= serde_json::from_value::<SchemaObjectType>(sch.clone()).unwrap_or(SchemaObjectType::new());
            if !tsp.properties().is_empty() {
                for arg in args {
                    if arg.is_object() {
                        let newarg = copy_value_excluded(&arg, &["cond".to_string(), "_cond".to_string()]);
                        let should_verify = match &newarg {
                            Value::Object(mp) => {
                                !mp.is_empty()
                            },
                            _ => false
                        };
                        if should_verify {
                            let va = jsonschema::Validator::new(&sch)?;
                            if let Err(err) = va.validate(&newarg) {
                                return Err(anyhow!("Validation error: {err}"));
                            }
                        }
                    }
                }
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    /**
     * 参数签名的验证只针对Plugin有效
     * Object/Query的基本操作，无须验证，如果针对某个接口，需要进行验签，则请包装成compose接口或其它
     * 参数验证的逻辑是基于JSON Object的Key来进行的，我们需要对参数的Key进行排序，从小到大排序，并将这些参数转换成为使用&符号连接
     * 如：
     * key1=aaaa&key2=bbbbb&key3=ccccc
     * 签名时，使用AppId/AppSecret配对中的AppSecret来对上述的参数文本进行HmacSHA256签名。但是，如果接口请求过程中，如果是使用了
     * 普通的用户登录login(即通过用户名/密码方式或SSO方式来获取的)，则无须进行签名验证。只有使用AppId/AppSecret方式（包括动态获取/
     * 以及永久Token的）才需要根据配置（should_verify_sign返回True）来进行验签
     */
    fn verify_args_sign(&self, uri: &InvokeUri, jwt: &Option<JwtUserClaims>, key: Option<String>, args: Vec<Value>) -> Result<bool, anyhow::Error> {
        if self.should_verify_sign(uri, jwt) {
            let param_sch = self.get_invoke_param_schema(uri).map(|s| serde_json::from_value::<SchemaObjectType>(s).map(Some).unwrap_or(None)).flatten();

            if let Some(signkey) = key {
                if signkey.is_empty() {
                    return Err(anyhow!("No appsecret provided or appsecret is empty."));
                }
                
                for arg in args {
                    if let Some(Value::String(signval)) = arg.get("sign") {
                        if let Value::String(text) =  canonicalized_value(&arg, &param_sch, false)? {
                            let calcsign = hmac_sha256(&signkey, &text);
                            let hexsign = hex_encode_string(&calcsign);
                            // args获得sign字段，
                            if *signval == hexsign {
                                return Ok(true)
                            }
                        }
                    }
                }
            } else {
                return Err(anyhow!("No appsecret provided."));
            }
            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn get_openapi(&self, ns: &str) -> Box<dyn Any>;

    fn invoke_return_option(
        &self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, Error>> + Send>>;
    fn invoke_return_vec(
        &self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>>;
    fn invoke_return_page(
        &self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, Error>> + Send>>;

    fn invoke_direct_query(
        &self,
        _config: Value,
        _ctx: Arc<Mutex<InvocationContext>>,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }

    fn to_sse_request(&self) -> Option<Box<&dyn RxSseRequest>> {
        None
    }
}

pub struct MethodDescription {
    pub uri: String,            // object://com.siline/User
    pub name: String, // select/insert/upsert/update/delete/delete_by/update_by/query/paged_query/search/paged_search
    pub func: Option<String>, //
    pub params_vec: bool, // false, (insert_batch/update_batch/delete_batch true)
    pub params1: Vec<Column>, // insert/upsert/update etc
    pub params2: Option<Value>, // QueryCondition
    pub response: Vec<Column>, // response columns
    pub return_page: bool, // return is Page<Value>
    pub return_vec: bool, // return is Vec<Value> other else is Option<Value>
}

#[cfg(test)]
mod test {
    use crate::service::sdk::InvokeUri;

    #[test]
    pub fn test_invoke_uri_parse() {
        match InvokeUri::parse("object://com.siline/Name#find_one?id=1") {
            Ok(t) => {
                println!("Schema: {}", t.schema);
                println!("Namespace: {}", t.namespace);
                println!("Object: {}", t.object);
                println!("Method: {}", t.method);
                println!("Query: {:?}", t.query);
                println!("QueryPairs: {:?}", t.query_pairs);
            }
            Err(err) => {
                println!("error {err:?}");
            }
        }
    }
}
