use anyhow::anyhow;
use rbatis::Page;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use tokio::task::JoinSet;

use std::any::Any;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
//use std::sync::LazyLock;
use std::time::Duration;

use crate::utils::GlobalConfig;

pub static CHIMES_THREAD_POOL_BLOCK: OnceLock<Pool> = OnceLock::new();
pub static CHIMES_THREAD_POOL: OnceLock<Pool> = OnceLock::new();

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
    AsyncBackend(AsyncJobProc, TxSender),
    BlockProcess(AsyncJobProc),
    AsyncProcess(AsyncJobProc),
    JoinSetProcess(JoinSet<()>),
}

struct Worker {
    _id: usize,
    t: Option<tokio::task::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let t = tokio::spawn(async move {
            CHIMES_THREAD_POOL
                .get()
                .unwrap()
                .counter
                .lock()
                .unwrap()
                .increase_longlive();

            loop {
                let message = match receiver.lock().unwrap().recv() {
                    Ok(msg) => msg,
                    Err(err) => {
                        log::error!("Received {err} Disconnected in Worker {id}.");
                        continue;
                    }
                };
                match message {
                    Message::NewAsyncJob(async_job, tx) => {
                        let ret = async_job.await;
                        CHIMES_THREAD_POOL
                            .get()
                            .unwrap()
                            .counter
                            .lock()
                            .unwrap()
                            .increase_completed();
                        tx.send(ret).unwrap();
                    }
                    Message::NewBackend(job, tx) => {
                        //log::info!("do job without return from worker[{}]", id);
                        // tokio::spawn(job);
                        job.await;
                        CHIMES_THREAD_POOL
                            .get()
                            .unwrap()
                            .counter
                            .lock()
                            .unwrap()
                            .increase_completed();
                        // job.await;
                        //log::info!("finished job without return from worker[{}]", id);
                        tx.send(Box::new(0i64)).unwrap();
                        // log::info!("after increase completed.");
                    }
                    Message::AsyncBackend(job, tx) => {
                        //log::info!("do job without return from worker[{}]", id);
                        tokio::spawn(async move {
                            job.await;
                            CHIMES_THREAD_POOL
                                .get()
                                .unwrap()
                                .counter
                                .lock()
                                .unwrap()
                                .increase_completed();
                        });
                        tokio::task::yield_now().await;
                        // job.await;
                        // job.await;
                        //log::info!("finished job without return from worker[{}]", id);
                        tx.send(Box::new(0i64)).unwrap();
                        // log::info!("after increase completed.");
                    }
                    Message::BlockProcess(job) => {
                        job.await;
                        CHIMES_THREAD_POOL
                            .get()
                            .unwrap()
                            .counter
                            .lock()
                            .unwrap()
                            .increase_completed();
                    }
                    Message::AsyncProcess(job) => {
                        // tokio::spawn(job);
                        tokio::spawn(async move {
                            job.await;
                            CHIMES_THREAD_POOL
                                .get()
                                .unwrap()
                                .counter
                                .lock()
                                .unwrap()
                                .increase_completed();
                        });
                        tokio::task::yield_now().await;
                    }
                    Message::JoinSetProcess(joinset) => {
                        tokio::spawn(async move {
                            joinset.join_all().await;
                        });
                        tokio::task::yield_now().await;
                    }
                    Message::ByeBye => {
                        log::info!("ByeBye from worker[{}]", id);
                        break;
                    }
                }
            }
            CHIMES_THREAD_POOL
                .get()
                .unwrap()
                .counter
                .lock()
                .unwrap()
                .increase_exitlive();
        });

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
    fn increase_longlive(&self);
    fn increase_exitlive(&self);
}

#[allow(dead_code)]
struct MockTaskCounter();

unsafe impl Send for MockTaskCounter {}

unsafe impl Sync for MockTaskCounter {}

impl TaskCounter for MockTaskCounter {
    fn increase_completed(&self) {
        log::error!("increase_completed in MockTaskCounter.");
    }

    fn increase_task(&self) {
        log::error!("increase_task in MockTaskCounter.");
    }

    fn increase_error(&self) {
        log::error!("increase_error in MockTaskCounter.");
    }

    fn increase_longlive(&self) {
        log::error!("increase_longlive in MockTaskCounter.");
    }

    fn increase_exitlive(&self) {
        log::error!("increase_exitlive in MockTaskCounter.");
    }
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

pub struct Pool {
    workers: Vec<Worker>,
    max_workers: usize,
    sender: Mutex<mpsc::Sender<Message>>,
    counter: Mutex<Box<dyn TaskCounter + Send + Sync>>,
    jobs: Mutex<Vec<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>>,
}

impl Pool {
    pub fn renew(&self) {}

    pub fn new(max_workers: usize, ct: Box<dyn TaskCounter + Send + Sync>) -> Pool {
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
            sender: Mutex::new(tx),
            counter: Mutex::new(ct),
            jobs: Mutex::new(Vec::new()),
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
        if let Err(err) = self.sender.lock().unwrap().send(job) {
            log::error!("Submit a job faild {:?}", err);
            self.counter.lock().unwrap().increase_error();
            return;
        }
        match rx.recv_timeout(Duration::from_secs(600)) {
            Ok(bx) => match bx.downcast::<i64>() {
                Ok(_rt) => {}
                Err(_) => {
                    log::info!("submit_sync downcast value error");
                    self.counter.lock().unwrap().increase_error();
                }
            },
            Err(err) => {
                log::info!("submit_sync wait for sync return with error {err}");
                self.counter.lock().unwrap().increase_error();
            }
        };
    }

    /**
     * Submit a job and without wait for return
     */
    pub fn submit_async<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let job = Message::AsyncBackend(Box::pin(f), Box::new(tx));
        self.counter.lock().unwrap().increase_task();
        if let Err(err) = self.sender.lock().unwrap().send(job) {
            log::error!("Submit a job faild {:?}", err);
            self.counter.lock().unwrap().increase_error();
            return;
        }

        match rx.recv_timeout(Duration::from_secs(6000)) {
            Ok(bx) => match bx.downcast::<i64>() {
                Ok(_rt) => {}
                Err(_) => {
                    self.counter.lock().unwrap().increase_error();
                }
            },
            Err(err) => {
                log::error!("submit_async wait for return with error {err}");
                self.counter.lock().unwrap().increase_error();
            }
        };
    }

    /**
     * Submit a job and without wait for return
     */
    pub fn blockon_process<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let job = Message::BlockProcess(Box::pin(f));
        self.counter.lock().unwrap().increase_task();
        if let Err(err) = self.sender.lock().unwrap().send(job) {
            log::error!("post a async job faild {:?}", err);
            self.counter.lock().unwrap().increase_error();
        }
    }

    /**
     * Submit a job and without wait for return
     */
    pub fn async_process<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let job = Message::AsyncProcess(Box::pin(f));
        self.counter.lock().unwrap().increase_task();
        if let Err(err) = self.sender.lock().unwrap().send(job) {
            log::error!("post a async job faild {:?}", err);
            self.counter.lock().unwrap().increase_error();
        }
    }

    pub fn execute_blockon<F, T>(&self, f: F) -> Result<T, anyhow::Error>
    where
        F: Future<Output = JobOutput> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let job = Message::NewAsyncJob(Box::pin(f), Box::new(tx));
        self.counter.lock().unwrap().increase_task();
        if let Err(err) = self.sender.lock().unwrap().send(job) {
            log::warn!("send the job to backend thread with an error {:?}", err);
            self.counter.lock().unwrap().increase_error();
            return Err(anyhow!(err.to_string()));
        }

        match rx.recv_timeout(Duration::from_secs(30000)) {
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

    /**
     * create a new thread
     */
    pub fn spawn_task<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.counter.lock().unwrap().increase_longlive();
        tokio::spawn(async {
            f.await;
            CHIMES_THREAD_POOL
                .get()
                .unwrap()
                .counter
                .lock()
                .unwrap()
                .increase_exitlive();
        });
    }

    pub fn join_tasks<F>(&self, runnow: bool, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.jobs.lock().unwrap().push(Box::pin(f));

        if runnow || self.jobs.lock().unwrap().len() >= self.max_workers {
            let joinset = JoinSet::new();
            // self.jobs.lock().unwrap().iter().for_each(|f| {
            //    joinset.spawn(f);
            // });
            let job = Message::JoinSetProcess(joinset);
            self.counter.lock().unwrap().increase_task();
            if let Err(err) = self.sender.lock().unwrap().send(job) {
                log::error!("post a async job faild {:?}", err);
                self.counter.lock().unwrap().increase_error();
            }
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        for _ in 0..self.max_workers {
            self.sender.lock().unwrap().send(Message::ByeBye).unwrap();
        }
        for w in self.workers.iter_mut() {
            if let Some(t) = w.t.take() {
                // t.join().unwrap();
                t.abort();
            }
        }
    }
}

pub fn init_async_task_pool(
    ct: Box<dyn TaskCounter + Send + Sync>,
    xt: Box<dyn TaskCounter + Send + Sync>,
) {
    log::info!(
        "Workthread: {}, Pool Size: {}",
        GlobalConfig::get_worker_threads(),
        GlobalConfig::get_pool_size()
    );
    let _ = CHIMES_THREAD_POOL.get_or_init(|| Pool::new(GlobalConfig::get_pool_size() / 2, ct));

    let _ =
        CHIMES_THREAD_POOL_BLOCK.get_or_init(|| Pool::new(GlobalConfig::get_pool_size() / 2, xt));
}
