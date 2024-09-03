use crate::pin_submit;
use crate::service::invoker::InvocationContext;
use crate::service::script::ExtensionRegistry;
use crate::service::sdk::InvokeUri;
use crate::service::starter::MxStoreService;
use crate::utils::global_data::i64_from_str;
use anyhow::anyhow;
use auth::JwtUserClaims;
use futures_lite::Future;
use itertools::Itertools;
use rbatis::Page;
use rbatis::PageRequest;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

pub mod auth;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u64,
    pub address: String,
    pub version: Option<String>,
}

pub trait HookInvoker {
    fn invoke_return_vec(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, anyhow::Error>> + Send>>;

    //impl Future<Output = Result<Vec<Value>, anyhow::Error>> + Send;
    fn invoke_return_option(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> impl Future<Output = Result<Option<Value>, anyhow::Error>> + Send;
    fn invoke_return_page(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> impl Future<Output = Result<Page<Value>, anyhow::Error>> + Send;
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MethodHook {
    pub lang: String,   // the lang ext for parse the script
    pub script: String, // the script to be execute
    pub before: bool,   // the hooktype for before or after
    pub event: bool, // event, if the event flag is true, the call will be hold on another thread and with out return.
}

unsafe impl Send for MethodHook {}
unsafe impl Sync for MethodHook {}

impl HookInvoker for MethodHook {
    /**
     * 根据Hook的脚本来执行对应的Hook
     */
    async fn invoke_return_option(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Option<Value>, anyhow::Error> {
        let script_ = self.script.clone();
        if self.lang == *"invoke_uri" {
            if self.event {
                let taskctx = Arc::new(Mutex::new(InvocationContext::new()));
                pin_submit!(async move {
                    if let Err(err) = MxStoreService::invoke_return_one(script_, taskctx, args).await {
                        log::info!("error in invoker {err}");
                    }
                });
                Ok(None)
            } else {
                MxStoreService::invoke_return_one(script_, ctx, args).await
            }
        } else if let Some(lang) = ExtensionRegistry::get_extension(&self.lang) {
            if let Some(eval_func) = lang.fn_return_option_script {
                if self.event {
                    let taskctx = Arc::new(Mutex::new(InvocationContext::new()));
                    pin_submit!(async move { 
                        if let Err(err) = eval_func(&script_, taskctx, &args) {
                            log::info!("error on eval script {err}");
                        } 
                    });
                    Ok(None)
                } else {
                    eval_func(&script_, ctx, &args)
                }
            } else {
                Err(anyhow!(
                    "Not Found the lang extension to eval return_one invoker {}",
                    self.lang
                ))
            }
        } else {
            Err(anyhow!("Not Found the lang extension {}", self.lang))
        }
    }

    /**
     * 根据Hook的脚本来执行对应的Hook
     */
    fn invoke_return_vec(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, anyhow::Error>> + Send>> {
        let lang_ = self.lang.clone();
        let event_ = self.event;
        let script_ = self.script.clone();
        if lang_ == *"invoke_uri" {
            if event_ {
                let taskctx = Arc::new(Mutex::new(InvocationContext::new()));
                pin_submit!(async move {
                    if let Err(err) = MxStoreService::invoke_return_vec(script_, taskctx, args).await {
                        log::info!("error in invoker vec {err}");
                    }
                });
                Box::pin(async move { Ok(vec![]) })
            } else {
                log::debug!("Calling the hook in blockon.");
                Box::pin(async move {
                    match MxStoreService::invoke_return_vec(script_, ctx, args).await
                    {
                        Ok(tt) => {
                            log::info!("Ret: {:?}", tt);
                            Ok(tt)
                        }
                        Err(err) => Err(err),
                    }
                })
            }
        } else if let Some(lang) = ExtensionRegistry::get_extension(&self.lang) {
            if let Some(eval_func) = lang.fn_return_vec_script {
                if self.event {
                    let taskctx = Arc::new(Mutex::new(InvocationContext::new()));
                    pin_submit!(async move { 
                        if let Err(err) = eval_func(&script_, taskctx, &args) {
                            log::info!("error eval script {err}");
                        }
                    });
                    Box::pin(async move { Ok(vec![]) })
                } else {
                    Box::pin(async move {
                        eval_func(&script_, ctx, &args)
                    })
                }
            } else {
                Box::pin(async move {
                    Err(anyhow!(
                        "Not Found the lang extension to eval return_one invoker {}",
                        lang_
                    ))
                })
            }
        } else {
            Box::pin(async move {
                Err(anyhow!("Not Found the lang extension {}", lang_))
            })
        }
    }

    /**
     * 根据Hook的脚本来执行对应的Hook
     */
    async fn invoke_return_page(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Page<Value>, anyhow::Error> {
        let script_ = self.script.clone();
        if self.lang == *"invoke_uri" {
            if self.event {
                let taskctx = Arc::new(Mutex::new(InvocationContext::new()));
                pin_submit!(async move {
                    if let Err(err) = MxStoreService::invoke_return_page(script_, taskctx, args).await {
                        log::info!("error in invoker page {err}");
                    }
                });
                Ok(Page::new(0, 0))
            } else {
                MxStoreService::invoke_return_page(script_, ctx, args).await
            }
        } else if let Some(lang) = ExtensionRegistry::get_extension(&self.lang) {
            if let Some(eval_func) = lang.fn_return_page_script {
                if self.event {
                    let taskctx = Arc::new(Mutex::new(InvocationContext::new()));
                    pin_submit!(async move { 
                        if let Err(err) = eval_func(&script_, taskctx, &args) {
                            log::info!("error eval script {err}");
                        }
                    });
                    Ok(Page::new(0, 0))
                } else {
                    eval_func(&script_, ctx, &args)
                }
            } else {
                Err(anyhow!(
                    "Not Found the lang extension to eval return_one invoker {}",
                    self.lang
                ))
            }
        } else {
            Err(anyhow!("Not Found the lang extension {}", self.lang))
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Column {
    pub field_name: String,
    pub prop_name: Option<String>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]    
    pub col_length: Option<i64>,
    pub col_type: Option<String>,
    pub field_type: Option<String>,
    #[serde(default)]
    pub pkey: bool,

    #[serde(default)]
    pub base64: bool,

    #[serde(default)]
    pub crypto_store: bool,        // 该字段保存加密后的信息

    #[serde(default)]
    pub detail_only: bool,         // 该字段只在select/find_one中体现，对query/paged_query则不进行查出，主要用于text/blob字段处理。
    pub title: Option<String>,
    pub generator: Option<String>,
    pub validation: Option<String>, // 验证该字段数据的表达式（主要作为于Insert/Update/Upsert）
    pub desensitize: Option<String>, // 脱敏配置
    pub permitted: Option<String>,  // 赋予某角色可读写
    pub relation_object: Option<String>,
    pub relation_field: Option<String>,
    #[serde(default)]
    pub relation_array: bool,            // 对于N..N关系，以及 1..N关系，，在Query时不加载出来，只有在find_one或select时才进行加载..N的关系
    pub relation_middle: Option<String>, // 中间表的表达式，用于实现N..N关系
}

unsafe impl Send for Column {}

unsafe impl Sync for Column {}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StoreObject {
    pub name: String,
    pub object_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Column>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys_: Vec<String>,
    pub object_type: String,
    pub select_sql: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub query_hooks: Vec<MethodHook>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub select_hooks: Vec<MethodHook>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub insert_hooks: Vec<MethodHook>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub update_hooks: Vec<MethodHook>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upsert_hooks: Vec<MethodHook>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub savebatch_hooks: Vec<MethodHook>,    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delete_hooks: Vec<MethodHook>,
    #[serde(default)]
    pub validation: bool,
    #[serde(default)]
    pub parti_valid: bool,
    #[serde(default)]
    pub enable_cache: bool,
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub cache_time: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_perm_roles: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub write_perm_roles: Vec<String>,
    #[serde(default)]
    pub data_permission: bool,            // 启用数据权限
    pub permission_field: Option<String>, // 用于与数据权限建立关联的字段
    pub relative_field: Option<String>,   // 用于与数据权限建立关联的字段

    #[serde(default, skip_serializing)]
    pub field_map: Arc<RefCell<HashMap<String, Column>>>,
}

unsafe impl Send for StoreObject {}

unsafe impl Sync for StoreObject {}

impl StoreObject {
    pub fn to_field_name(&self, pn: &str) -> String {
        self.refine();
        self.field_map
            .clone()
            .borrow()
            .get(pn)
            .map(|c| c.field_name.clone())
            .unwrap_or(pn.to_owned())
    }

    fn refine(&self) {
        if self.field_map.borrow().is_empty() {
            let fmap = self
                .fields
                .clone()
                .into_iter()
                .map(|f| (f.field_name.clone(), f))
                .collect();
            self.field_map.replace(fmap);
        }
    }

    pub fn contains_field(&self, key: &str) -> bool {
        self.refine();
        self.field_map.borrow().contains_key(key)
    }

    pub fn fields_map(&self) -> HashMap<String, Column> {
        self.refine();
        self.field_map.clone().borrow().clone()
    }

    pub fn get_column(&self, field: &str, kp: bool) -> Option<Column> {
        // self.fields.clone().into_iter().filter(|c| c.field_name == *field && ( !kp || (kp && c.col_type != Some("relation".to_owned())))).last()
        self.fields
            .clone()
            .into_iter()
            .filter(|c| {
                !(c.field_name != *field || kp && c.col_type == Some("relation".to_owned()))
            })
            .last()
    }

    pub fn get_key_columns(&self) -> Vec<Column> {
        // self.fields.clone().into_iter().filter(|c| c.field_name == *field && ( !kp || (kp && c.col_type != Some("relation".to_owned())))).last()
        self.fields
            .clone()
            .into_iter()
            .filter(|k| k.pkey)
            .collect_vec()
    }

    pub fn has_permission(&self, uri: &InvokeUri, _jwt: &JwtUserClaims, roles: &[String]) -> bool {
        if self.read_perm_roles.is_empty() && self.write_perm_roles.is_empty() {
            true
        } else if uri.is_write_method() {
            //self.write_perm_roles.contains("x")
            if self.write_perm_roles.is_empty() {
                true
            } else {
                self.write_perm_roles
                    .clone()
                    .into_iter()
                    .any(|f| roles.contains(&f))
            }
        } else if self.read_perm_roles.is_empty() {
            true
        } else {
            self.read_perm_roles
                .clone()
                .into_iter()
                .any(|f| roles.contains(&f))
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginConfig {
    pub name: String,
    pub protocol: String,
    pub config: String,
    pub enable: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StoreServiceConfig {
    pub filename: String,
    #[serde(default)]
    pub db_url: String,
    pub aes_solt: Option<String>,
    pub aes_key: Option<String>,
    pub rsa_public_key: Option<String>,
    pub rsa_private_key: Option<String>,
    pub redis_url: Option<String>,

    #[serde(default)]
    pub relaxy_timezone: bool,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub max_filesize: Option<i64>,
    pub upload_filepath: Option<String>,
    #[serde(default)]
    pub subfolder_bydate: bool,
    #[serde(default)]
    pub subfolder_bytype: bool,
    pub download_prefix: Option<String>,
    #[serde(default)]
    pub download_direct: bool,
    pub download_query: Option<String>,
    pub download_file_name: Option<String>,
    pub download_file_path: Option<String>,

    #[serde(default)]
    pub upload_to_oss: bool,
    pub oss_endpoint: Option<String>,
    pub oss_bulk_id: Option<String>,
    pub oss_auth: Option<String>,

    pub namespace: String,
    pub objects: Vec<StoreObject>,
    pub querys: Vec<QueryObject>,
    pub plugins: Vec<PluginConfig>,

    #[serde(skip_serializing)]
    pub object_map: Arc<HashMap<String, StoreObject>>,

    #[serde(skip_serializing)]
    pub query_map: Arc<HashMap<String, QueryObject>>,

    #[serde(skip_serializing)]
    pub plugin_map: Arc<HashMap<String, PluginConfig>>,
}

unsafe impl Send for StoreServiceConfig {}
unsafe impl Sync for StoreServiceConfig {}

impl StoreServiceConfig {
    pub(crate) fn refine(&mut self) {
        let obj_map = self
            .objects
            .clone()
            .into_iter()
            .map(|f| (f.name.clone(), f))
            .collect();
        let qry_map = self
            .querys
            .clone()
            .into_iter()
            .map(|f| (f.name.clone(), f))
            .collect();
        let plugin_map = self
            .plugins
            .clone()
            .into_iter()
            .map(|f| (f.name.clone(), f))
            .collect();

        self.object_map = Arc::new(obj_map);
        self.query_map = Arc::new(qry_map);
        self.plugin_map = Arc::new(plugin_map);
    }

    pub fn create(
        fname: &str,
        url: &str,
        ns: &str,
        objs: Vec<StoreObject>,
        querys: Vec<QueryObject>,
        plugins: Vec<PluginConfig>,
    ) -> Self {
        let obj_map = objs
            .clone()
            .into_iter()
            .map(|f| (f.name.clone(), f))
            .collect();
        let qry_map = querys
            .clone()
            .into_iter()
            .map(|f| (f.name.clone(), f))
            .collect();
        let plugin_map = plugins
            .clone()
            .into_iter()
            .map(|f| (f.protocol.clone(), f))
            .collect();
        Self {
            filename: fname.to_owned(),
            db_url: url.to_owned(),
            aes_solt: None,
            aes_key: None,
            rsa_private_key: None,
            rsa_public_key: None,
            redis_url: None,
            namespace: ns.to_owned(),
            objects: objs,
            object_map: Arc::new(obj_map),
            querys,
            query_map: Arc::new(qry_map),
            plugins,
            plugin_map: Arc::new(plugin_map),
            ..Default::default()
        }
    }

    pub fn get_object(&self, name: &str) -> Option<StoreObject> {
        let cp = self.object_map.get(name).map(|f| f.to_owned());
        cp
    }

    pub fn get_query(&self, name: &str) -> Option<&QueryObject> {
        self.query_map.get(name)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QueryObject {
    pub name: String,
    pub object_name: String,
    #[serde(default)]
    pub pagable: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<MethodHook>,
    pub query_body: String,
    pub count_query: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<Column>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Column>,

    #[serde(default)]
    pub enable_cache: bool,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub cache_time: Option<i64>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub perm_roles: Vec<String>,

    #[serde(default)]
    pub data_permission: bool, // 启用数据权限
    pub permission_field: Option<String>, // 用于与数据权限建立关联的字段
    pub relative_field: Option<String>,   // 用于与数据权限建立关联的字段
}

impl QueryObject {
    pub fn fields_map(&self) -> HashMap<String, Column> {
        self.fields
            .iter()
            .map(|c| (c.field_name.clone(), c.clone()))
            .collect()
    }

    pub fn has_permission(&self, _uri: &InvokeUri, _jwt: &JwtUserClaims, roles: &[String]) -> bool {
        if self.perm_roles.is_empty() {
            true
        } else {
            self.perm_roles
                .clone()
                .into_iter()
                .any(|f| roles.contains(&f))
        }
    }
}

unsafe impl Send for QueryObject {}

unsafe impl Sync for QueryObject {}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OrdianlItem {
    pub field: String,
    pub sort_asc: bool,
}

unsafe impl Send for OrdianlItem {}

unsafe impl Sync for OrdianlItem {}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConditionItem {
    pub field: String,
    pub op: String,
    pub value: Value,
    pub value2: Value,
    pub and: Vec<ConditionItem>,
    pub or: Vec<ConditionItem>,
}

unsafe impl Send for ConditionItem {}

unsafe impl Sync for ConditionItem {}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IPaging {
    pub size: u64,
    pub current: u64,
}

unsafe impl Send for IPaging {}

unsafe impl Sync for IPaging {}

impl ConditionItem {
    fn compose_query(&self, args: &mut Vec<Value>) -> String {
        if self.op.trim().to_lowercase() == "between" {
            args.push(self.value.clone());
            args.push(self.value2.clone());
            format!("{} {} ? and ?", self.field.clone(), self.op.clone())
        } else if self.op.trim().to_lowercase() == "not in" {
            let (cs, mut vals) = match self.value.clone() {
                Value::Array(mps) => {
                    let st = mps.iter().map(|_| "?").join(",");
                    (st, mps.clone())
                }
                _ => ("?".to_owned(), vec![self.value.clone()]),
            };
            args.append(&mut vals);
            format!("{} {} ({})", self.field.clone(), self.op.clone(), cs)
        } else if self.op.trim().to_lowercase() == "in" {
            let (cs, mut vals) = match self.value.clone() {
                Value::Array(mps) => {
                    let st = mps.iter().map(|_| "?").join(",");
                    (st, mps.clone())
                }
                _ => ("?".to_owned(), vec![self.value.clone()]),
            };
            args.append(&mut vals);
            format!("{} {} ({})", self.field.clone(), self.op.clone(), cs)
        } else {
            args.push(self.value.clone());
            format!("{} {} ?", self.field.clone(), self.op.clone())
        }
    }

    pub(crate) fn to_query(&self, args: &mut Vec<Value>) -> anyhow::Result<String> {
        let mut cond_sql = String::new();

        let c = self.compose_query(args);
        cond_sql.push_str(&c);

        for cond in self.and.iter() {
            let q = cond.to_query(args)?;
            cond_sql.push_str(" and ");
            cond_sql.push('(');
            cond_sql.push_str(&q);
            cond_sql.push(')');
        }

        if !self.or.is_empty() {
            if !self.and.is_empty() {
                cond_sql.push_str(" or ")
            }

            for cond in self.or.iter() {
                let q = cond.to_query(args)?;
                cond_sql.push_str(" or ");
                cond_sql.push('(');
                cond_sql.push_str(&q);
                cond_sql.push(')');
            }
        }

        Ok(cond_sql)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QueryCondition {
    pub and: Vec<ConditionItem>,
    pub or: Vec<ConditionItem>,
    pub sorts: Vec<OrdianlItem>,
    pub group_by: Vec<OrdianlItem>,
    pub paging: Option<IPaging>,
}

unsafe impl Send for QueryCondition {}

unsafe impl Sync for QueryCondition {}

impl QueryCondition {
    pub fn is_empty(&self) -> bool {
        self.and.is_empty()
            && self.or.is_empty()
            && self.group_by.is_empty()
            && self.sorts.is_empty()
    }

    pub fn is_empty_condition(&self) -> bool {
        self.and.is_empty() && self.or.is_empty()
    }

    pub fn to_query(&self, onlyquery: bool) -> anyhow::Result<(String, Vec<Value>)> {
        let mut cond_sql = String::new();
        let mut args = vec![];

        for (idx, cond) in self.and.iter().enumerate() {
            if idx == 0 {
                let q = cond.to_query(&mut args)?;
                cond_sql.push('(');
                cond_sql.push_str(&q);
                cond_sql.push(')');
            } else {
                let q = cond.to_query(&mut args)?;
                cond_sql.push_str(" and ");
                cond_sql.push('(');
                cond_sql.push_str(&q);
                cond_sql.push(')');
            }
        }

        if !self.or.is_empty() {
            if !self.and.is_empty() {
                cond_sql.push_str(" or ")
            }

            for (idx, cond) in self.or.iter().enumerate() {
                if idx == 0 {
                    let q = cond.to_query(&mut args)?;
                    cond_sql.push('(');
                    cond_sql.push_str(&q);
                    cond_sql.push(')');
                } else {
                    let q = cond.to_query(&mut args)?;
                    cond_sql.push_str(" or ");
                    cond_sql.push('(');
                    cond_sql.push_str(&q);
                    cond_sql.push(')');
                }
            }
        }

        if !onlyquery {
            if !self.group_by.is_empty() {
                cond_sql.push_str(" group by ");
                cond_sql.push_str(&self.group_by.clone().into_iter().map(|f| f.field).join(","));
            }
            if !self.sorts.is_empty() {
                cond_sql.push_str(" order by ");
                cond_sql.push_str(
                    &self
                        .sorts
                        .clone()
                        .into_iter()
                        .map(|f| {
                            let ord = if f.sort_asc { "asc" } else { "desc" };
                            format!("{} {}", f.field, ord)
                        })
                        .join(","),
                );
            }
        }
        Ok((cond_sql, args))
    }

    pub fn to_page_request(&self) -> Option<PageRequest> {
        self.paging
            .clone()
            .map(|p| PageRequest::new(p.current, p.size))
    }
}
