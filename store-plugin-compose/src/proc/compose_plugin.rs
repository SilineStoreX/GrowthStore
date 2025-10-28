use anyhow::anyhow;
use chimes_store_core::config::auth::JwtUserClaims;
use chimes_store_core::config::{MethodHook, PluginConfig};
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::queue::SyncTaskQueue;
use chimes_store_core::service::script::ExtensionRegistry;
use chimes_store_core::service::sdk::{
    InvokeUri, MethodDescription, RxHookInvoker, RxPluginService,
};
use chimes_store_core::service::starter::{load_config, MxStoreService};
use chimes_store_core::utils::global_data::i64_from_str;
use chimes_store_core::utils::{get_multiple_rbatis_async, GlobalConfig};
use rbatis::executor::Executor;
use rbatis::Page;
use salvo::oapi::{
    schema, Array, Content, Object, OpenApi, Operation, PathItem, RefOr, RequestBody, Response,
    Schema,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::{self, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::template::json_path_get;

pub const ACQUIRE_TIMEOUT: u64 = 30;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ComposeServiceInfo {
    pub name: String,
    pub desc: Option<String>,
    pub lang: String,

    #[serde(default)]
    pub rest_api: bool,
    #[serde(default)]
    pub fileupload: bool,
    pub file_field: Option<String>,

    #[serde(default)]
    pub schedule_on: bool,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub interval_second: Option<i64>,

    pub cron_express: Option<String>,
    pub schedule_simulate: Option<String>,
    pub script: String,
    pub return_type: String,

    #[serde(default)]
    pub enable_synctask: bool,

    #[serde(default)]
    pub avoid_reentry: bool, // 防止重入，即在调用前需要对其进行加锁，这个会

    #[serde(default)]
    pub mcp_tool: bool,
    pub mcp_schema: Option<String>,
    pub rest_desc: Option<String>,
    pub response_schema: Option<String>,

    pub task_id: Option<String>,

    pub prepare_connection: Option<String>,

    #[serde(default)]
    pub execute_delete: bool, // 执行删除动作，如果为true，所有接收到的都都是删除操作
    pub check_delete: Option<String>, // 检查删除标识，JSONPath表示，检查成功，表示该记录应该执行删除操作

    #[serde(default)]
    pub perm_roles: Vec<String>,

    #[serde(default)]
    pub bypass_permission: bool, // 允许匿名访问，只有在允许匿名访问的时候，才能通过passoff调用

    #[serde(default)]
    pub verify_param_sign: bool, // 是否启用接口验签

    #[serde(default)]
    pub encryption_body: bool,   // 是否对请求的Body进行加解密

    #[serde(default)]
    pub validate_params: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<MethodHook>,
}

unsafe impl Sync for ComposeServiceInfo {}

unsafe impl Send for ComposeServiceInfo {}

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

fn to_api_result_page_schema(t: RefOr<Schema>) -> schema::Schema {
    let mut apiresult = Object::new();
    apiresult = apiresult.property(
        "total",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "page_no",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "page_size",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "do_count",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property("records", Array::new().items(t));
    to_api_result_schema(
        RefOr::Type(schema::Schema::Object(Box::new(apiresult))),
        false,
    )
}

fn to_upload_file_schema() -> schema::Schema {
    let mut apiresult = Object::new();
    apiresult = apiresult.property(
        "file_id",
        Object::new()
            .schema_type(schema::BasicType::String)
            .description("文件ID"),
    );
    apiresult = apiresult.property(
        "source",
        Object::new()
            .schema_type(schema::BasicType::String)
            .description("源文件名，它是文件上传时的文件名"),
    );
    apiresult = apiresult.property(
        "dest_file",
        Object::new()
            .schema_type(schema::BasicType::String)
            .description("目标文件名，它是后续由StoreX管理存放的文件名，只包含逻辑存放路径"),
    );
    apiresult = apiresult.property(
        "dest_path",
        Object::new()
            .schema_type(schema::BasicType::String)
            .description("目标文件名，它是后续由StoreX管理存放的文件名，包含实际存放路径"),
    );
    apiresult = apiresult.property(
        "file_type",
        Object::new()
            .schema_type(schema::BasicType::String)
            .description("文件类型，即文件的后缀名"),
    );
    apiresult = apiresult.property(
        "content_type",
        Object::new()
            .schema_type(schema::BasicType::String)
            .description("文件的content-type，自动识别"),
    );
    apiresult = apiresult.property(
        "file_size",
        Object::new()
            .schema_type(schema::BasicType::Integer)
            .description("文件的大小"),
    );
    apiresult = apiresult.property(
        "access_url",
        Object::new()
            .schema_type(schema::BasicType::String)
            .description("件进行下载访问的URL，由StoreX管理产生"),
    );
    apiresult = apiresult.property(
        "copied",
        Object::new()
            .schema_type(schema::BasicType::Boolean)
            .description("文件是否被复制到目标位置"),
    );
    apiresult = apiresult.property(
        "data",
        Object::new()
            .schema_type(schema::BasicType::Object)
            .description("自定义数据，由前端传入，通常为对应的业务数据"),
    );
    schema::Schema::Object(Box::new(apiresult))
}

impl ComposeServiceInfo {
    
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

    pub(crate) fn to_operation(&self) -> Operation {
        let mut ins_op = Operation::new();

        let param_schema = if let Some(mcpschema) = self.mcp_schema.clone() {
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

        ins_op = ins_op.summary(self.desc.clone().unwrap_or_default());

        let mut description = "方法在执行的时候会根据URL中所传递的Query参数，组装成第一个参数（根据参数名转成JSON Object），从Request Body接收第二个参数（必须为JSON对象），脚本须按这个规则来处理对应的参数。".to_owned();
        if self.bypass_permission {
            description.push_str("该方法可以被匿名调用。");
        }
        if self.fileupload {
            description.push_str(&format!(
                "请使用文件上传的方式来进行请求数据。文件数据的字段名为{}",
                self.file_field.clone().unwrap_or("file".to_owned())
            ));
        }

        ins_op = ins_op.description(description);
        if self.fileupload {
            ins_op = ins_op.add_tag("fileupload");
        }

        let mut resp = if self.return_type == *"List" {
            Response::new("返回查询到的对象列表")
        } else if self.return_type == *"Page" {
            Response::new("返回查询到的对象分页列表")
        } else {
            Response::new("返回查询到的对象")
        };

        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_page_schema(RefOr::Type(
                schema::Schema::Object(Box::new(Object::new())),
            ))),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));
        ins_op
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ComposePluginConfig {
    pub services: Vec<ComposeServiceInfo>,
}

unsafe impl Sync for ComposePluginConfig {}

unsafe impl Send for ComposePluginConfig {}

impl ComposePluginConfig {
    pub fn get(&self, name: &str) -> Option<ComposeServiceInfo> {
        self.services
            .clone()
            .into_iter()
            .filter(|p| p.name == *name)
            .next_back()
    }
}

#[allow(dead_code)]
pub struct ComposePluginService {
    namespace: String,
    conf: PluginConfig,
    compose: Mutex<Option<ComposePluginConfig>>,
    service_map: HashMap<String, ComposeServiceInfo>,
    lock_map: HashMap<String, Arc<tokio::sync::Mutex<String>>>,
    script_nss: Mutex<HashMap<String, Vec<String>>>,
}

impl ComposePluginService {
    pub fn new(ns: &str, conf: &PluginConfig) -> Result<Self, anyhow::Error> {
        log::debug!("Plugin config load from {}", conf.config.clone());
        let t = match load_config(conf.config.clone()) {
            Ok(r) => r,
            Err(err) => {
                log::debug!("Could not load the config file: {err:?}");
                Some(ComposePluginConfig::default())
            }
        };

        let mut map = HashMap::new();
        let mut lockmap = HashMap::new();

        if let Some(tcplc) = t.clone() {
            tcplc.services.into_iter().for_each(|f| {
                if f.avoid_reentry {
                    lockmap.insert(
                        f.name.clone(),
                        Arc::new(tokio::sync::Mutex::new(f.name.clone())),
                    );
                }
                map.insert(f.name.clone(), f);
            });
        }

        Ok(Self {
            namespace: ns.to_owned(),
            conf: conf.to_owned(),
            compose: Mutex::new(t),
            service_map: map,
            lock_map: lockmap,
            script_nss: Mutex::new(HashMap::new()),
        })
    }

    pub fn get_named_service(&self, name: &str) -> Option<ComposeServiceInfo> {
        self.service_map.get(name).cloned()
    }

    pub fn get_script_namespaces(&self, name: &str) -> Vec<String> {
        match self.script_nss.lock().unwrap().entry(name.to_owned()) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                occupied_entry.get().clone()
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                if let Some(namedsvc) = self.get_named_service(name) {
                    if let Some(lang) = ExtensionRegistry::get_extension(&namedsvc.lang) {
                        if let Some(func_nss) = lang.fn_eval_script_namespace {
                            if let Ok(nss) = func_nss(&namedsvc.script) {
                                vacant_entry.insert(nss.clone());
                                nss
                            } else {
                                vec![]
                            }
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
        }
    }

    fn try_accquire_lock(
        &self,
        svcname: &str,
    ) -> Result<Option<Arc<tokio::sync::Mutex<String>>>, anyhow::Error> {
        if let Some(lock) = self.lock_map.get(svcname) {
            Ok(Some(lock.clone()))
        } else {
            Ok(None)
        }
    }

    async fn do_prepare_connection(
        nss: &[String],
        plc: &ComposeServiceInfo,
        ctx: &Arc<Mutex<InvocationContext>>,
    ) -> Result<(), anyhow::Error> {
        for ns in nss {
            let dburl = if let Some(mss) = MxStoreService::get(ns) {
                mss.get_db_url()
            } else {
                String::new()
            };

            if !dburl.is_empty() {
                if plc.prepare_connection == Some("transaction".to_owned()) {
                    let tx_opt = ctx.lock().unwrap().get_tx_executor_sync(ns);
                    if tx_opt.is_none() {
                        let rb_ = get_multiple_rbatis_async(&dburl).await;
                        let tcon = rb_
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await?;
                        let con_tx = tcon.begin().await?;
                        let txc = Arc::new(con_tx);
                        // let con = rb_.acquire_begin().await?;
                        // let txc = Arc::new(con);
                        ctx.lock().unwrap().set_tx_executor_sync(ns, txc);
                        // log::warn!("Prepared Transaction Connection");
                    }
                } else if plc.prepare_connection == Some("connection".to_owned()) {
                    let cnn_opt = ctx.lock().unwrap().get_rbatis_connection(ns);
                    if cnn_opt.is_none() {
                        let rb_ = get_multiple_rbatis_async(&dburl).await;
                        let con = rb_
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await?;
                        let xcon: Arc<dyn Executor> = Arc::new(con);
                        ctx.lock().unwrap().set_rbatis_connection(ns, xcon);
                        // log::warn!("Prepared Base Connection");
                    }
                } else {
                    log::warn!("Auto Connection. So no need to create the connection at start.");
                }
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_names(&self) -> Vec<String> {
        self.service_map.keys().cloned().collect()
    }

    pub fn get_services(&self) -> Vec<ComposeServiceInfo> {
        self.service_map.values().cloned().collect()
    }

    fn to_mcp_tool(&self) -> Vec<Value> {
        let mut tools = vec![];
        let tconf = self.compose.lock().unwrap().clone();
        if let Some(restconf) = tconf {
            let conf_name = self.conf.name.clone();
            for svc in restconf.services.iter() {
                if svc.mcp_tool {
                    let ns_ = self.namespace.clone().replace(".", "_");
                    let uri = format!("compose://{}/{}#{}", &self.namespace, &conf_name, svc.name);
                    let ret_type = svc.return_type.clone();
                    let func_name = format!("compose_{}_{}_{}", ns_, &conf_name, svc.name.clone());
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

    fn to_openapi_doc(&self, ns: &str) -> OpenApi {
        let mut openapi = OpenApi::new(self.conf.name.clone(), "0.1.0");

        openapi = openapi.add_schema("UploadFile", to_upload_file_schema());
        let tconf = self.compose.lock().unwrap().clone();
        if let Some(composeconf) = tconf {
            for svc in composeconf.services.clone() {
                if svc.rest_api {
                    let schpath = format!(
                            "/api/compose/{}/{}/{}/schema",
                            ns,
                            self.conf.name.clone(),
                            svc.name
                    );

                    let path = if svc.fileupload {
                        format!(
                            "/api/compose/{}/{}/{}/upload",
                            ns,
                            self.conf.name.clone(),
                            svc.name
                        )
                    } else if svc.return_type == *"List" {
                        format!(
                            "/api/compose/{}/{}/{}/list",
                            ns,
                            self.conf.name.clone(),
                            svc.name
                        )
                    } else if svc.return_type == *"Page" {
                        format!(
                            "/api/compose/{}/{}/{}/page",
                            ns,
                            self.conf.name.clone(),
                            svc.name
                        )
                    } else {
                        format!(
                            "/api/compose/{}/{}/{}/single",
                            ns,
                            self.conf.name.clone(),
                            svc.name
                        )
                    };

                    let opt = svc.to_operation();
                    openapi = openapi.add_path(schpath, PathItem::new(salvo::oapi::PathItemType::Get, svc.to_schema_operation()));
                    if svc.fileupload {
                        openapi = openapi.add_path(
                            path.clone(),
                            PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                        );
                    } else {
                        openapi = openapi.add_path(
                            path.clone(),
                            PathItem::new(salvo::oapi::PathItemType::Get, opt.clone()),
                        );
                        openapi = openapi.add_path(
                            path.clone(),
                            PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                        );
                        openapi = openapi.add_path(
                            path.clone(),
                            PathItem::new(salvo::oapi::PathItemType::Put, opt.clone()),
                        );
                    }
                    if svc.bypass_permission {
                        let mut newpath = path.clone();
                        newpath.insert_str(4, "/passoff");
                        if svc.fileupload {
                            openapi = openapi.add_path(
                                newpath,
                                PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                            );
                        } else {
                            openapi = openapi.add_path(
                                newpath.clone(),
                                PathItem::new(salvo::oapi::PathItemType::Get, opt.clone()),
                            );
                            openapi = openapi.add_path(
                                newpath.clone(),
                                PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                            );
                            openapi = openapi.add_path(
                                newpath.clone(),
                                PathItem::new(salvo::oapi::PathItemType::Put, opt.clone()),
                            );
                        }
                    }

                    if let Some(mcpschema) = svc.mcp_schema.clone() {
                        if let Ok(schema) = serde_json::from_str::<Object>(&mcpschema) {
                            openapi = openapi
                                .add_schema("params", schema::Schema::Object(Box::new(schema)));
                        }
                    }
                }
            }
        }
        openapi
    }

    fn do_add_synctask_queue(css: ComposeServiceInfo, rt: Value) -> Result<(), anyhow::Error> {
        if let Some(task_id) = css.task_id.clone() {
            let state_action = css.execute_delete
                || if let Some(chk) = css.check_delete.clone() {
                    json_path_get(&rt, &chk).is_some()
                } else {
                    false
                };

            let state = if state_action { 2 } else { 1 };

            let taskname = Some(css.name);

            if let Err(err) =
                SyncTaskQueue::get_mut().push_task(&task_id, &taskname, &css.desc, &rt, state)
            {
                log::info!("could not add the value into SyncTaskQueue {err}");
            }
        }
        Ok(())
    }
}

impl RxPluginService for ComposePluginService {

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

    /**
     * 是否需要校验参数签名
     * 参数签名采用
     */
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

    fn invoke_return_option(
        &self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, anyhow::Error>> + Send>> {
        // Compose
        let full_uri = uri.url();
        let st_uri = uri.clone();
        // let ns_ = uri.namespace.clone();
        let nss_ = self.get_script_namespaces(&st_uri.method);
        // log::warn!("nss_: {:?}", nss_);
        let mst_args = args.clone();
        // let mst_args = if args.len() == 1 {
        //     vec![args[0].clone(), args[0].clone()]
        // } else if args.len() == 2 {
        //     if args[0].is_null() {
        //         vec![args[1].clone(), args[1].clone()]
        //     } else {
        //         match args[0].clone() {
        //             Value::Object(t) => if t.is_empty() {
        //                 vec![args[1].clone(), args[1].clone()]
        //             } else {
        //                 vec![args[0].clone(), args[1].clone()]
        //             },
        //             _ => {
        //                 vec![args[0].clone(), args[1].clone()]
        //             }
        //         }
        //     }
        // } else {
        //     args.clone()
        // };

        if let Err(err) = self.verify_params(&st_uri, args.clone()) {
            return Box::pin(async move {
                Err(err)
            });
        }

        let named_service = self.get_named_service(&st_uri.method);
        if let Ok(lock) = self.try_accquire_lock(&st_uri.method) {
            Box::pin(async move {
                if let Some(plc) = named_service {
                    if plc.return_type != *"List" && plc.return_type != *"Page" {
                        let _guard = match &lock {
                            Some(tlock) => Some(tlock.lock().await),
                            None => None,
                        };

                        Self::do_prepare_connection(&nss_, &plc, &ctx).await?;
                        if let Some(lang) = ExtensionRegistry::get_extension(&plc.lang) {
                            if let Some(eval_func) = lang.fn_return_option_script {
                                let mix_args = MxStoreService::invoke_pre_hook_(
                                    full_uri.clone(),
                                    plc.hooks.clone(),
                                    ctx.clone(),
                                    mst_args.clone(),
                                )
                                .await?;

                                match eval_func(&plc.script, ctx.clone(), &mix_args) {
                                    Ok(ret) => {
                                        if plc.enable_synctask {
                                            // add into Queue only
                                            if let Some(retval) = ret.clone() {
                                                Self::do_add_synctask_queue(plc.clone(), retval)?;
                                            }
                                        }
                                        if plc.hooks.is_empty() {
                                            return Ok(ret);
                                        } else {
                                            ctx.lock().unwrap().insert("RETURN_VALUE", ret.clone());
                                            MxStoreService::invoke_post_hook_(
                                                full_uri.clone(),
                                                plc.hooks.clone(),
                                                ctx.clone(),
                                                mix_args,
                                            )
                                            .await?;
                                            return match ctx
                                                .lock()
                                                .unwrap()
                                                .get::<Option<Value>>("RETURN_VALUE")
                                            {
                                                Ok(mt) => Ok(mt.to_owned()),
                                                Err(_) => Ok(ret),
                                            };
                                        }
                                    }
                                    Err(err) => {
                                        if !plc.hooks.is_empty() {
                                            ctx.lock()
                                                .unwrap()
                                                .insert("EXCEPTION", err.to_string());
                                            MxStoreService::invoke_post_hook_(
                                                full_uri.clone(),
                                                plc.hooks.clone(),
                                                ctx.clone(),
                                                mix_args,
                                            )
                                            .await?;
                                        }
                                        return Err(err);
                                    }
                                }
                            }
                        } else if plc.enable_synctask {
                            // add into Queue only
                            Self::do_add_synctask_queue(plc.clone(), args[0].clone())?;
                            return Ok(None);
                        }
                    }
                }
                Err(anyhow!("Not Found {:?}", st_uri.url()))
            })
        } else {
            Box::pin(async move { Err(anyhow!("Not Found -- {:?}", st_uri.url())) })
        }
    }

    fn invoke_return_vec(
        &self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, anyhow::Error>> + Send>> {
        let full_uri = uri.url();
        let st_uri = uri.clone();
        // let ns_ = uri.namespace.clone();
        let nss_ = self.get_script_namespaces(&st_uri.method);
        let named_service = self.get_named_service(&st_uri.method);
        let mst_args = args.clone();
        // let mst_args = if args.len() == 1 {
        //     vec![args[0].clone(), args[0].clone()]
        // } else if args.len() == 2 {
        //     if args[0].is_null() {
        //         vec![args[1].clone(), args[1].clone()]
        //     } else {
        //         match args[0].clone() {
        //             Value::Object(t) => if t.is_empty() {
        //                 vec![args[1].clone(), args[1].clone()]
        //             } else {
        //                 vec![args[0].clone(), args[1].clone()]
        //             },
        //             _ => {
        //                 vec![args[0].clone(), args[1].clone()]
        //             }
        //         }
        //     }
        // } else {
        //     args.clone()
        // };

        if let Err(err) = self.verify_params(&st_uri, args.clone()) {
            return Box::pin(async move {
                Err(err)
            });
        }

        if let Ok(lock) = self.try_accquire_lock(&st_uri.method) {
            
            Box::pin(async move {
                if let Some(plc) = named_service {
                    if plc.return_type == *"List" {
                        let _guard = match &lock {
                            Some(tlock) => Some(tlock.lock().await),
                            None => None,
                        };
                        Self::do_prepare_connection(&nss_, &plc, &ctx).await?;
                        if let Some(lang) = ExtensionRegistry::get_extension(&plc.lang) {
                            if let Some(eval_func) = lang.fn_return_vec_script {
                                let mix_args = MxStoreService::invoke_pre_hook_(
                                    full_uri.clone(),
                                    plc.hooks.clone(),
                                    ctx.clone(),
                                    mst_args.clone(),
                                )
                                .await?;
                                match eval_func(&plc.script, ctx.clone(), &mix_args) {
                                    Ok(ret) => {
                                        if plc.hooks.is_empty() {
                                            log::info!(
                                                "here to return the value directly: {ret:?}"
                                            );
                                            return Ok(ret);
                                        } else {
                                            ctx.lock().unwrap().insert("RETURN_VALUE", ret.clone());
                                            MxStoreService::invoke_post_hook_(
                                                full_uri.clone(),
                                                plc.hooks.clone(),
                                                ctx.clone(),
                                                mix_args,
                                            )
                                            .await?;
                                            return match ctx
                                                .lock()
                                                .unwrap()
                                                .get::<Vec<Value>>("RETURN_VALUE")
                                            {
                                                Ok(mt) => Ok(mt.to_owned()),
                                                Err(_) => Ok(ret),
                                            };
                                        }
                                    }
                                    Err(err) => {
                                        if !plc.hooks.is_empty() {
                                            ctx.lock()
                                                .unwrap()
                                                .insert("EXCEPTION", err.to_string());
                                            MxStoreService::invoke_post_hook_(
                                                full_uri.clone(),
                                                plc.hooks.clone(),
                                                ctx.clone(),
                                                mix_args,
                                            )
                                            .await?;
                                        }
                                        return Err(err);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(anyhow!("Not Found {:?}", st_uri.url()))
            })
        } else {
            Box::pin(async move { Err(anyhow!("Could not get lock for {:?}", st_uri.url())) })
        }
    }

    fn invoke_return_page(
        &self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, anyhow::Error>> + Send>> {
        let full_uri = uri.url();
        let st_uri = uri.clone();
        // let ns_ = uri.namespace.clone();
        let nss_ = self.get_script_namespaces(&st_uri.method);
        let named_service = self.get_named_service(&st_uri.method);
        let mst_args = args.clone();

        // let mst_args = if args.len() == 1 {
        //     vec![args[0].clone(), args[0].clone()]
        // } else if args.len() == 2 {
        //     if args[0].is_null() {
        //         vec![args[1].clone(), args[1].clone()]
        //     } else {
        //         match args[0].clone() {
        //             Value::Object(t) => if t.is_empty() {
        //                 vec![args[1].clone(), args[1].clone()]
        //             } else {
        //                 vec![args[0].clone(), args[1].clone()]
        //             },
        //             _ => {
        //                 vec![args[0].clone(), args[1].clone()]
        //             }
        //         }
        //     }
        // } else {
        //     args.clone()
        // };

        if let Err(err) = self.verify_params(&st_uri, args.clone()) {
            return Box::pin(async move {
                Err(err)
            });
        }

        if let Ok(lock) = self.try_accquire_lock(&st_uri.method) {
            Box::pin(async move {
                if let Some(plc) = named_service {
                    if plc.return_type == *"Page" {
                        let _guard = match &lock {
                            Some(tlock) => Some(tlock.lock().await),
                            None => None,
                        };

                        Self::do_prepare_connection(&nss_, &plc, &ctx).await?;
                        if let Some(lang) = ExtensionRegistry::get_extension(&plc.lang) {
                            if let Some(eval_func) = lang.fn_return_page_script {
                                let mix_args = MxStoreService::invoke_pre_hook_(
                                    full_uri.clone(),
                                    plc.hooks.clone(),
                                    ctx.clone(),
                                    mst_args.clone(),
                                )
                                .await?;
                                match eval_func(&plc.script, ctx.clone(), &mix_args) {
                                    Ok(ret) => {
                                        if plc.hooks.is_empty() {
                                            return Ok(ret);
                                        } else {
                                            ctx.lock().unwrap().insert("RETURN_VALUE", ret.clone());
                                            MxStoreService::invoke_post_hook_(
                                                full_uri.clone(),
                                                plc.hooks.clone(),
                                                ctx.clone(),
                                                mix_args,
                                            )
                                            .await?;
                                            return match ctx
                                                .lock()
                                                .unwrap()
                                                .get::<Page<Value>>("RETURN_VALUE")
                                            {
                                                Ok(mt) => Ok(mt.to_owned()),
                                                Err(_) => Ok(ret),
                                            };
                                        }
                                    }
                                    Err(err) => {
                                        if !plc.hooks.is_empty() {
                                            ctx.lock()
                                                .unwrap()
                                                .insert("EXCEPTION", err.to_string());
                                            MxStoreService::invoke_post_hook_(
                                                full_uri.clone(),
                                                plc.hooks.clone(),
                                                ctx.clone(),
                                                mix_args,
                                            )
                                            .await?;
                                        }
                                        return Err(err);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(anyhow!("Not Found {:?}", st_uri.url()))
            })
        } else {
            Box::pin(async move { Err(anyhow!("Could not get lock for {:?}", st_uri.url())) })
        }
    }

    fn get_config(&self) -> Option<Value> {
        match serde_json::to_value(self.compose.lock().unwrap().clone()) {
            Ok(t) => Some(t),
            Err(err) => {
                log::debug!("Convert to json with error: {err:?}");
                None
            }
        }
    }

    fn parse_config(&self, val: &Value) -> Result<(), anyhow::Error> {
        match serde_json::from_value::<ComposePluginConfig>(val.to_owned()) {
            Ok(t) => {
                self.compose.lock().unwrap().replace(t);
                // self.compose.replace(Some(t));
                Ok(())
            }
            Err(err) => {
                log::warn!("Parse JSON value to config with error: {err:?}");
                Err(anyhow!(err))
            }
        }
    }

    fn save_config(&self, conf: &PluginConfig) -> Result<(), anyhow::Error> {
        let path: PathBuf = conf.config.clone().into();
        chimes_store_core::service::starter::save_config(
            &self.compose.lock().unwrap().clone(),
            path,
        )
    }

    fn get_metadata(&self) -> Vec<chimes_store_core::service::sdk::MethodDescription> {
        let mut desc = vec![];
        if let Some(compse) = self.compose.lock().unwrap().clone() {
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
                    return_page: svc.return_type == *"Page",
                    return_vec: svc.return_type == *"List",
                });
            }
        }
        desc
    }

    fn get_method_metadata(
        &self,
        name: &str,
    ) -> Option<chimes_store_core::service::sdk::MethodDescription> {
        if let Some(compse) = self.compose.lock().unwrap().clone() {
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
                    return_page: svc.return_type == *"Page",
                    return_vec: svc.return_type == *"List",
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
        if let Some(compose) = self.compose.lock().unwrap().to_owned() {
            for it in compose.services.iter().filter(|p| p.name == uri.method) {
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
            // ts.mcp_schema.map(|s| serde_json::from_str(&s).map(|t| Some(t)).unwrap_or(None)).unwrap_or(None)
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

pub fn invoke_shell_script(cs: &str) -> Result<(), anyhow::Error> {
    let lines = cs
        .lines()
        .map(|f| f.to_owned())
        .collect::<Vec<String>>()
        .join(" && ");

    #[cfg(windows)]
    let res = process::Command::new("cmd")
        .arg("/c")
        .arg(lines)
        .stdout(Stdio::piped())
        .spawn();

    #[cfg(not(windows))]
    let res = process::Command::new("bash")
        .arg("-c")
        .arg(lines)
        .stdout(Stdio::piped())
        .spawn();

    match res {
        Ok(child) => match child.wait_with_output() {
            Ok(output) => {
                let codepage = GlobalConfig::config()
                    .console_code_page
                    .unwrap_or("utf-8".to_owned());
                let (text, _enc, _repl) =
                    match encoding_rs::Encoding::for_label(codepage.as_bytes()) {
                        Some(enc) => enc.decode(&output.stdout),
                        None => encoding_rs::UTF_8.decode(&output.stdout),
                    };
                log::debug!("{text}");
                Ok(())
            }
            Err(err) => {
                log::warn!("Could not wait to execute the shell script {err:?}");
                Err(anyhow!("{err}"))
            }
        },
        Err(err) => {
            log::warn!("Could not execute the shell script {err:?}");
            Err(anyhow!("{err}"))
        }
    }
}
