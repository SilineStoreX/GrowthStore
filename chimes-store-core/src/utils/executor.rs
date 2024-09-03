use anyhow::anyhow;
use futures_lite::Future;
use std::pin::Pin;
use rbatis::Page;
use serde_json::Value;

use lazy_static::lazy_static;
use std::any::Any;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use crate::utils::GlobalConfig;

lazy_static! {
    pub static ref TOKIO_EXCEED_RUNTIME: tokio::runtime::Runtime = tokio::runtime::Runtime::new().unwrap();
        // tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
}

lazy_static! {
    /**
     * 共享的线程池
     * 创建的线程数量与当前电脑的CPU数量相当
     * 注意：
     * 1、当请求的数量大于线程池的数量时，后面的请求需要等待前面的请求执行完毕后，才能得到执行
     * 2、其中的队列使用的的mpsc::channel实现
     * 3、Pool的执行效率大概有1ms的损失（实际测试，AMD CPU笔记本）
     * 4、这个值未来应该是可以配置的。它将占用tokio::runtime的这个数量的线程，如果，它的值与tokio的runtime work_threads相同，
     *    系统将会死锁
     */
    pub static ref CHIMES_THREAD_POOL: Pool = Pool::new(GlobalConfig::get_pool_size()); 
}

type JobOutput = Box<dyn Any + Send + Sync>;
// type JobFn = Box<dyn core::ops::FnOnce() -> JobOutput + 'static + Send>;
type AsyncJob = Pin<Box<dyn Future<Output = JobOutput> + 'static + Send>>;
type AsyncJobProc = Pin<Box<dyn Future<Output = ()> + 'static + Send>>;
// type JobProc = Box<dyn core::ops::FnOnce() + 'static + Send>;
type TxSender = Box<mpsc::Sender<Box<dyn Any + Send + Sync>>>;
enum Message {
    ByeBye,
    NewAsyncJob(AsyncJob, TxSender),
    NewBackend(AsyncJobProc, TxSender),
}

struct Worker {
    _id: usize,
    t: Option<tokio::task::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let t = tokio::spawn(async move { loop {
            let message = receiver.lock().unwrap().recv().unwrap();
            match message {
                Message::NewAsyncJob(async_job, tx) => {
                    log::info!(
                        "do job with return from worker[{}] on thread {:?}",
                        id,
                        std::thread::current().id()
                    );
                    let ret = async_job.await;
                    tx.send(ret).unwrap();
                },
                Message::NewBackend(job, tx) => {
                    //log::info!("do job without return from worker[{}]", id);
                    tokio::spawn(job);
                    // job.await;
                    //log::info!("finished job without return from worker[{}]", id);
                    tx.send(Box::new(0)).unwrap();
                    // log::info!("after increase completed.");
                },
                Message::ByeBye => {
                    log::info!("ByeBye from worker[{}]", id);
                    break;
                }
            }
        }});

        Worker {
            _id: id,
            t: Some(t),
        }
    }
}

pub trait TaskCounter {
    fn increase_completed(&self);
    fn increase_task(&self);
    fn increase_error(&self);
}

struct MockTaskCounter();

unsafe impl Send for MockTaskCounter {}

unsafe impl Sync for MockTaskCounter {}

impl TaskCounter for MockTaskCounter {
    fn increase_completed(&self) {}

    fn increase_task(&self) {}

    fn increase_error(&self) {}
}

#[allow(dead_code)]
fn is_error(ret: &JobOutput) -> bool {
    if ret.is::<Result<Vec<Value>, anyhow::Error>>() {
        match ret.downcast_ref::<Result<Vec<Value>, anyhow::Error>>() {
            Some(t) => t.is_err(),
            None => false,
        }
    } else if ret.is::<Result<Page<Value>, anyhow::Error>>() {
        match ret.downcast_ref::<Result<Page<Value>, anyhow::Error>>() {
            Some(t) => t.is_err(),
            None => false,
        }
    } else if ret.is::<Result<Option<Value>, anyhow::Error>>() {
        match ret.downcast_ref::<Result<Option<Value>, anyhow::Error>>() {
            Some(t) => t.is_err(),
            None => false,
        }
    } else if ret.is::<Result<Value, anyhow::Error>>() {
        match ret.downcast_ref::<Result<Value, anyhow::Error>>() {
            Some(t) => t.is_err(),
            None => false,
        }
    } else if ret.is::<Box<Result<Option<Value>, anyhow::Error>>>() {
        match ret.downcast_ref::<Box<Result<Option<Value>, anyhow::Error>>>() {
            Some(t) => t.is_err(),
            None => false,
        }
    } else if ret.is::<Box<Result<Value, anyhow::Error>>>() {
        match ret.downcast_ref::<Box<Result<Value, anyhow::Error>>>() {
            Some(t) => t.is_err(),
            None => false,
        }
    } else if ret.is::<Box<Result<Vec<Value>, anyhow::Error>>>() {
        match ret.downcast_ref::<Box<Result<Vec<Value>, anyhow::Error>>>() {
            Some(t) => t.is_err(),
            None => false,
        }
    } else if ret.is::<Box<Result<Page<Value>, anyhow::Error>>>() {
        match ret.downcast_ref::<Box<Result<Page<Value>, anyhow::Error>>>() {
            Some(t) => t.is_err(),
            None => false,
        }
    } else {
        false
    }
}

#[repr(C)]
pub struct Pool {
    workers: Vec<Worker>,
    max_workers: usize,
    sender: mpsc::Sender<Message>,
    counter: Mutex<Box<dyn TaskCounter + Send + Sync>>,
}

impl Pool {
    pub fn new(max_workers: usize) -> Pool {
        let act_workers = if max_workers == 0 {
            num_cpus::get()
        } else {
            max_workers
        };

        let (tx, rx) = mpsc::channel();

        let mut workers = Vec::with_capacity(act_workers);
        let receiver = Arc::new(Mutex::new(rx));
        for i in 0..act_workers {
            workers.push(Worker::new(i, Arc::clone(&receiver)));
        }

        Pool {
            workers,
            max_workers: act_workers,
            sender: tx,
            counter: Mutex::new(Box::new(MockTaskCounter())),
        }
    }

    pub fn setup_counter(&self, ct: Box<dyn TaskCounter + Send + Sync>) {
        *self.counter.lock().unwrap() = ct;
    }

    /**
     * Submit a job and without wait for return
     */
    pub fn submit<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let job = Message::NewBackend(Box::pin(f), Box::new(tx));
        self.counter.lock().unwrap().increase_task();
        if let Err(err) = self.sender.send(job) {
            log::error!("Submit a job faild {:?}", err);
            self.counter.lock().unwrap().increase_error();
        }
        match rx.recv_timeout(Duration::from_secs(600)) {
            Ok(bx) => match bx.downcast::<i64>() {
                Ok(_rt) => {
                    self.counter.lock().unwrap().increase_completed();
                },
                Err(_) => {
                    self.counter.lock().unwrap().increase_error();
                }
            },
            Err(_) => {
                self.counter.lock().unwrap().increase_error();
            }
        };
    }

    pub fn execute_async<F, T>(&self, f: F) -> Result<T, anyhow::Error>
    where
        F: Future<Output = JobOutput> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let job = Message::NewAsyncJob(Box::pin(f), Box::new(tx));
        self.counter.lock().unwrap().increase_task();
        if let Err(err) = self.sender.send(job) {
            log::warn!("send the job to backend thread with an error {:?}", err);
            self.counter.lock().unwrap().increase_error();
            return Err(anyhow!(err.to_string()));
        }

        match rx.recv_timeout(Duration::from_secs(600)) {
            Ok(bx) => match bx.downcast::<T>() {
                Ok(rt) => Ok(*rt),
                Err(_) => {
                    self.counter.lock().unwrap().increase_error();
                    Err(anyhow::anyhow!("Result cannot be cast"))
                }
            },
            Err(err) => {
                self.counter.lock().unwrap().increase_error();
                Err(anyhow::anyhow!("timeout {err}"))
            }
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        for _ in 0..self.max_workers {
            self.sender.send(Message::ByeBye).unwrap();
        }
        for w in self.workers.iter_mut() {
            if let Some(t) = w.t.take() {
                // t.join().unwrap();
                t.abort();
            }
        }
    }
}
