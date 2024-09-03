use std::{any::Any, future::Future, mem::MaybeUninit, pin::Pin, sync::{Mutex, Once}};
use uuid::Uuid;

use crate::pin_blockon_async;

pub trait JobInvoker {
    fn exec(&self) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
}

pub trait SchedulerManager: Send + Sync {
    fn add_job(&self, job_id: &str, cron: &str, run: Box<dyn JobInvoker + Send + Sync>) -> Pin<Box<dyn Future<Output = Result<Uuid, anyhow::Error>> + Send + 'static>>;
    fn remove_job(&self, job_id: &str) -> Pin<Box<dyn Future<Output = Result<Uuid, anyhow::Error>> + Send + 'static>>;
    fn start(&self);
}


pub struct SchedulerHolder {
    scheduler: Mutex<Option<&'static dyn SchedulerManager>>,
}

impl SchedulerHolder {
    
    pub fn get_() -> &'static mut SchedulerHolder {
        static mut SCHEDULER_CTX_HOLDER: MaybeUninit<SchedulerHolder> =
            MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static SCHEDULER_CTX_HOLDER_ONCE: Once = Once::new();

        SCHEDULER_CTX_HOLDER_ONCE.call_once(|| unsafe {
            SCHEDULER_CTX_HOLDER.as_mut_ptr().write(SchedulerHolder {
                scheduler: Mutex::new(None)
            });
        });

        unsafe { &mut (*SCHEDULER_CTX_HOLDER.as_mut_ptr()) }
    }

    pub fn update(sch: &'static dyn SchedulerManager) {
        Self::get_().scheduler.lock().unwrap().replace(sch);
    }


    pub fn addjob(&self, jobid: &str, cronexpress: &str, invoker: Box<dyn JobInvoker + Send + Sync>) {
        if let Some(ts) = *Self::get_().scheduler.lock().unwrap() {
            let job_id = jobid.to_owned();
            let cronxpr = cronexpress.to_owned();
            pin_blockon_async!(async move {
                if let Err(err) = ts.add_job(&job_id, &cronxpr, invoker).await {
                    log::info!("error on add job {err}  for {job_id}.");
                }
                Box::new(0) as Box<dyn Any + Send + Sync>
            }).unwrap_or(0);
        }
    }

    pub fn removejob(&self, jobid: &str) {
        if let Some(ts) = *Self::get_().scheduler.lock().unwrap() {
            let job_id = jobid.to_owned();
            pin_blockon_async!(async move {
                if let Err(err) = ts.remove_job(&job_id).await {
                    log::info!("error on remove job {err} for {job_id}.");
                }
                Box::new(0) as Box<dyn Any + Send + Sync>
            }).unwrap_or(0);
        }
    }

    pub fn start(&self) {
        if let Some(ts) = *Self::get_().scheduler.lock().unwrap() {
            ts.start();
        }
    }

}
