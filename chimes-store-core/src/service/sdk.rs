use anyhow::{anyhow, Error};
use futures_lite::Future;
use rbatis::Page;
use serde_json::Value;
use std::any::Any;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use substring::Substring;

use crate::config::auth::JwtUserClaims;
use crate::config::{Column, MethodHook, PluginConfig, QueryCondition};
use crate::dbs::probe::{ColumnInfo, KeyColumnInfo, TableInfo};

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
    ) -> impl Future<Output = Result<Page<Value>, Error>>;
}

#[derive(Clone, Debug)]
pub struct InvokeUri {
    pub schema: String,
    pub namespace: String,
    pub object: String,
    pub method: String,
    pub query: Option<String>,
}

unsafe impl Send for InvokeUri {}

impl InvokeUri {
    pub fn parse(uri: &str) -> Result<Self, anyhow::Error> {
        match url::Url::parse(uri) {
            Ok(url) => {
                let path = url.path();
                Ok(Self {
                    schema: url.scheme().to_owned(),
                    namespace: url.host_str().unwrap_or_default().to_owned(),
                    object: path.substring(1, path.len()).to_owned(),
                    method: url.fragment().unwrap_or("find_one").to_owned(),
                    query: url.query().map(|f| f.to_owned()),
                })
            }
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn url_no_method(&self) -> String {
        format!("{}://{}/{}", self.schema, self.namespace, self.object)
    }

    pub fn url(&self) -> String {
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
        _namespace: String,
        _ctx: Arc<Mutex<InvocationContext>>,
        _query: String,
        _args: Vec<Value>) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>> {
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

pub trait RxPluginService: Send + Sync {
    fn get_config(&self) -> Option<Value>;
    fn parse_config(&self, val: &Value) -> Result<(), Error>;
    fn save_config(&self, conf: &PluginConfig) -> Result<(), Error>;
    fn get_metadata(&self) -> Vec<MethodDescription>;
    fn has_permission(
        &self,
        uri: &InvokeUri,
        jwt: &JwtUserClaims,
        roles: &[String],
        bypass: bool,
    ) -> bool;
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
