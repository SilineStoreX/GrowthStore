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
use chimes_store_core::utils::GlobalConfig;
use rbatis::Page;
use salvo::oapi::{
    schema, Content, Object, OpenApi, Operation, PathItem, RefOr, RequestBody, Response, Schema,
    ToArray,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::{self, Stdio};
use std::sync::{Arc, Mutex};

use super::template::json_path_get;

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
    pub cron_express: Option<String>,
    pub schedule_simulate: Option<String>,
    pub script: String,
    pub return_type: String,

    #[serde(default)]
    pub enable_synctask: bool,
    pub task_id: Option<String>,

    #[serde(default)]
    pub execute_delete: bool,      // 执行删除动作，如果为true，所有接收到的都都是删除操作 
    pub check_delete: Option<String>,  // 检查删除标识，JSONPath表示，检查成功，表示该记录应该执行删除操作


    #[serde(default)]
    pub perm_roles: Vec<String>,

    #[serde(default)]
    pub bypass_permission: bool, // 允许匿名访问，只有在允许匿名访问的时候，才能通过passoff调用

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
    apiresult = apiresult.property("records", t.to_array());
    to_api_result_schema(RefOr::T(schema::Schema::Object(apiresult)), false)
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
    schema::Schema::Object(apiresult)
}

impl ComposeServiceInfo {
    pub(crate) fn to_operation(&self) -> Operation {
        let mut ins_op = Operation::new();
        if self.fileupload {
            ins_op = ins_op.request_body(
                RequestBody::new()
                    .add_content("application/json", RefOr::T(to_upload_file_schema())),
            );
        } else {
            ins_op = ins_op.request_body(
                RequestBody::new()
                    .add_content("application/json", RefOr::T(Schema::Object(Object::new()))),
            );
        }
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
            Content::new(to_api_result_page_schema(RefOr::T(schema::Schema::Object(
                Object::new(),
            )))),
        );
        ins_op = ins_op.add_response("200", RefOr::T(resp));
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
            .last()
    }
}

#[allow(dead_code)]
pub struct ComposePluginService {
    namespace: String,
    conf: PluginConfig,
    compose: Mutex<Option<ComposePluginConfig>>,
    service_map: HashMap<String, ComposeServiceInfo>,
}

impl ComposePluginService {
    pub fn new(ns: &str, conf: &PluginConfig) -> Result<Self, anyhow::Error> {
        log::debug!("Plugin config load from {}", conf.config.clone());
        let t = match load_config(conf.config.clone()) {
            Ok(r) => r,
            Err(err) => {
                log::debug!("Could not load the config file: {:?}", err);
                Some(ComposePluginConfig::default())
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
            compose: Mutex::new(t),
            service_map: map,
        })
    }

    pub fn get_named_service(&self, name: &str) -> Option<ComposeServiceInfo> {
        self.service_map.get(name).cloned()
    }

    #[allow(dead_code)]
    pub fn get_names(&self) -> Vec<String> {
        self.service_map.keys().cloned().collect()
    }

    pub fn get_services(&self) -> Vec<ComposeServiceInfo> {
        self.service_map.values().cloned().collect()
    }

    fn to_openapi_doc(&self, ns: &str) -> OpenApi {
        let mut openapi = OpenApi::new(self.conf.name.clone(), "0.1.0");

        openapi = openapi.add_schema("UploadFile", to_upload_file_schema());
        let tconf = self.compose.lock().unwrap().clone();
        if let Some(composeconf) = tconf {
            for svc in composeconf.services.clone() {
                if svc.rest_api {
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
                }
            }
        }
        openapi
    }


    fn do_add_synctask_queue(css: ComposeServiceInfo, rt: Value) -> Result<(), anyhow::Error> {
        if let Some(task_id) = css.task_id.clone() {
            let state_action = css.execute_delete || if let Some(chk) = css.check_delete.clone() {
                json_path_get(&rt, &chk).is_some()
            } else {
                false
            };

            let state = if state_action {
                2
            } else {
                1
            };

            if let Err(err) = SyncTaskQueue::get_mut().push_task(&task_id, &rt, state) {
                log::info!("could not add the value into SyncTaskQueue {err}");
            }
        }
        Ok(())
    }
}

impl RxPluginService for ComposePluginService {
    fn invoke_return_option(
        &self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, anyhow::Error>> + Send>> {
        // Compose
        let full_uri = uri.url();
        let st_uri = uri.clone();
        let named_service = self.get_named_service(&st_uri.method);
        Box::pin(async move {
            if let Some(plc) = named_service {
                if plc.return_type != *"List" && plc.return_type != *"Page" {
                    if let Some(lang) = ExtensionRegistry::get_extension(&plc.lang) {
                        if let Some(eval_func) = lang.fn_return_option_script {
                            let mix_args = MxStoreService::invoke_pre_hook_(
                                full_uri.clone(),
                                plc.hooks.clone(),
                                ctx.clone(),
                                args.clone(),
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
                                        ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
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
    }

    fn invoke_return_vec(
        &self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, anyhow::Error>> + Send>> {
        let full_uri = uri.url();
        let st_uri = uri.clone();
        let named_service = self.get_named_service(&st_uri.method);
        Box::pin(async move {
            if let Some(plc) = named_service {
                if plc.return_type == *"List" {
                    if let Some(lang) = ExtensionRegistry::get_extension(&plc.lang) {
                        if let Some(eval_func) = lang.fn_return_vec_script {
                            let mix_args = MxStoreService::invoke_pre_hook_(
                                full_uri.clone(),
                                plc.hooks.clone(),
                                ctx.clone(),
                                args.clone(),
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
                                            .get::<Vec<Value>>("RETURN_VALUE")
                                        {
                                            Ok(mt) => Ok(mt.to_owned()),
                                            Err(_) => Ok(ret),
                                        };
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
                                    return Err(err);
                                }
                            }
                        }
                    }
                }
            }
            Err(anyhow!("Not Found {:?}", st_uri.url()))
        })
    }

    fn invoke_return_page(
        &self,
        uri: InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, anyhow::Error>> + Send>> {
        let full_uri = uri.url();
        let st_uri = uri.clone();
        let named_service = self.get_named_service(&st_uri.method);
        Box::pin(async move {
            if let Some(plc) = named_service {
                if plc.return_type == *"Page" {
                    if let Some(lang) = ExtensionRegistry::get_extension(&plc.lang) {
                        if let Some(eval_func) = lang.fn_return_page_script {
                            let mix_args = MxStoreService::invoke_pre_hook_(
                                full_uri.clone(),
                                plc.hooks.clone(),
                                ctx.clone(),
                                args.clone(),
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
                                        ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
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
    }

    fn get_config(&self) -> Option<Value> {
        match serde_json::to_value(self.compose.lock().unwrap().clone()) {
            Ok(t) => Some(t),
            Err(err) => {
                log::debug!("Convert to json with error: {:?}", err);
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
                log::warn!("Parse JSON value to config with error: {:?}", err);
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

pub fn invoke_shell_script(cs: &str) {
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
                log::debug!("{}", text.to_string());
            }
            Err(err) => {
                log::warn!("Could not wait to execute the shell script {:?}", err);
            }
        },
        Err(err) => {
            log::warn!("Could not execute the shell script {:?}", err);
        }
    }
}
