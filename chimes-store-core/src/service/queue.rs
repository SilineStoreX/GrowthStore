use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    sync::{atomic::AtomicU64, Arc, Condvar, Mutex, OnceLock},
    time::Duration,
};

use anyhow::anyhow;
use serde_json::{json, Map, Value};

use super::{invoker::InvocationContext, starter::MxStoreService};

#[derive(Clone)]
pub struct SyncTaskInfo {
    pub task_id: String,
    pub task_object: Value,
    pub state: Option<i64>, // State: 0, SUCCESS,  1: FOR UPDATE, 2: FOR DELETE
    pub message: Option<String>, // Store the message into.
    pub subject: Option<String>,
    pub taskname: Option<String>,
}

impl SyncTaskInfo {
    pub fn new(
        taskid: &str,
        taskname: &Option<String>,
        subject: &Option<String>,
        tsobj: Value,
        state: i64,
    ) -> Self {
        Self {
            task_id: taskid.to_string(),
            task_object: tsobj,
            state: Some(state),
            message: None,
            subject: taskname.clone(),
            taskname: subject.clone(),
        }
    }
}

pub trait SyncWriter: Send + Sync {
    fn write(
        &self,
        val: &SyncTaskInfo,
    ) -> Pin<Box<dyn Future<Output = Result<SyncTaskInfo, anyhow::Error>> + Send>>;
}

pub struct SyncTaskQueue {
    writer_map: Mutex<HashMap<String, Arc<Box<dyn SyncWriter>>>>,
    queue: Mutex<VecDeque<SyncTaskInfo>>,
    // lock: Mutex<i64>,
    cond: Condvar,
    receive_count: Mutex<HashMap<String, AtomicU64>>,
    consume_count: Mutex<HashMap<String, AtomicU64>>,
    write_count: Mutex<HashMap<String, AtomicU64>>,
    error_count: Mutex<HashMap<String, AtomicU64>>,
}

impl SyncTaskQueue {
    pub fn get_mut() -> &'static SyncTaskQueue {
        static SYNC_TASK_QUEUE_HOLDER: OnceLock<SyncTaskQueue> = OnceLock::new();
        // Once带锁保证只进行一次初始化

        SYNC_TASK_QUEUE_HOLDER.get_or_init(|| {
            SyncTaskQueue {
                writer_map: Mutex::new(HashMap::new()),
                queue: Mutex::new(VecDeque::new()),
                // lock: Mutex::new(1i64),
                cond: Condvar::new(),
                receive_count: Mutex::new(HashMap::new()),
                consume_count: Mutex::new(HashMap::new()),
                write_count: Mutex::new(HashMap::new()),
                error_count: Mutex::new(HashMap::new()),
            }
        })
    }

    pub async fn post_log(&self, task: SyncTaskInfo) {
        // send the result of this task after write
        let success = task.state.map(|t| t == 0).unwrap_or_default();
        let desc = task.message.unwrap_or_default();
        let level = if success { "INFO" } else { "ERROR" };
        let subject = task.subject.unwrap_or_default();
        let taskname = task.taskname.unwrap_or_default();
        TaskLogger::write_log(
            &task.task_id,
            "Write",
            &taskname,
            level,
            &subject,
            &desc,
            success,
        )
        .await;
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }

    pub fn pop(&self) -> Result<Option<SyncTaskInfo>, anyhow::Error> {
        let o = self.queue.lock().unwrap().pop_front();

        if o.is_some() {
            let tp = o.clone().unwrap();
            self.increase_consume(&tp.task_id);
        }

        Ok(o)
    }

    pub fn has_more(&self) -> bool {
        !self.queue.lock().unwrap().is_empty()
    }

    pub fn push(&self, task: &SyncTaskInfo) -> Result<(), anyhow::Error> {
        self.queue.lock().unwrap().push_back(task.clone());
        self.cond.notify_all();
        let task_id = task.task_id.clone();

        if self.receive_count.lock().unwrap().contains_key(&task_id) {
            self.receive_count
                .lock()
                .unwrap()
                .get(&task_id)
                .map(|f| f.fetch_add(1, std::sync::atomic::Ordering::Acquire));
        } else {
            self.receive_count
                .lock()
                .unwrap()
                .insert(task_id.to_string(), AtomicU64::new(1));
        }

        Ok(())
    }

    pub fn push_task(
        &self,
        task_id: &str,
        taskname: &Option<String>,
        subject: &Option<String>,
        val: &Value,
        state: i64,
    ) -> Result<(), anyhow::Error> {
        self.queue.lock().unwrap().push_back(SyncTaskInfo::new(
            task_id,
            taskname,
            subject,
            val.clone(),
            state,
        ));
        self.cond.notify_all();
        if self.receive_count.lock().unwrap().contains_key(task_id) {
            self.receive_count
                .lock()
                .unwrap()
                .get(task_id)
                .map(|f| f.fetch_add(1, std::sync::atomic::Ordering::Acquire));
        } else {
            self.receive_count
                .lock()
                .unwrap()
                .insert(task_id.to_string(), AtomicU64::new(1));
        }
        Ok(())
    }

    pub fn increase_consume(&self, task_id: &str) {
        if self.consume_count.lock().unwrap().contains_key(task_id) {
            self.consume_count
                .lock()
                .unwrap()
                .get(task_id)
                .map(|f| f.fetch_add(1, std::sync::atomic::Ordering::Acquire));
        } else {
            self.consume_count
                .lock()
                .unwrap()
                .insert(task_id.to_string(), AtomicU64::new(1));
        }
    }

    pub fn increase_write(&self, task_id: &str) {
        if self.write_count.lock().unwrap().contains_key(task_id) {
            self.write_count
                .lock()
                .unwrap()
                .get(task_id)
                .map(|f| f.fetch_add(1, std::sync::atomic::Ordering::Acquire));
        } else {
            self.write_count
                .lock()
                .unwrap()
                .insert(task_id.to_string(), AtomicU64::new(1));
        }
    }

    pub fn increase_error(&self, task_id: &str) {
        if self.error_count.lock().unwrap().contains_key(task_id) {
            self.error_count
                .lock()
                .unwrap()
                .get(task_id)
                .map(|f| f.fetch_add(1, std::sync::atomic::Ordering::Acquire));
        } else {
            self.error_count
                .lock()
                .unwrap()
                .insert(task_id.to_string(), AtomicU64::new(1));
        }
    }

    pub fn get_count(&self) -> Option<Value> {
        let keys = self
            .receive_count
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<String>>();
        let mut valmap = Map::new();
        for k in keys {
            let rc_count = if let Some(v) = self.receive_count.lock().unwrap().get(&k) {
                v.load(std::sync::atomic::Ordering::Acquire)
            } else {
                0u64
            };
            let wc_count = if let Some(v) = self.write_count.lock().unwrap().get(&k) {
                v.load(std::sync::atomic::Ordering::Acquire)
            } else {
                0u64
            };
            let cs_count = if let Some(v) = self.consume_count.lock().unwrap().get(&k) {
                v.load(std::sync::atomic::Ordering::Acquire)
            } else {
                0u64
            };

            let err_count = if let Some(v) = self.error_count.lock().unwrap().get(&k) {
                v.load(std::sync::atomic::Ordering::Acquire)
            } else {
                0u64
            };

            valmap.insert(k.clone(), json!({"received": rc_count, "written": wc_count, "consumed": cs_count, "error": err_count}));
        }
        Some(Value::Object(valmap))
    }

    pub fn lookup_and_write(
        &self,
        task_id: &str,
        task: &SyncTaskInfo,
    ) -> Pin<Box<dyn Future<Output = Result<SyncTaskInfo, anyhow::Error>> + Send>> {
        self.writer_map
            .lock()
            .unwrap()
            .get(task_id)
            .map(|f| f.write(task))
            .unwrap_or(Box::pin(async { Err(anyhow!("Unable do it")) }))
    }

    //pub fn get_writer(&'static self, task_id: &str) -> Option<&'static Box<dyn SyncWriter>> {
    // self.writer_map.lock().unwrap().get(task_id)
    //}

    pub async fn invoke_write<F>(&self, task_id: &str, func: F) -> Result<(), anyhow::Error>
    where
        F: AsyncFnOnce(&Box<dyn SyncWriter>) -> (),
    {
        let wrt = match self.writer_map.lock() {
            Ok(ts) => {
                if let Some(w) = ts.get(task_id) {
                    // func(w).await;
                    // Ok(())
                    w.to_owned()
                } else {
                    return Err(anyhow!("Not found the writer by {task_id}"));
                }
            }
            Err(err) => {
                return Err(anyhow!("error for lock {err}"));
            }
        };

        func(&wrt).await;
        Ok(())
    }

    pub fn add_writer(&self, task_id: &str, writer: Box<dyn SyncWriter>) {
        self.writer_map
            .lock()
            .unwrap()
            .insert(task_id.to_string(), Arc::new(writer));
    }

    pub fn remove_writer(&self, task_id: &str) {
        self.writer_map.lock().unwrap().remove(task_id);
    }

    pub fn wait_for(&self) -> Result<(), anyhow::Error> {
        if let Err(err) = self
            .cond
            .wait_timeout(self.queue.lock().unwrap(), Duration::from_millis(2000))
        {
            Err(anyhow!(err.to_string()))
        } else {
            Ok(())
        }
    }

    pub fn notify_all(&self) {
        self.cond.notify_all();
    }
}

pub struct TaskLogger {
    store_uri: Option<String>,
}

impl TaskLogger {
    fn get() -> &'static tokio::sync::Mutex<TaskLogger> {
        static SYNC_TASK_LOGGER_HOLDER: OnceLock<tokio::sync::Mutex<TaskLogger>> = OnceLock::new();
        // Once带锁保证只进行一次初始化

        SYNC_TASK_LOGGER_HOLDER
            .get_or_init(|| tokio::sync::Mutex::new(TaskLogger { store_uri: None }))
    }

    pub async fn update_store_uri(uri: &str) {
        Self::get().lock().await.store_uri = Some(uri.to_owned());
    }

    pub async fn write_log(
        taskid: &str,
        nodeid: &str,
        taskname: &str,
        level: &str,
        subject: &str,
        desc: &str,
        success: bool,
    ) {
        let val = json!({
            "task_id": taskid,
            "node_id": nodeid,
            "task_name": taskname,
            "log_level": level,
            "subject": subject,
            "description": desc,
            "success": success
        });

        if level.to_lowercase() == "warn" || level.to_lowercase() == "error" {
            log::warn!("tasklog warn: {val:?}");
        } else if level.to_lowercase() == "info" {
            log::debug!("tasklog info: {val:?}");
        }

        if let Some(uri) = Self::get().lock().await.store_uri.clone() {
            let ctx = Arc::new(std::sync::Mutex::new(InvocationContext::new()));
            if let Err(err) = MxStoreService::invoke_return_one(uri, ctx, vec![val]).await {
                log::debug!("write to task log with error {err}");
            }
        }
    }
}
