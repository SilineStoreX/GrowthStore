use std::{collections::{HashMap, VecDeque}, future::Future, mem::MaybeUninit, pin::Pin, sync::{Arc, Condvar, Mutex, Once}, time::Duration};

use anyhow::anyhow;
use serde_json::{json, Value};

use super::{invoker::InvocationContext, starter::MxStoreService};



#[derive(Clone)]
pub struct SyncTaskInfo {
    pub task_id: String,
    pub task_object: Value,
    pub state: Option<i64>,   // State: 0, SUCCESS,  1: FOR UPDATE, 2: FOR DELETE
}

impl SyncTaskInfo {
    pub fn new(taskid: &str, tsobj: Value, state: i64) -> Self {
        Self {
            task_id: taskid.to_owned(),
            task_object: tsobj,
            state: Some(state)
        }
    }
}

pub trait SyncWriter {
    fn write(&self, val: &SyncTaskInfo) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>>;
}


pub struct SyncTaskQueue {
    writer_map: HashMap<String, Box<dyn SyncWriter>>,
    queue: VecDeque<SyncTaskInfo>,
    lock: Mutex<i64>,
    cond: Condvar,
}

impl SyncTaskQueue {

    pub fn get_mut() -> &'static mut SyncTaskQueue {
        static mut SYNC_TASK_QUEUE_HOLDER: MaybeUninit<SyncTaskQueue> =
            MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static SYNC_TASK_QUEUE_HOLDER_ONCE: Once = Once::new();

        SYNC_TASK_QUEUE_HOLDER_ONCE.call_once(|| unsafe {
            SYNC_TASK_QUEUE_HOLDER.as_mut_ptr().write(SyncTaskQueue {
                writer_map: HashMap::new(),
                queue: VecDeque::new(),
                lock: Mutex::new(1i64),
                cond: Condvar::new(),
            });
        });

        unsafe { &mut (*SYNC_TASK_QUEUE_HOLDER.as_mut_ptr()) }
    } 

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn pop(&mut self) -> Result<Option<SyncTaskInfo>, anyhow::Error> {
        let g = self.lock.lock().unwrap();
        let o = self.queue.pop_front();
        drop(g);
        Ok(o)
    }

    pub fn has_more(&mut self) -> bool {
        !self.queue.is_empty()
    }

    pub fn push(&mut self, task: &SyncTaskInfo) -> Result<(), anyhow::Error> {
        let g = self.lock.lock().unwrap();
        self.queue.push_back(task.clone());
        self.cond.notify_all();
        drop(g);
        Ok(())
    }

    pub fn push_task(&mut self, task_id:&str, val: &Value, state: i64) -> Result<(), anyhow::Error> {
        let g = self.lock.lock().unwrap();
        self.queue.push_back(SyncTaskInfo::new(task_id, val.to_owned(), state));
        self.cond.notify_all();
        drop(g);
        Ok(())
    }

    pub fn get_writer(&mut self, task_id: &str) -> Option<&dyn SyncWriter> {
        self.writer_map.get(task_id).map(|f| f.to_owned().as_ref())
    }

    pub fn add_writer(&mut self, task_id: &str, writer: Box<dyn SyncWriter>) {
        self.writer_map.insert(task_id.to_string(), writer);
    }

    pub fn wait_for(&mut self) -> Result<(), anyhow::Error> {
        if let Err(err) = self.cond.wait_timeout(self.lock.lock().unwrap(), Duration::from_millis(2000)) {
            Err(anyhow!(err.to_string()))
        } else {
            Ok(())
        }
    }

    pub fn notify_all(&mut self) {
        self.cond.notify_all();
    }

}


pub struct TaskLogger {
    store_uri: Option<String>,
}

impl TaskLogger {
    fn get_mut() -> &'static mut TaskLogger {
        static mut SYNC_TASK_LOGGER_HOLDER: MaybeUninit<TaskLogger> =
            MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static SYNC_TASK_LOGGER_HOLDER_ONCE: Once = Once::new();

        SYNC_TASK_LOGGER_HOLDER_ONCE.call_once(|| unsafe {
            SYNC_TASK_LOGGER_HOLDER.as_mut_ptr().write(TaskLogger {
                store_uri: None
            });
        });

        unsafe { &mut (*SYNC_TASK_LOGGER_HOLDER.as_mut_ptr()) }
    }

    pub fn update_store_uri(uri: &str) {
        Self::get_mut().store_uri = Some(uri.to_owned());
    }

    pub async fn write_log(taskid: &str, nodeid: &str, taskname: &str,  level: &str, subject: &str, desc: &str, success: bool) {
        let val = json!({
            "task_id": taskid, 
            "node_id": nodeid,
            "task_name": taskname,
            "log_level": level,
            "subject": subject,
            "description": desc,
            "success": success 
        });

        if let Some(uri) = Self::get_mut().store_uri.clone() {
            let ctx = Arc::new(Mutex::new(InvocationContext::new()));
            if let Err(err) = MxStoreService::invoke_return_one(uri, ctx, vec![val]).await {
                log::debug!("write to task log with error {err}");
            }
        }
    }

    pub async fn success(taskid: &str, subject: &str, desc: &str) {
        Self::write_log(taskid, taskid, taskid, "SUCCESS", subject, desc, true).await
    }

    pub async fn debug(taskid: &str, subject: &str, desc: &str, success: bool) {
        Self::write_log(taskid, taskid, taskid, "DEBUG", subject, desc, success).await
    }

    pub async fn error(taskid: &str, subject: &str, desc: &str) {
        Self::write_log(taskid, taskid, taskid, "ERROR", subject, desc, false).await
    }

}