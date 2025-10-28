use super::{template::template_eval, SyncTaskDefinition};
use anyhow::anyhow;
use chimes_store_core::{
    pin_spawnthread,
    service::{
        invoker::InvocationContext,
        queue::{SyncTaskInfo, SyncTaskQueue, SyncWriter},
        script::ExtensionRegistry,
        starter::MxStoreService,
    },
};
use chimes_store_utils::template::json_path_get;
use serde_json::{json, Value};
use std::{
    future::Future,
    pin::Pin,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

#[allow(dead_code)]
pub struct DummySyncWriter(pub(crate) SyncTaskDefinition);

impl DummySyncWriter {}

impl SyncWriter for DummySyncWriter {
    fn write(
        &self,
        val: &SyncTaskInfo,
    ) -> Pin<Box<dyn Future<Output = Result<SyncTaskInfo, anyhow::Error>> + Send>> {
        let ctval = val.to_owned();
        Box::pin(async move {
            log::info!("SYNC: {}", ctval.task_object);
            Ok(ctval)
        })
    }
}

#[allow(dead_code)]
pub struct GeneralStoreUriWriter {
    task: SyncTaskDefinition,
    namespace: String,
}

impl GeneralStoreUriWriter {
    pub fn new(ns: &str, task: &SyncTaskDefinition) -> Self {
        Self {
            task: task.clone(),
            namespace: ns.to_owned(),
        }
    }
}

pub async fn do_sync_transform(
    task: &SyncTaskDefinition,
    val: &Value,
) -> Result<Value, anyhow::Error> {
    let lang = task.lang.clone().unwrap_or("tera".to_owned());
    let transform = task.transform.clone().unwrap_or_default();

    // log::error!("do_sync_transform by {lang}, {transform}");

    if transform.is_empty() {
        Ok(val.to_owned())
    } else if lang == *"tera" {
        let param = task
            .params
            .clone()
            .map(|f| serde_json::from_str::<Value>(&f).unwrap_or(Value::Null))
            .unwrap_or(Value::Null);
        let ctxval = json!({"params": param, "value": val});
        match template_eval(&transform, ctxval) {
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(valtret) => Ok(valtret),
                Err(err) => Err(anyhow!(err)),
            },
            Err(err) => Err(anyhow!(err)),
        }
    } else if lang == *"invoke_uri" {
        let ctx = Arc::new(Mutex::new(InvocationContext::new()));
        let param = task
            .params
            .clone()
            .map(|f| serde_json::from_str::<Value>(&f).unwrap_or(Value::Null))
            .unwrap_or(Value::Null);
        match MxStoreService::invoke_return_one(transform, ctx, vec![val.to_owned(), param]).await {
            Ok(ts) => {
                if let Some(rt) = ts {
                    Ok(rt)
                } else {
                    Ok(Value::Null)
                }
            }
            Err(err) => {
                log::info!("error {err}");
                Ok(Value::Null)
            }
        }
    } else {
        let param = task
            .params
            .clone()
            .map(|f| serde_json::from_str::<Value>(&f).unwrap_or(Value::Null))
            .unwrap_or(Value::Null);
        match ExtensionRegistry::get_extension(&lang) {
            Some(langext) => match langext.fn_return_option_script {
                Some(tvalscript) => {
                    let ctx = Arc::new(Mutex::new(InvocationContext::new()));
                    match tvalscript(&transform, ctx, &[val.to_owned(), param]) {
                        Ok(ts) => {
                            if let Some(tsu) = ts {
                                Ok(tsu)
                            } else {
                                Ok(Value::Null)
                            }
                        }
                        Err(err) => Err(anyhow!(err)),
                    }
                }
                None => Err(anyhow!(
                    "Unknown eval return option value function for {lang}."
                )),
            },
            None => Err(anyhow!("Unknown lang {lang}.")),
        }
    }
}

/**
 * GeneralStoreUriWriter使用InvokeURI来执行写入操作
 */
impl SyncWriter for GeneralStoreUriWriter {
    fn write(
        &self,
        val: &SyncTaskInfo,
    ) -> Pin<Box<dyn Future<Output = Result<SyncTaskInfo, anyhow::Error>> + Send>> {
        let taskdef = self.task.clone();
        let mut valt = val.clone();
        Box::pin(async move {
            // let mut mutval = valt.clone();
            valt.taskname = taskdef.task_desc.clone();
            valt.subject = taskdef.task_desc.clone();
            let res = do_sync_transform(&taskdef, &valt.task_object.clone()).await?;
            let targeturi = if valt.state == Some(1i64) {
                // 执行UPDATE/INSERT的操作
                taskdef.target_uri.unwrap_or_default()
            } else if valt.state == Some(2i64) {
                // 执行delete操作
                taskdef.remove_uri.unwrap_or_default()
            } else {
                String::new()
            };

            if targeturi.is_empty() {
                return Err(anyhow!("Unknow target Store InvokeURI defined."));
            }

            if res.is_null() {
                log::debug!("After transform is NULL.");
                return Ok(valt);
            }

            let reqargs = if res.is_array() {
                res.as_array().map(|f| f.to_owned()).unwrap()
            } else {
                vec![res]
            };

            log::debug!(
                "Sync Writer: {targeturi}, Args: {}",
                serde_json::to_string(&reqargs).unwrap_or_default()
            );

            let ctx = Arc::new(Mutex::new(InvocationContext::new()));
            match MxStoreService::invoke_return_one(targeturi, ctx, reqargs).await {
                Ok(tt) => {
                    let ct = if let Some(vt) = tt { vt } else { Value::Null };

                    let pattern = match taskdef.check_pattern {
                        Some(pt) => pt.trim().to_owned(),
                        None => String::new(),
                    };

                    if pattern.is_empty() {
                        Ok(valt)
                    } else {
                        match json_path_get(&ct, &pattern) {
                            Some(_) => Ok(valt),
                            None => Err(anyhow!(
                                "Could not pass the result check by {pattern}. the result is {}",
                                serde_json::to_string(&ct).unwrap_or_default()
                            )),
                        }
                    }
                }
                Err(err) => {
                    // log::error!("write error {err}");
                    Err(anyhow!(err))
                }
            }
        })
    }
}

#[allow(dead_code)]
pub async fn lookup_and_execute(
    task_id: &str,
    task: &SyncTaskInfo,
) -> Result<SyncTaskInfo, anyhow::Error> {
    SyncTaskQueue::get_mut()
        .lookup_and_write(task_id, task)
        .await
}

// 启动落库线程
pub fn start_write_thread(start: Arc<AtomicBool>) {
    let is_started = start.load(std::sync::atomic::Ordering::Acquire);

    if is_started {
        return;
    }

    pin_spawnthread!(async move {
        start.store(true, std::sync::atomic::Ordering::Release);
        log::warn!("start the fetch thread now.");
        loop {
            let started = start.load(std::sync::atomic::Ordering::Acquire);
            if !started {
                log::info!("exit the SyncTaskQueue loop.");
                break;
            }
            // calling the method
            tokio::task::yield_now().await;
            // log::warn!("try to pop the SyncTask object ... {}", SyncTaskQueue::get_mut().len());
            if let Ok(Some(mut ts)) = SyncTaskQueue::get_mut().pop() {
                // lookup the writer
                let task_id = ts.task_id.clone();
                // log::warn!("Pop a task {task_id}");
                // Use pin_submit to invoke the function as now.
                let task_proc = async move {
                    // use thread-pool to process concurrently.
                    log::debug!("execute the lookup write for task {task_id}.");
                    match lookup_and_execute(&task_id, &ts).await {
                        Ok(_) => {
                            // log::warn!("execute the lookup write --- writted.");
                            SyncTaskQueue::get_mut().increase_write(&task_id);
                            ts.message = Some("SUCCESS".to_string());
                            ts.state = Some(0);
                        }
                        Err(err) => {
                            ts.state = Some(1);
                            ts.message = Some(format!("write {task_id} failed by {err}"));
                            SyncTaskQueue::get_mut().increase_error(&task_id);
                            log::warn!("write {task_id} failed {err}");
                        }
                    };

                    SyncTaskQueue::get_mut().post_log(ts).await;
                };
                task_proc.await;

                // tokio::task::yield_now().await; // perform to another cpu task
            } else if let Err(err) = SyncTaskQueue::get_mut().wait_for() {
                log::warn!("waiting for next with error {err:?}");
            }
        }
    });
}
