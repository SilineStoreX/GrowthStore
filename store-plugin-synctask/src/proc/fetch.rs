use crate::proc::template::json_path_get;
use chimes_store_core::{
    config::{ConditionItem, QueryCondition},
    service::{
        invoker::InvocationContext, queue::SyncTaskQueue, sched::JobInvoker,
        starter::MxStoreService,
    },
    tasklog_error, tasklog_info, tasklog_success, tasklog_warn,
};
use rbatis::rbdc::{Date, DateTime};
use std::{
    collections::{hash_map::Entry, HashMap},
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU64},
        Arc, Mutex,
    },
};
use tera::Number;

use super::{template::template_eval, SyncTaskDefinition, SyncVariable, VariableValue};
use serde_json::{json, Value};

pub const FETCH_AND_WRITE_RETRY_TIMES: u64 = 8;

pub trait SyncReader {
    fn interval_second(&self) -> Option<u64>;
    fn cron_express(&self) -> Option<String>;
    fn fetch(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>>;
    fn to_invoker(&self) -> Box<dyn JobInvoker + Send + Sync>;
}

#[derive(Debug, Clone)]
pub struct RestapiConfig {
    pub cron_express: Option<String>,
    pub interval_second: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct QueryRestapiReader {
    conf: RestapiConfig,
}

#[allow(dead_code)]
pub struct QueryRestapiReaderInvoker {
    inner: QueryRestapiReader,
}

impl QueryRestapiReaderInvoker {
    pub fn new(inner: &QueryRestapiReader) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

unsafe impl Send for QueryRestapiReader {}

unsafe impl Sync for QueryRestapiReader {}

impl JobInvoker for QueryRestapiReaderInvoker {
    fn exec(&self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send + 'static>> {
        Box::pin(async move { Ok(()) })
    }

    fn get_invoke_uri(&self) -> String {
        String::new()
    }

    fn get_invoke_params(&self) -> Option<Value> {
        None
    }

    fn get_description(&self) -> String {
        String::new()
    }
}

impl SyncReader for QueryRestapiReader {
    fn interval_second(&self) -> Option<u64> {
        self.conf.interval_second
    }

    fn cron_express(&self) -> Option<String> {
        self.conf.cron_express.clone()
    }

    fn fetch(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>> {
        todo!()
    }

    fn to_invoker(&self) -> Box<dyn JobInvoker + Send + Sync> {
        Box::new(QueryRestapiReaderInvoker::new(self))
    }
}

// We should defined a Object protocol to manage the variable

#[derive(Clone)]
pub struct GeneralStoreUriReader {
    namespace: String,
    variables: Vec<SyncVariable>,
    task: SyncTaskDefinition,
    cookie: Arc<AtomicU64>,
}

impl GeneralStoreUriReader {
    pub fn new(ns: &str, vars: &[SyncVariable], task: &SyncTaskDefinition) -> Self {
        Self {
            namespace: ns.to_owned(),
            variables: vars.to_vec(),
            task: task.clone(),
            cookie: Arc::new(AtomicU64::new(1)),
        }
    }

    #[allow(dead_code)]
    pub async fn load_variable_redis(
        ns: &str,
        task_id: &str,
        varbs: &[SyncVariable],
    ) -> HashMap<String, Value> {
        let mut hash = HashMap::new();
        let ctx = Arc::new(Mutex::new(InvocationContext::new()));
        for v in varbs {
            if let Some(varname) = v.var_name.clone() {
                let uri = format!("redis://{ns}/redis#get?{task_id}.{varname}");
                if let Ok(t) = MxStoreService::invoke_return_one(uri, ctx.clone(), vec![]).await {
                    if let Some(tval) = t {
                        hash.insert(varname, tval);
                    } else {
                        hash.insert(varname, v.to_default_value());
                    }
                }
            }
        }
        hash
    }

    #[allow(dead_code)]
    pub async fn update_variables_redis(
        ns: &str,
        task_id: &str,
        varis: &[SyncVariable],
        map: &HashMap<String, VariableValue>,
    ) -> Result<(), anyhow::Error> {
        let ctx = Arc::new(Mutex::new(InvocationContext::new()));
        for it in varis {
            let wrt = it.var_write.clone().unwrap_or_default().to_uppercase();
            let dtexpr = it.var_data_express.clone().unwrap_or_default();
            let varn = it.var_name.clone().unwrap_or_default();
            // let vartype = it.var_type.clone().unwrap_or_default();
            let tval = if wrt == *"CURRENT_DATE" {
                let dtnow = DateTime::now();
                let dt = Date::from(dtnow.0);
                dt.to_string()
            } else if wrt == *"CURRENT_DATETIME" {
                let dtnow = DateTime::now();
                dtnow.to_string()
            } else if wrt == *"MIN" || wrt == *"MAX" {
                if let Some(curr) = map.get(&varn) {
                    curr.to_string()
                } else {
                    String::new()
                }
            } else if wrt == *"SQL" || wrt == *"INVOKEURI" {
                let gtctx = Arc::new(Mutex::new(InvocationContext::new()));
                match if wrt.starts_with("object://") || wrt.starts_with("query://") {
                    MxStoreService::invoke_return_vec(dtexpr, gtctx, vec![]).await
                } else {
                    MxStoreService::execute_query(ns, gtctx, &dtexpr, &[]).await
                } {
                    Ok(ts) => {
                        if ts.is_empty() {
                            it.to_default_value()
                                .as_str()
                                .map(|t| t.to_owned())
                                .unwrap_or_default()
                        } else {
                            let rt = ts[0].clone();
                            match rt.as_object() {
                                Some(ts) => {
                                    let mut valret = String::new();
                                    if ts.len() == 1 {
                                        if let Some(val) = ts.values().next_back() {
                                            match val {
                                                Value::String(t) => valret.clone_from(t),
                                                Value::Number(mt) => valret = mt.to_string(),
                                                _ => valret = val.to_string(),
                                            }
                                        }
                                    } else {
                                        for key in ts.keys() {
                                            if key.ends_with("_value") {
                                                let val = ts
                                                    .get(key)
                                                    .map(|t| t.to_owned())
                                                    .unwrap_or_default();
                                                match val {
                                                    Value::String(t) => valret = t,
                                                    Value::Number(mt) => valret = mt.to_string(),
                                                    _ => valret = val.to_string(),
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    valret
                                }
                                None => rt.as_str().map(|t| t.to_owned()).unwrap_or_default(),
                            }
                        }
                    }
                    Err(_err) => it
                        .to_default_value()
                        .as_str()
                        .map(|t| t.to_owned())
                        .unwrap_or_default(),
                }
            } else {
                String::new()
            };

            let nsuri = format!("redis://{ns}/redis#set?{task_id}.{varn}");
            // let taskid = task_id.to_owned();

            if let Err(err) =
                MxStoreService::invoke_return_one(nsuri, ctx.clone(), vec![Value::String(tval)])
                    .await
            {
                log::info!("error when update variable {varn}. {err}");
            }
        }
        Ok(())
    }

    pub async fn load_variable(
        ns: &str,
        task_id: &str,
        varbs: &[SyncVariable],
    ) -> HashMap<String, Value> {
        let mut hash = HashMap::new();
        let nsuri = format!("object://{ns}/SyncTaskVariables#query");
        let ctx = Arc::new(Mutex::new(InvocationContext::new()));

        let mut qc = QueryCondition::default();
        qc.and.push(ConditionItem {
            field: "task_id".to_owned(),
            op: "=".to_owned(),
            value: Value::String(task_id.to_owned()),
            ..Default::default()
        });
        let args = vec![serde_json::to_value(qc).unwrap()];
        if let Ok(ret) = MxStoreService::invoke_return_vec(nsuri, ctx, args).await {
            // log::warn!("query variable: {ret:?}");
            for tc in ret {
                if let Ok(syncval) = serde_json::from_value::<SyncVariable>(tc) {
                    if let Some(text) = syncval.var_name.clone() {
                        hash.insert(text, syncval.to_default_value());
                    }
                }
            }
        }

        for it in varbs.iter().cloned() {
            let varname = it.var_name.clone().unwrap_or_default();
            if !varname.is_empty() {
                if let Entry::Vacant(vc) = hash.entry(varname) {
                    vc.insert(it.to_default_value());
                }

                // hash.entry(varname).or_insert_with(|| it.to_default_value());
                // if !hash.contains_key(&varname) {
                //    hash.insert(varname, it.to_default_value());
                // }
            }
        }

        // 如果hash的大小没有定义变量，则需要往里面加入变理的
        hash
    }

    // 调用该方法对变量进行更新
    // 如果是使用数据的更新办法，则以task_id和var_name作为唯一性条件进行更新，var_value都转换成为String
    // 如果是使用redis，则使用task_id.var_name的方式来更新值，同样的var_value都转换成为String
    pub async fn update_variable(
        ns: &str,
        task_id: &str,
        varis: &[SyncVariable],
        map: &HashMap<String, VariableValue>,
    ) -> Result<(), anyhow::Error> {
        let ctx = Arc::new(Mutex::new(InvocationContext::new()));
        for it in varis {
            let wrt = it.var_write.clone().unwrap_or_default().to_uppercase();
            let varn = it.var_name.clone().unwrap_or_default();
            let vartype = it.var_type.clone().unwrap_or_default();
            let tval = if wrt == *"CURRENT_DATE" {
                let dtnow = DateTime::now();
                let dt = Date::from(dtnow.0);
                dt.to_string()
            } else if wrt == *"CURRENT_DATETIME" {
                let dtnow = DateTime::now();
                dtnow.to_string()
            } else if wrt == *"MIN" || wrt == *"MAX" {
                if let Some(curr) = map.get(&varn) {
                    curr.to_string()
                } else {
                    // String::new()
                    continue;
                }
            } else if wrt == *"SQL" || wrt == *"INVOKEURI" {
                let dtexpr = it.var_data_express.clone().unwrap_or_default();
                let gtctx = Arc::new(Mutex::new(InvocationContext::new()));
                match if wrt.starts_with("object://") || wrt.starts_with("query://") {
                    MxStoreService::invoke_return_vec(dtexpr, gtctx, vec![]).await
                } else {
                    MxStoreService::execute_query(ns, gtctx, &dtexpr, &[]).await
                } {
                    Ok(ts) => {
                        if ts.is_empty() {
                            it.to_default_value()
                                .as_str()
                                .map(|t| t.to_owned())
                                .unwrap_or_default()
                        } else {
                            let rt = ts[0].clone();
                            match rt.as_object() {
                                Some(ts) => {
                                    let mut valret = String::new();
                                    if ts.len() == 1 {
                                        if let Some(val) = ts.values().next_back() {
                                            match val {
                                                Value::String(t) => valret.clone_from(t),
                                                Value::Number(mt) => valret = mt.to_string(),
                                                _ => valret = val.to_string(),
                                            }
                                        }
                                    } else {
                                        for key in ts.keys() {
                                            if key.ends_with("_value") {
                                                let val = ts
                                                    .get(key)
                                                    .map(|t| t.to_owned())
                                                    .unwrap_or_default();
                                                match val {
                                                    Value::String(t) => valret = t,
                                                    Value::Number(mt) => valret = mt.to_string(),
                                                    _ => valret = val.to_string(),
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    valret
                                }
                                None => rt.as_str().map(|t| t.to_owned()).unwrap_or_default(),
                            }
                        }
                    }
                    Err(_err) => it
                        .to_default_value()
                        .as_str()
                        .map(|t| t.to_owned())
                        .unwrap_or_default(),
                }
            } else {
                // String::new()
                continue;
            };

            let nsuri = format!("object://{ns}/SyncTaskVariables#upsert");
            let taskid = task_id.to_owned();

            let varup = json!({
                "task_id": taskid.clone(),
                "var_name": varn.clone(),
                "var_type": vartype.clone(),
                "var_value": tval,
                "state": 1
            });

            // let varup = json!({"var_value": tval});

            let varcond = json!({"and": [{"field": "task_id", "op": "=", "value": taskid}, {"field": "var_name", "op": "=", "value": varn}]});

            if let Err(err) =
                MxStoreService::invoke_return_one(nsuri, ctx.clone(), vec![varup, varcond]).await
            {
                log::info!("error when update variable {varn}. {err}");
            }
        }
        Ok(())
    }

    /**
     * 该方法动态更新指定变量的当前值
     * 主要用于MIN/MAX
     * 类型必须为Date/DateTime或i64
     */
    fn update_variable_current(
        map: &mut HashMap<String, VariableValue>,
        varbs: &[SyncVariable],
        curr_obj: &Value,
    ) {
        for v in varbs {
            let name = v.var_name.clone().unwrap_or_default();
            let dt = v.var_type.clone().unwrap_or_default().to_lowercase();
            let wrt = v.var_write.clone().unwrap_or_default().to_uppercase();
            let dtexpr = v.var_data_express.clone().unwrap_or_default();
            if wrt == "MIN" {
                if let Some(val) = json_path_get(curr_obj, &dtexpr) {
                    let varval = VariableValue::from_json_value(
                        val,
                        dt == "date" || dt == "datetime",
                        dt == "float",
                        dt == "number" || dt == "long",
                    );
                    if let Some(st) = map.get(&name) {
                        if varval < *st {
                            map.insert(name, varval);
                        }
                    } else {
                        map.insert(name, varval);
                    }
                }
            } else if wrt == "MAX" {
                if let Some(val) = json_path_get(curr_obj, &dtexpr) {
                    let varval = VariableValue::from_json_value(
                        val,
                        dt == "date" || dt == "datetime",
                        dt == "float",
                        dt == "number" || dt == "long",
                    );
                    if let Some(st) = map.get(&name) {
                        if varval > *st {
                            map.insert(name, varval);
                        }
                    } else {
                        map.insert(name, varval);
                    }
                }
            }
        }
    }
}

impl SyncReader for GeneralStoreUriReader {
    fn interval_second(&self) -> Option<u64> {
        self.task.interval_second.map(|s| s as u64)
    }

    fn cron_express(&self) -> Option<String> {
        self.task.cron_express.clone()
    }

    fn fetch(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>> {
        let uri = self.task.source_uri.clone().unwrap_or_default();
        let reqbody_tmp = self.task.source_request.clone().unwrap_or_default();
        let ns = self.namespace.clone();
        let task_id = self.task.task_id.clone();
        let varbs = self.variables.clone();
        let taskconf = self.task.clone();
        let cookie = self
            .cookie
            .fetch_add(1, std::sync::atomic::Ordering::Acquire);

        Box::pin(async move {
            let mut current_map = HashMap::new();
            let varmap = Self::load_variable(&ns, &task_id, &varbs).await;
            let mut varval = varmap
                .into_iter()
                .collect::<serde_json::Map<String, Value>>();
            let error_times = AtomicI32::new(0);
            let error_exit = AtomicBool::new(false);
            let page_no = AtomicI32::new(1);
            let load_data = AtomicI64::new(0);
            let has_data = AtomicBool::new(false);
            let next_page = AtomicBool::new(true);

            tasklog_info!(
                &task_id,
                cookie,
                "general.uri.reader.job.starting",
                "Starting Job to fetch records by general uri reader",
                true
            );

            loop {
                // lookup the variables
                // setup paged variables
                // eval the template to string
                let pno = page_no.load(std::sync::atomic::Ordering::Acquire);
                let ps = taskconf.page_size.unwrap_or(10);
                // log::info!("Then, we will fetch next page ({pno} by {ps}).");

                let next_page_request = next_page.load(std::sync::atomic::Ordering::Acquire);
                if next_page_request && taskconf.paged_request {
                    varval.insert("page_no".to_owned(), Value::Number(Number::from(pno)));
                    varval.insert("page_size".to_owned(), Value::Number(Number::from(ps)));
                }

                // log::error!("try to eval template by reqbody_tmp {reqbody_tmp}, {varval:?}");

                let reqargs = if !reqbody_tmp.is_empty() {
                    match template_eval(&reqbody_tmp, Value::Object(varval.clone())) {
                        Ok(text) => {
                            log::info!("reqbody: {text}");
                            if let Ok(ts) = serde_json::from_str::<Value>(&text) {
                                if ts.is_array() {
                                    ts.as_array().map(|f: &Vec<Value>| f.to_owned()).unwrap()
                                } else {
                                    vec![ts]
                                }
                            } else {
                                log::warn!("could not parse to json");
                                tasklog_error!(
                                    &task_id,
                                    cookie,
                                    "error.parse.json",
                                    "error to parse the request body to json"
                                );
                                error_exit.store(true, std::sync::atomic::Ordering::Release);
                                break;
                            }
                        }
                        Err(err) => {
                            log::warn!("Could not parse to json/template {err}");
                            tasklog_error!(
                                &task_id,
                                cookie,
                                "error.parse.template",
                                &format!("error to transform template with error {err}")
                            );
                            // vec![json!({}), json!({"paging":{"size": 100, "current": 1}})]
                            error_exit.store(true, std::sync::atomic::Ordering::Release);
                            break;
                        }
                    }
                } else {
                    log::debug!("No req body defined.");
                    vec![json!({}), json!({"paging":{"size": ps, "current": pno}})]
                };

                let page_size = ps as usize;

                let ctx = Arc::new(Mutex::new(InvocationContext::new()));

                let res = if taskconf.paged_request {
                    match MxStoreService::invoke_return_page(uri.clone(), ctx, reqargs).await {
                        Ok(rt) => {
                            if !rt.records.is_empty() {
                                page_no.fetch_add(1, std::sync::atomic::Ordering::Release);
                                let record_len = rt.records.len();
                                load_data.fetch_add(
                                    record_len as i64,
                                    std::sync::atomic::Ordering::Acquire,
                                );
                                // next_page, now, it just calling one page, may be we need to call many paged(times)
                                next_page.store(
                                    record_len > 0 && record_len <= page_size,
                                    std::sync::atomic::Ordering::Release,
                                );
                                has_data.store(true, std::sync::atomic::Ordering::Release);
                            } else {
                                next_page.store(false, std::sync::atomic::Ordering::Release);
                                has_data.store(false, std::sync::atomic::Ordering::Release);
                            }
                            rt.records
                        }
                        Err(err) => {
                            // let btrace = std::backtrace::Backtrace::force_capture().to_string();
                            log::error!("could not fetch by paged : {err}");
                            let new_error_times =
                                error_times.fetch_add(1, std::sync::atomic::Ordering::Acquire);
                            tasklog_error!(
                                &task_id,
                                cookie,
                                "error.fetch.data",
                                &format!("error to read from {uri} with error {err}. retried {new_error_times} times.")
                            );
                            let msg = err.to_string();
                            if msg.contains("timeout")
                                && new_error_times < FETCH_AND_WRITE_RETRY_TIMES as i32
                            {
                                continue;
                            }

                            vec![]
                        }
                    }
                } else {
                    match MxStoreService::invoke_return_vec(uri.clone(), ctx, reqargs).await {
                        Ok(rt) => {
                            if !rt.is_empty() {
                                has_data.store(true, std::sync::atomic::Ordering::Release);
                                load_data.fetch_add(
                                    rt.len() as i64,
                                    std::sync::atomic::Ordering::Acquire,
                                );
                            }
                            rt
                        }
                        Err(err) => {
                            // let btrace = std::backtrace::Backtrace::force_capture().to_string();
                            log::info!("could not fetch by vec: {err}");
                            let new_error_times =
                                error_times.fetch_add(1, std::sync::atomic::Ordering::Acquire);
                            tasklog_error!(
                                &task_id,
                                cookie,
                                "error.fetch.data",
                                &format!("error to read from {uri} with error {err}. retried {new_error_times} times.")
                            );
                            let msg = err.to_string();
                            if msg.contains("timeout")
                                && new_error_times < FETCH_AND_WRITE_RETRY_TIMES as i32
                            {
                                continue;
                            }

                            vec![]
                        }
                    }
                };

                // 现在有一个问题，如果查询时（数据库查询）都是在同一个线程（起始）应该就可以正常执行下去。
                // 如果它们是跨线程了，如写入操作时，我们用了去获取了相应的东西，则会出现内存错误（段错误，或非法访问），这是一个奇怪的问题。
                for tr in res {
                    // 保持一致性，我们在这里只需要检查是否为删除标志
                    //if let Ok(val) = tr /* do_sync_transform(&taskconf, &tr).await */ {
                    let val = tr.clone();
                    {
                        let state = if let Some(chk) = taskconf.check_delete.clone() {
                            if json_path_get(&val, &chk).is_some() {
                                2
                            } else {
                                1
                            }
                        } else {
                            1
                        };

                        let taskname = taskconf.target_uri.clone();
                        let subject = taskconf.task_desc.clone();

                        // log::info!("Values: {val:?}");
                        // log::warn!("Add into sync task queue. {task_id}");

                        if let Err(err) = SyncTaskQueue::get_mut()
                            .push_task(&task_id, &taskname, &subject, &val, state)
                        {
                            log::info!("add to sync task queue failed: {err}");
                        }

                        Self::update_variable_current(&mut current_map, &varbs, &val);
                    }
                }
                // }

                let next_page_againt = next_page.load(std::sync::atomic::Ordering::Acquire);
                if !next_page_againt || !taskconf.paged_request {
                    break;
                }

                // If there are many error, the loop will be break.
                if error_times.load(std::sync::atomic::Ordering::Acquire) > 10 {
                    error_exit.store(true, std::sync::atomic::Ordering::Release);
                    break;
                }
                // page_no.fetch_add(1, std::sync::atomic::Ordering::Acquire);
            }

            let exit_on_error = error_exit.load(std::sync::atomic::Ordering::Acquire);

            if !exit_on_error {
                let hasdata = has_data.load(std::sync::atomic::Ordering::Acquire);
                let error_time_count = error_times.load(std::sync::atomic::Ordering::Acquire);
                if hasdata || taskconf.update_variable_epoch {
                    if let Err(err) =
                        Self::update_variable(&ns, &task_id, &varbs, &current_map).await
                    {
                        log::info!("update the variable failed: {err}");
                    }
                }

                let loaded_records = load_data.load(std::sync::atomic::Ordering::Acquire);
                if error_time_count > 0 {
                    tasklog_warn!(
                        &task_id,
                        cookie,
                        "warn.fetch.data",
                        &format!("warn to read from {uri} with {loaded_records} records and have {error_time_count} errors."),
                        true
                    );
                } else {
                    tasklog_success!(
                        &task_id,
                        cookie,
                        "success.fetch.data",
                        &format!("success to read from {uri} with {loaded_records} records")
                    );
                }
            } else {
                log::error!("exit on error occurred.");
                tasklog_error!(
                    &task_id,
                    cookie,
                    "error.fetch.data",
                    &format!("error to read from {uri}")
                );
            }

            Ok(())
        })
    }

    fn to_invoker(&self) -> Box<dyn JobInvoker + Send + Sync> {
        Box::new(GeneralStoreUriReaderJob {
            inner: self.clone(),
        })
    }
}

pub struct GeneralStoreUriReaderJob {
    inner: GeneralStoreUriReader,
}

impl JobInvoker for GeneralStoreUriReaderJob {
    fn exec(&self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>> {
        let mut urireader = self.inner.clone();
        Box::pin(async move {
            log::info!("start to fetch the records...");
            if let Err(err) = urireader.fetch().await {
                log::info!("error for reader : {err}");
                Err(err)
            } else {
                Ok(())
            }
        })
    }

    fn get_invoke_uri(&self) -> String {
        format!(
            "synctask://{}/{}",
            self.inner.namespace.clone(),
            self.inner.task.task_id.clone()
        )
    }

    fn get_invoke_params(&self) -> Option<Value> {
        self.inner
            .task
            .params
            .clone()
            .map(|s| serde_json::from_str::<Value>(&s).unwrap_or(Value::Null))
    }

    fn get_description(&self) -> String {
        self.inner.task.task_desc.clone().unwrap_or_default()
    }
}
