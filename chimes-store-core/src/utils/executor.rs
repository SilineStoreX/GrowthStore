use anyhow::anyhow;
use rbatis::Page;
use serde_json::Value;
use std::backtrace::Backtrace;
use std::future::Future;
use std::pin::Pin;
use tokio::runtime::Runtime;
use tokio::task::JoinSet;

use std::any::Any;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
//use std::sync::LazyLock;
use crate::utils::GlobalConfig;
use std::time::Duration;
use tokio_util::context::TokioContext;

pub static CHIMES_THREAD_POOL_BLOCK: OnceLock<Pool> = OnceLock::new();
pub static CHIMES_THREAD_POOL: OnceLock<Pool> = OnceLock::new();

// static CHIMES_RUNTIME_EXECUTOR: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

static GLOBAL_RUNTIME_EXECUTOR: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub static CHIMES_THREAD_POOL_COUNTER: OnceLock<Mutex<Box<dyn TaskCounter + Send + Sync>>> =
    OnceLock::new();

type JobOutput = Box<dyn Any + Send + Sync>;
// type JobFn = Box<dyn core::ops::FnOnce() -> JobOutput + 'static + Send>;
type AsyncJob = Pin<Box<dyn Future<Output = JobOutput> + 'static + Send>>;
type AsyncJobProc = Pin<Box<dyn Future<Output = ()> + 'static + Send>>;
// type JobProc = Box<dyn core::ops::FnOnce() + 'static + Send>;
type TxSender = Box<mpsc::Sender<Box<dyn Any + Send + Sync>>>;
enum Message {
    ByeBye,
    NewAsyncJob(AsyncJob, TxSender),
    AsyncBlockJob(AsyncJob, TxSender),
    NewBackend(AsyncJobProc, TxSender),
    AsyncBackend(AsyncJobProc, TxSender),
    BlockProcess(AsyncJobProc),
    AsyncProcess(AsyncJobProc),
    JoinSetProcess(JoinSet<()>),
    Empty,
}

unsafe impl Send for Message {}

struct Worker {
    _id: usize,
    t: Option<tokio::task::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let fut = async move {
            CHIMES_THREAD_POOL_COUNTER
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .increase_longlive();
            let thread_name = std::thread::current()
                .name()
                .unwrap_or("unknown-worker-thread")
                .to_string();
            log::debug!("Thread {thread_name} is working for Worker {id}");
            loop {
                let message = match receiver
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(1))
                {
                    Ok(msg) => msg,
                    Err(err) => match err {
                        mpsc::RecvTimeoutError::Timeout => Message::Empty,
                        mpsc::RecvTimeoutError::Disconnected => {
                            log::error!("Received {err} Disconnected in Worker {id}.");
                            continue;
                        }
                    },
                };

                match message {
                    Message::NewAsyncJob(async_job, tx) => {
                        log::debug!(
                            "[Worker {id}] ({thread_name}) received a blockon function job."
                        );
                        let ret = async_job.await;
                        CHIMES_THREAD_POOL_COUNTER
                            .get()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .increase_completed();
                        log::debug!(
                            "[Worker {id}] ({thread_name}) finished the blockon function job."
                        );

                        if let Err(err) = tx.send(ret) {
                            log::info!("[NewAsyncJob] Unable to send the result {err}");
                        }
                    }
                    Message::AsyncBlockJob(async_job, tx) => {
                        log::debug!(
                            "[Worker {id}] ({thread_name}) received a blockon function job."
                        );
                        let thread_name_clone = thread_name.clone();

                        let ft = async move {
                            log::warn!("Calling the async_job now...");
                            let ret = async_job.await;
                            CHIMES_THREAD_POOL_COUNTER
                                .get()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .increase_completed();
                            log::debug!(
                                "[Worker {id}] ({thread_name_clone}) finished the blockon function job."
                            );

                            if is_error(&ret) {
                                log::warn!("It return an error.");
                            }

                            log::warn!("sending the return by async_job.");
                            if let Err(err) = tx.send(ret) {
                                log::info!("[AsyncBlockJob] Unable to send the result {err}");
                            }
                        };
                        let h = GLOBAL_RUNTIME_EXECUTOR.get().unwrap().handle();
                        let fut = TokioContext::new(ft, h.to_owned());
                        match tokio::spawn(fut).await {
                            Ok(_) => {
                                log::info!("thread called finish.");
                            }
                            Err(_err) => {
                                log::info!("Thread join error");
                            }
                        }
                    }
                    Message::NewBackend(job, tx) => {
                        //log::info!("do job without return from worker[{}]", id);
                        // tokio::spawn(job);
                        log::debug!("[Worker {id}] ({thread_name}) received a blockon submit job.");
                        job.await;
                        CHIMES_THREAD_POOL_COUNTER
                            .get()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .increase_completed();
                        log::debug!(
                            "[Worker {id}] ({thread_name}) finished the blockon submit job."
                        );
                        // job.await;
                        //log::info!("finished job without return from worker[{}]", id);
                        if let Err(err) = tx.send(Box::new(0i64)) {
                            log::info!("[NewBackend] Unable to send the result {err}");
                        }
                        // log::info!("after increase completed.");
                    }
                    Message::AsyncBackend(job, tx) => {
                        //log::info!("do job without return from worker[{}]", id);
                        let th = thread_name.clone();
                        log::debug!("[Worker {id}] ({th}) received a async submit job.");
                        tokio::spawn(async move {
                            job.await;
                            log::debug!("[Worker {id}] ({th}) finished the async submit job.");
                            CHIMES_THREAD_POOL_COUNTER
                                .get()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .increase_completed();
                        });
                        tokio::task::yield_now().await;
                        // job.await;
                        // job.await;
                        //log::info!("finished job without return from worker[{}]", id);
                        // tx.send(Box::new(0i64)).unwrap();
                        if let Err(err) = tx.send(Box::new(0i64)) {
                            log::info!("[AsyncBackend] Unable to send the result {err}");
                        }
                    }
                    Message::BlockProcess(job) => {
                        let th = thread_name.clone();
                        log::debug!("[Worker {id}] ({th}) received a blockon procedure process.");
                        job.await;
                        CHIMES_THREAD_POOL_COUNTER
                            .get()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .increase_completed();
                        log::debug!("[Worker {id}] ({th}) finished the blockon procedure process.");
                    }
                    Message::AsyncProcess(job) => {
                        // tokio::spawn(job);
                        let th = thread_name.clone();
                        log::debug!("[Worker {id}] ({th}) received a async procedure process.");

                        tokio::spawn(async move {
                            job.await;
                            log::debug!(
                                "[Worker {id}] ({th}) finished the async procedure process."
                            );
                            CHIMES_THREAD_POOL_COUNTER
                                .get()
                                .unwrap()
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
                    Message::Empty => {
                        tokio::task::yield_now().await;
                    }
                    Message::ByeBye => {
                        log::info!("ByeBye from worker[{id}]");
                        break;
                    }
                }

                tokio::task::yield_now().await;
            }

            CHIMES_THREAD_POOL_COUNTER
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .increase_exitlive();
        };

        let rt = GLOBAL_RUNTIME_EXECUTOR.get().unwrap();
        let t = rt.spawn(fut);
        // std::thread::spawn( move || rt.spawn(fut) );

        std::thread::yield_now();

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
    workers: Mutex<Vec<Worker>>,
    max_workers: usize,
    // max_buffer: usize,
    sender: Mutex<mpsc::SyncSender<Message>>,
    // counter: Mutex<Box<dyn TaskCounter + Send + Sync>>,
    jobs: Mutex<Vec<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>>,
}

impl Pool {
    fn print_backstack() {
        let bt = Backtrace::capture();
        log::error!("Backtrace:\n{bt:#?}");
    }

    /**
     * Attempt to lock the workers and re-init the works and sender/receiver
     */
    fn renew(&self) {
        if let Ok(mut worker_guard) = self.workers.try_lock() {
            let (tx, rx) = mpsc::sync_channel(100 * self.max_workers);

            let mut workers = Vec::with_capacity(self.max_workers);
            let receiver = Arc::new(Mutex::new(rx));
            for i in 0..self.max_workers {
                workers.push(Worker::new(i, Arc::clone(&receiver)));
            }
            *self.sender.lock().unwrap() = tx;
            *worker_guard = workers;
            log::warn!("Pool workers was renewed.");
        }
    }

    pub fn new(max_workers: usize) -> Pool {
        let act_workers = if max_workers == 0 {
            num_cpus::get()
        } else {
            max_workers
        };

        let (tx, rx) = mpsc::sync_channel(100 * act_workers);

        let mut workers = Vec::with_capacity(act_workers);
        let receiver = Arc::new(Mutex::new(rx));
        for i in 0..act_workers {
            workers.push(Worker::new(i, Arc::clone(&receiver)));
        }

        Pool {
            workers: Mutex::new(workers),
            max_workers: act_workers,
            sender: Mutex::new(tx),
            // counter: Mutex::new(ct),
            jobs: Mutex::new(Vec::new()),
        }
    }

    //pub fn setup_counter(&self, ct: Box<dyn TaskCounter + Send + Sync>) {
    //    *self.counter.lock().unwrap() = ct;
    //}

    /**
     * Submit a job and without wait for return
     */
    pub fn submit<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let job = Message::NewBackend(Box::pin(f), Box::new(tx));
        // self.counter.lock().unwrap().increase_task();
        CHIMES_THREAD_POOL_COUNTER
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .increase_task();
        if let Err(err) = self.sender.lock().unwrap().send(job) {
            log::error!("Submit a job faild {err:?}");
            Self::print_backstack();
            // self.counter.lock().unwrap().increase_error();
            CHIMES_THREAD_POOL_COUNTER
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .increase_error();
            self.renew();
            return;
        }
        match rx.recv_timeout(Duration::from_secs(6000)) {
            Ok(bx) => match bx.downcast::<i64>() {
                Ok(_rt) => {}
                Err(_) => {
                    log::error!("submit_sync downcast value error");
                    // self.counter.lock().unwrap().increase_error();
                    Self::print_backstack();
                    CHIMES_THREAD_POOL_COUNTER
                        .get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .increase_error();
                }
            },
            Err(err) => {
                log::error!("submit_sync wait for sync return with error {err}");
                // self.counter.lock().unwrap().increase_error();
                Self::print_backstack();
                CHIMES_THREAD_POOL_COUNTER
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .increase_error();
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
        // self.counter.lock().unwrap().increase_task();
        CHIMES_THREAD_POOL_COUNTER
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .increase_task();
        if let Err(err) = self.sender.lock().unwrap().send(job) {
            log::error!("Submit a submit_async faild {err:?}");
            Self::print_backstack();
            // self.counter.lock().unwrap().increase_error();
            CHIMES_THREAD_POOL_COUNTER
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .increase_error();
            self.renew();
            return;
        }

        match rx.recv_timeout(Duration::from_secs(6000)) {
            Ok(bx) => match bx.downcast::<i64>() {
                Ok(_rt) => {}
                Err(_) => {
                    log::warn!("submit_async downcat to return value");
                    Self::print_backstack();
                    //self.counter.lock().unwrap().increase_error();
                    CHIMES_THREAD_POOL_COUNTER
                        .get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .increase_error();
                }
            },
            Err(err) => {
                log::warn!("submit_async wait for return with error {err}");
                // self.counter.lock().unwrap().increase_error();
                Self::print_backstack();
                CHIMES_THREAD_POOL_COUNTER
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .increase_error();
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
        // self.counter.lock().unwrap().increase_task();
        CHIMES_THREAD_POOL_COUNTER
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .increase_task();
        if let Err(err) = self.sender.lock().unwrap().send(job) {
            log::error!("post a async blockon_process faild {err:?}");
            // self.counter.lock().unwrap().increase_error();
            Self::print_backstack();
            CHIMES_THREAD_POOL_COUNTER
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .increase_error();
            self.renew();
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
        // self.counter.lock().unwrap().increase_task();
        CHIMES_THREAD_POOL_COUNTER
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .increase_task();
        if let Err(err) = self.sender.lock().unwrap().send(job) {
            log::error!("post a async async_process faild {err:?}");
            // self.counter.lock().unwrap().increase_error();
            Self::print_backstack();
            CHIMES_THREAD_POOL_COUNTER
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .increase_error();
            self.renew();
        }
    }

    pub fn execute_blockon<F, T>(&self, f: F) -> Result<T, anyhow::Error>
    where
        F: Future<Output = JobOutput> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let job = Message::NewAsyncJob(Box::pin(f), Box::new(tx));
        // self.counter.lock().unwrap().increase_task();
        CHIMES_THREAD_POOL_COUNTER
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .increase_task();
        if let Err(err) = self.sender.lock().unwrap().send(job) {
            log::error!("send the job to backend thread with an error {err:?}");
            // self.counter.lock().unwrap().increase_error();
            Self::print_backstack();
            CHIMES_THREAD_POOL_COUNTER
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .increase_error();
            self.renew();
            return Err(anyhow!(err.to_string()));
        }
        // std::thread::yield_now();
        match rx.recv_timeout(Duration::from_secs(30000)) {
            Ok(bx) => match bx.downcast::<T>() {
                Ok(rt) => Ok(*rt),
                Err(_) => {
                    // self.counter.lock().unwrap().increase_error();
                    Self::print_backstack();
                    CHIMES_THREAD_POOL_COUNTER
                        .get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .increase_error();
                    Err(anyhow::anyhow!("Result cannot be cast"))
                }
            },
            Err(err) => {
                // self.counter.lock().unwrap().increase_error();
                Self::print_backstack();
                CHIMES_THREAD_POOL_COUNTER
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .increase_error();
                Err(anyhow::anyhow!("timeout {err}"))
            }
        }
    }

    pub fn execute_blockon_async<F, T>(&self, f: F) -> Result<T, anyhow::Error>
    where
        F: Future<Output = JobOutput> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let job = Message::AsyncBlockJob(Box::pin(f), Box::new(tx));
        // self.counter.lock().unwrap().increase_task();
        CHIMES_THREAD_POOL_COUNTER
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .increase_task();
        if let Err(err) = self.sender.lock().unwrap().send(job) {
            log::error!("send the job to backend thread with an error {err:?}");
            // self.counter.lock().unwrap().increase_error();
            Self::print_backstack();
            CHIMES_THREAD_POOL_COUNTER
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .increase_error();
            self.renew();
            return Err(anyhow!(err.to_string()));
        }

        match rx.recv_timeout(Duration::from_secs(30000)) {
            Ok(bx) => match bx.downcast::<T>() {
                Ok(rt) => Ok(*rt),
                Err(_) => {
                    // self.counter.lock().unwrap().increase_error();
                    Self::print_backstack();
                    CHIMES_THREAD_POOL_COUNTER
                        .get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .increase_error();
                    Err(anyhow::anyhow!("Result cannot be cast"))
                }
            },
            Err(err) => {
                // self.counter.lock().unwrap().increase_error();
                Self::print_backstack();
                CHIMES_THREAD_POOL_COUNTER
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .increase_error();
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
        // self.counter.lock().unwrap().increase_longlive();
        CHIMES_THREAD_POOL_COUNTER
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .increase_longlive();
        tokio::spawn(async {
            f.await;
            CHIMES_THREAD_POOL_COUNTER
                .get()
                .unwrap()
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
            // self.counter.lock().unwrap().increase_task();
            CHIMES_THREAD_POOL_COUNTER
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .increase_task();
            if let Err(err) = self.sender.lock().unwrap().send(job) {
                log::error!("post a async job faild {err:?}");
                Self::print_backstack();
                // self.counter.lock().unwrap().increase_error();
                CHIMES_THREAD_POOL_COUNTER
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .increase_error();
                self.renew();
            }
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        for _ in 0..self.max_workers {
            if let Err(err) = self.sender.lock().unwrap().send(Message::ByeBye) {
                log::debug!("--send error {err}");
            }
        }
        for w in self.workers.lock().unwrap().iter_mut() {
            if let Some(t) = w.t.take() {
                // t.join().unwrap();
                t.abort();
            }
        }
    }
}

pub fn init_async_task() {
    init_async_task_pool(Box::new(MockTaskCounter()));
}

pub fn init_async_task_pool(ct: Box<dyn TaskCounter + Send + Sync>) {
    log::info!(
        "Init the task workthread: {}, pool size: {}",
        GlobalConfig::get_worker_threads(),
        GlobalConfig::get_pool_size()
    );

    let pool_size = GlobalConfig::get_pool_size();

    CHIMES_THREAD_POOL_COUNTER.get_or_init(|| Mutex::new(ct));

    // let _ = CHIMES_RUNTIME_EXECUTOR.get_or_init(|| Builder::new_multi_thread()
    //                                                         .enable_all()
    //                                                         .worker_threads(GlobalConfig::get_worker_threads())
    //                                                         .build()
    //                                                         .expect("could net create a new runtime"));

    let _ = CHIMES_THREAD_POOL.get_or_init(|| Pool::new(pool_size / 2));
    let _ = CHIMES_THREAD_POOL_BLOCK.get_or_init(|| Pool::new(pool_size / 2));
}

pub fn init_global_runtime(rt: Runtime) -> &'static Runtime {
    GLOBAL_RUNTIME_EXECUTOR.get_or_init(|| rt)
}

pub fn independ_block_on<F: Future>(future: F) -> Result<F::Output, anyhow::Error> {
    if let Some(rt1) = GLOBAL_RUNTIME_EXECUTOR.get() {
        Ok(tokio::task::block_in_place(move || rt1.block_on(future)))
        // Ok(blo(rt1.wrap(future)))
    } else {
        Err(anyhow!("Could not get the globale executor"))
    }
}

pub fn queued_block_on<F: Future>(future: F) -> Result<F::Output, anyhow::Error> {
    if let Some(rt1) = GLOBAL_RUNTIME_EXECUTOR.get() {
        Ok(tokio::task::block_in_place(move || rt1.block_on(future)))
        // Ok(blo(rt1.wrap(future)))
    } else {
        Err(anyhow!("Could not get the globale executor"))
    }
}
