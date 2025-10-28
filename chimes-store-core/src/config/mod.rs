use crate::docs::openapi::ToMCPTools;
use crate::pin_submit;
use crate::service::invoker::InvocationContext;
use crate::service::script::ExtensionRegistry;
use crate::service::sdk::InvokeUri;
use crate::service::starter::MxStoreService;
use crate::utils::global_data::i64_from_str;
use anyhow::anyhow;
use auth::JwtUserClaims;
use chimes_dbs_factory::get_query_field_value_present;
use chimes_store_utils::template::SchemaObjectType;
use itertools::Itertools;
use rbatis::Page;
use rbatis::PageRequest;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use substring::Substring;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
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
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, anyhow::Error>> + Send>>;
    fn invoke_return_page(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, anyhow::Error>> + Send>>;
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
    fn invoke_return_option(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, anyhow::Error>> + Send>> {
        let script_ = self.script.clone();
        let lang_ = self.lang.clone();
        let event_ = self.event;

        if lang_ == *"invoke_uri" {
            if event_ {
                let taskctx = Arc::new(Mutex::new(InvocationContext::new()));
                pin_submit!(async move {
                    if let Err(err) =
                        MxStoreService::invoke_return_one(script_, taskctx, args).await
                    {
                        log::info!("error in invoker {err}");
                    }
                });
                Box::pin(async move { Ok(None) })
            } else {
                Box::pin(async move { MxStoreService::invoke_return_one(script_, ctx, args).await })
            }
        } else if let Some(langext) = ExtensionRegistry::get_extension(&lang_) {
            if let Some(eval_func) = langext.fn_return_option_script {
                if event_ {
                    let taskctx = Arc::new(Mutex::new(InvocationContext::new()));
                    pin_submit!(async move {
                        if let Err(err) = eval_func(&script_, taskctx, &args) {
                            log::info!("error on eval script {err}");
                        }
                    });
                    Box::pin(async move { Ok(None) })
                } else {
                    Box::pin(async move { eval_func(&script_, ctx, &args) })
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
            Box::pin(async move { Err(anyhow!("Not Found the lang extension {}", lang_)) })
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
                    if let Err(err) =
                        MxStoreService::invoke_return_vec(script_, taskctx, args).await
                    {
                        log::info!("error in invoker vec {err}");
                    }
                });
                Box::pin(async move { Ok(vec![]) })
            } else {
                Box::pin(async move {
                    match MxStoreService::invoke_return_vec(script_, ctx, args).await {
                        Ok(tt) => {
                            log::info!("Ret: {tt:?}");
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
                    Box::pin(async move { eval_func(&script_, ctx, &args) })
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
            Box::pin(async move { Err(anyhow!("Not Found the lang extension {}", lang_)) })
        }
    }

    /**
     * 根据Hook的脚本来执行对应的Hook
     */
    fn invoke_return_page(
        &self,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, anyhow::Error>> + Send>> {
        let lang_ = self.lang.clone();
        let event_ = self.event;
        let script_ = self.script.clone();

        if lang_ == *"invoke_uri" {
            if event_ {
                let taskctx = Arc::new(Mutex::new(InvocationContext::new()));
                pin_submit!(async move {
                    if let Err(err) =
                        MxStoreService::invoke_return_page(script_, taskctx, args).await
                    {
                        log::info!("error in invoker page {err}");
                    }
                });

                Box::pin(async move { Ok(Page::new_total(0, 0, 0)) })
            } else {
                Box::pin(
                    async move { MxStoreService::invoke_return_page(script_, ctx, args).await },
                )
            }
        } else if let Some(lang) = ExtensionRegistry::get_extension(&lang_) {
            if let Some(eval_func) = lang.fn_return_page_script {
                if self.event {
                    let taskctx = Arc::new(Mutex::new(InvocationContext::new()));
                    pin_submit!(async move {
                        if let Err(err) = eval_func(&script_, taskctx, &args) {
                            log::info!("error eval script {err}");
                        }
                    });
                    Box::pin(async move { Ok(Page::new_total(0, 0, 0)) })
                } else {
                    Box::pin(async move { eval_func(&script_, ctx, &args) })
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
            Box::pin(async move { Err(anyhow!("Not Found the lang extension {}", lang_)) })
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
    pub required: bool,

    #[serde(default)]
    pub base64: bool,

    #[serde(default)]
    pub incidental: bool, // True表示该字段为附带字段，将不作为保存处理

    #[serde(default)]
    pub crypto_store: bool, // 该字段保存加密后的信息

    #[serde(default)]
    pub detail_only: bool, // 该字段只在select/find_one中体现，对query/paged_query则不进行查出，主要用于text/blob字段处理。
    pub title: Option<String>,
    pub generator: Option<String>,
    pub validation: Option<String>, // 验证该字段数据的表达式（主要作为于Insert/Update/Upsert）
    pub desensitize: Option<String>, // 脱敏配置
    pub conv_params: Option<String>, // 字典转换或查询转换时指定的参数
    pub permitted: Option<String>,  // 赋予某角色可读写
    pub relation_object: Option<String>,
    pub relation_field: Option<String>,
    #[serde(default)]
    pub relation_array: bool, // 对于N..N关系，以及 1..N关系，，在Query时不加载出来，只有在find_one或select时才进行加载..N的关系
    pub relation_middle: Option<String>, // 中间表的表达式，用于实现N..N关系
}

unsafe impl Send for Column {}

unsafe impl Sync for Column {}


impl Column {
    
    pub fn is_autogenerate_column(&self) -> bool {
        if let Some(g) = &self.generator {
            match g.as_str() {
                "autoincrement" | "uuid" | "snowflakeid" => true,
                _ => false
            }
        } else {
            false
        }
    }

    fn parse_validate(&self, valpatter: Option<String>) -> Result<(Option<i64>, Option<i64>, Option<i64>, Option<String>), anyhow::Error> {
        if let Some(validation) = valpatter {
            if validation.contains("min=") || validation.contains("max=") {
                if let Some((stfirst, stlast)) = validation.split_once(";") {
                    let first = stfirst.trim();
                    let last = stlast.trim();

                    let min = if first.starts_with("min=") {
                        first.substring(4, first.len()).parse::<i64>().map(Some).unwrap_or(None)
                    } else if last.starts_with("min=") {
                        last.substring(4, last.len()).parse::<i64>().map(Some).unwrap_or(None)
                    } else {
                        None
                    };

                    let max = if first.starts_with("max=") {
                        first.substring(4, first.len()).parse::<i64>().map(Some).unwrap_or(None)
                    } else if last.starts_with("max=") {
                        last.substring(4, last.len()).parse::<i64>().map(Some).unwrap_or(None)
                    } else {
                        None
                    };
                    Ok((max, min, None, None))
                } else {
                    Ok((None, None, None, None))
                }
            } else if validation.contains("min-len=") || validation.contains("pattern="){
                if let Some((stfirst, stlast)) = validation.split_once(";") {
                    let first = stfirst.trim();
                    let last = stlast.trim();                    
                    let minlen = if first.starts_with("min-len=") {
                        first.substring("min-len=".len(), first.len()).parse::<i64>().map(Some).unwrap_or(None)
                    } else if last.starts_with("min-len=") {
                        last.substring(4, last.len()).parse::<i64>().map(Some).unwrap_or(None)
                    } else {
                        None
                    };

                    let pattern = if first.starts_with("pattern=") {
                        first.substring("pattern=".len(), first.len())
                    } else if last.starts_with("pattern=") {
                        last.substring("pattern=".len(), last.len())
                    } else {
                        last
                    };
                    Ok((None, None, minlen, Some(pattern.to_owned())))
                } else {
                    Ok((None, None, None, None))
                }
            } else {
                // Only regex pattern
                Ok((None, None, None, Some(validation)))
            }
        } else {
            Ok((None, None, None, None))
        }
    }

    pub fn to_column_schema(&self, stc: &StoreServiceConfig, withpk: bool) -> SchemaObjectType {
        // 这里有一个问题，需要重新建模
        // 如果该字段所映射的是一个一对多的关系，则在这里无法进行表达了，
        // 一对一的，则应该可以
        // let prop_name = self.prop_name.clone().unwrap_or(self.field_name.clone());
        if let Some(objname) = &self.relation_object {
            if let Some(sto) = stc.get_object(&objname) {
                if let Some(sch) = sto.to_validate_schema(stc, false, withpk).map(Some).unwrap_or(None) {
                    let prop_schema = if self.relation_array {
                        SchemaObjectType {
                            title: self.title.clone(),
                            r#type: Some("array".to_string()),
                            items: Some(vec![sch]),
                            ..Default::default()
                        }
                    } else {
                        sch
                    };

                    return prop_schema;
                }
            }
        }
        let (max, min, minlen, pattern) = self.parse_validate(self.validation.clone()).unwrap_or((None, None, None, None));
        // let required = if withpk && self.pkey {
        //     true
        // } else {
        //     self.required || (!withpk && !self.is_autogenerate_column())
        // };

        // if required {
        //     // required_fields.push(prop_name.clone());
        // }

        let col_type = self.col_type
            .clone()
            .map(|f| match f.to_lowercase().as_str() {
                "String" | "varchar" | "string" | "text" | "longtext" => {
                    "string"
                },
                "Long" | "long" | "i64" | "u64" | "i128" | "bigint" => {
                    "int64"
                },
                "Integer" | "integer" | "int" | "i32" | "u32" => {
                    "integer"
                },
                "Float" | "Double" | "f32" | "f64" => {
                    "number"
                },
                "Boolean" | "Bool" | "bool" | "boolean" => {
                    "boolean"
                },
                "date" | "datetime" | "time" | "timestamp" => {
                    "string"
                },
                _ => "object",
            })
            .unwrap_or("object");
        
        let col_fmt = self.col_type
            .clone()
            .map(|f| match f.to_lowercase().as_str() {
                "long" | "i64" | "u64" | "i128" | "bigint" => {
                    Some("int64".to_string())
                }
                "int" | "i32" | "integer" | "u32" => {
                    Some("int32".to_string())
                }
                "smallint" => Some("int16".to_string()),
                "tinyint" => Some("int8".to_string()),
                "bool" | "boolean" => {
                    None
                }
                "date" => Some("date".to_string()),
                "datetime" => Some("date-time".to_string()),
                "decimal" | "numeric" => {
                    Some("decimal".to_string())
                }
                "float" | "f32" => {
                    Some("float".to_string())
                }
                "double" | "f64" => {
                    Some("double".to_string())
                }
                "relation" => {
                    None
                }
                _ => None,
            })
            .unwrap_or(None);

        SchemaObjectType {
            title: self.title.clone(),
            r#type: Some(col_type.to_string()),
            format: if pattern.is_none() { // 如果已经指定了Regex模式，则col_fmt须置为None
                col_fmt
            } else {
                None
            },
            max_length: self.col_length,
            maximum: max,
            minimum: min,
            min_length: minlen,
            pattern: pattern,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StoreObject {
    pub name: String,
    pub schema_name: Option<String>,
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

    pub rest_desc: Option<String>,

    #[serde(default)]
    pub mcp_tool_query: bool,
    #[serde(default)]
    pub mcp_tool_update: bool,
    #[serde(default)]
    pub mcp_tool_delete: bool,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub cache_time: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_perm_roles: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub write_perm_roles: Vec<String>,
    #[serde(default)]
    pub data_permission: bool, // 启用数据权限
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
            .next_back()
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
                    .any(|f| f.is_empty() || roles.contains(&f))
            }
        } else if self.read_perm_roles.is_empty() {
            true
        } else {
            self.read_perm_roles
                .clone()
                .into_iter()
                .any(|f| f.is_empty() || roles.contains(&f))
        }
    }

    pub fn get_mcp_tools(&self, ns: &str) -> Result<Vec<Value>, anyhow::Error> {
        self.to_mcp_tools(ns)
    }

    pub fn validate_value_by_schema(schema: &Value, objval: &Value) -> Result<(), anyhow::Error>  {
        log::debug!("JSONSchema: {}", serde_json::to_string(schema).unwrap_or_default());
        let mut errors = vec![];
        for err in jsonschema::options()
                    .should_validate_formats(true)
                    .should_ignore_unknown_formats(true)
                    .build(schema)
                    .map_err(|m| anyhow!("error for create validate: {m}"))?
                    .iter_errors(&objval)
        {
            errors.push(format!("{err:?}"));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(errors.join("\n")))
        }
    }

    /**
     * for insert, withpk should be false if the pk is auto-increment (serial for pgsql).
     * for update, withpk should be true
     * for delete, withpk will be ignored
     */
    pub fn to_validate_schema(&self, stc: &StoreServiceConfig, onlypk: bool, withpk: bool) -> Result<SchemaObjectType, anyhow::Error> {
        let mut required_fields = vec![];

        let map = self.fields.iter().filter(|p| {
                if onlypk {
                    return p.pkey;
                } else {
                    return true;
                }
            }).map(|c| {
                // 这里有一个问题，需要重新建模
                // 如果该字段所映射的是一个一对多的关系，则在这里无法进行表达了，
                // 一对一的，则应该可以
                let prop_name = c.prop_name.clone().unwrap_or(c.field_name.clone());
                let required = if withpk == false && c.is_autogenerate_column() {
                    false
                } else if withpk && c.pkey {
                    true
                } else {
                    c.required
                };

                if required {
                    required_fields.push(prop_name.clone());
                }

                (prop_name, c.to_column_schema(stc, withpk))
            }).collect::<HashMap<String, SchemaObjectType>>();

        let topobj = SchemaObjectType {
            title: self.rest_desc.clone(),
            r#type: Some("object".to_string()),
            properties: map,
            required: required_fields,
            ..Default::default()
        };

        Ok(topobj)
    }
}

impl QueryColumn for StoreObject {
    fn get_column(&self, field_name: &str) -> Option<Column> {
        self.fields
            .clone()
            .into_iter()
            .filter(|c| c.field_name == *field_name)
            .next_back()
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

unsafe impl Send for PluginConfig {}
unsafe impl Sync for PluginConfig {}

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
    pub api_audit: bool,

    pub datetime_format: Option<String>,

    pub dict_uri: Option<String>,
    pub range_uri: Option<String>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub max_redis_pool: Option<i64>,

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
        self.object_map.get(name).map(|f| f.to_owned())
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

    #[serde(default)]
    pub updatable: bool,

    #[serde(default)]
    pub onlyone: bool,

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
    pub validate_params: bool,    

    pub rest_desc: Option<String>,

    #[serde(default)]
    pub mcp_tool: bool,

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
                .any(|f| f.is_empty() || roles.contains(&f))
        }
    }

    pub fn get_mcp_tools(&self, ns: &str) -> Result<Vec<Value>, anyhow::Error> {
        self.to_mcp_tools(ns)
    }

    pub fn to_params_schema(&self, stc: &StoreServiceConfig) -> Result<SchemaObjectType, anyhow::Error> {
        let map = self.params.iter().map(|c| {
                // 这里有一个问题，需要重新建模
                // 如果该字段所映射的是一个一对多的关系，则在这里无法进行表达了，
                // 一对一的，则应该可以
                let prop_name = c.prop_name.clone().unwrap_or(c.field_name.clone());
                (prop_name, c.to_column_schema(stc, true))
            }).collect::<HashMap<String, SchemaObjectType>>();

        Ok(SchemaObjectType {
            r#type: Some("object".to_string()),
            title: self.rest_desc.clone(),
            properties: map,
            ..Default::default()
        })
    }

    pub fn to_result_schema(&self, stc: &StoreServiceConfig) -> Result<SchemaObjectType, anyhow::Error> {
        let map = self.fields.iter().map(|c| {
                // 这里有一个问题，需要重新建模
                // 如果该字段所映射的是一个一对多的关系，则在这里无法进行表达了，
                // 一对一的，则应该可以
                let prop_name = c.prop_name.clone().unwrap_or(c.field_name.clone());
                (prop_name, c.to_column_schema(stc, true))
            }).collect::<HashMap<String, SchemaObjectType>>();

        Ok(SchemaObjectType {
            r#type: Some("object".to_string()),
            title: self.rest_desc.clone(),
            properties: map,
            ..Default::default()
        })
    }
}

impl QueryColumn for QueryObject {
    fn get_column(&self, field_name: &str) -> Option<Column> {
        self.fields
            .iter()
            .filter(|p| p.field_name == *field_name)
            .map(|c| c.to_owned())
            .next_back()
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
    fn compose_query(
        &self,
        cols: &dyn QueryColumn,
        driver_type: &str,
        args: &mut Vec<Value>,
    ) -> String {
        let col_type = match cols.get_column(&self.field) {
            Some(col) => col.field_type.unwrap_or_default(),
            None => String::new(),
        };

        let queryplace = get_query_field_value_present(driver_type, &col_type);

        if self.op.trim().to_lowercase() == "between" {
            args.push(self.value.clone());
            args.push(self.value2.clone());

            format!(
                "{} {} {queryplace} and {queryplace}",
                self.field.clone(),
                self.op.clone()
            )
        } else if self.op.trim().to_lowercase() == "not in" {
            let (cs, mut vals) = match self.value.clone() {
                Value::Array(mps) => {
                    let st = mps.iter().map(|_| queryplace.clone()).join(",");
                    (st, mps.clone())
                }
                _ => (queryplace.clone(), vec![self.value.clone()]),
            };
            args.append(&mut vals);
            format!("{} {} ({})", self.field.clone(), self.op.clone(), cs)
        } else if self.op.trim().to_lowercase() == "in" {
            let (cs, mut vals) = match self.value.clone() {
                Value::Array(mps) => {
                    let st = mps.iter().map(|_| queryplace.clone()).join(",");
                    (st, mps.clone())
                }
                _ => (queryplace.clone(), vec![self.value.clone()]),
            };
            args.append(&mut vals);
            format!("{} {} ({})", self.field.clone(), self.op.clone(), cs)
        } else if self.op.trim().to_lowercase() == "is null"
            || self.op.trim().to_lowercase() == "is not null"
        {
            // args.push(self.value.clone());
            format!("{} {}", self.field.clone(), self.op.clone())
        } else {
            args.push(self.value.clone());
            format!("{} {} {queryplace}", self.field.clone(), self.op.clone())
        }
    }

    pub(crate) fn to_query(
        &self,
        cols: &dyn QueryColumn,
        driver_type: &str,
        args: &mut Vec<Value>,
    ) -> anyhow::Result<String> {
        let mut cond_sql = String::new();

        let c = self.compose_query(cols, driver_type, args);
        cond_sql.push_str(&c);

        for cond in self.and.iter() {
            let q = cond.to_query(cols, driver_type, args)?;
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
                let q = cond.to_query(cols, driver_type, args)?;
                cond_sql.push_str(" or ");
                cond_sql.push('(');
                cond_sql.push_str(&q);
                cond_sql.push(')');
            }
        }

        Ok(cond_sql)
    }
}

pub trait QueryColumn {
    fn get_column(&self, field_name: &str) -> Option<Column>;
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QueryCondition {
    pub and: Vec<ConditionItem>,
    pub or: Vec<ConditionItem>,
    pub sorts: Vec<OrdianlItem>,
    pub group_by: Vec<OrdianlItem>,
    pub paging: Option<IPaging>,
    pub dont_update_exist: Option<bool>,
    pub retrieved_detail: Option<bool>, // Get the detail for detail-field on query and paged_query
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

    pub fn to_query(
        &self,
        cols: &dyn QueryColumn,
        driver_type: &str,
        onlyquery: bool,
    ) -> anyhow::Result<(String, Vec<Value>)> {
        let mut cond_sql = String::new();
        let mut args = vec![];

        for (idx, cond) in self.and.iter().enumerate() {
            if idx == 0 {
                let q = cond.to_query(cols, driver_type, &mut args)?;
                cond_sql.push('(');
                cond_sql.push_str(&q);
                cond_sql.push(')');
            } else {
                let q = cond.to_query(cols, driver_type, &mut args)?;
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
                    let q = cond.to_query(cols, driver_type, &mut args)?;
                    cond_sql.push('(');
                    cond_sql.push_str(&q);
                    cond_sql.push(')');
                } else {
                    let q = cond.to_query(cols, driver_type, &mut args)?;
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
