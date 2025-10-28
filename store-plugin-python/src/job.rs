use std::{future::Future, pin::Pin, sync::{Arc, Mutex}};

use chimes_store_core::{config::auth::JwtUserClaims, service::{invoker::InvocationContext, sched::JobInvoker, starter::MxStoreService}};
use serde_json::Value;

use crate::engine::python_plugin::PythonServiceInfo;


pub(crate) struct PythonJobInvoker {
    uuid: String,
    uri: String,
    simulator: String,
    desc: String,
}

impl PythonJobInvoker {
    pub fn new(uuid: &str, ns: &str, name: &str, pyth: &PythonServiceInfo) -> Self {
      Self { 
        uuid: uuid.to_string(), 
        uri: format!("python://{ns}/{name}#{}", pyth.name.clone()), 
        simulator: pyth.schedule_simulate.clone().unwrap_or_default(), 
        desc: pyth.desc.clone().unwrap_or_default(), 
      }
    }
}

impl JobInvoker for PythonJobInvoker {
    fn exec(&self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send + 'static>> {
        let cs_simulate = self.simulator.clone();
        let inv_full_uri = self.uri.clone();
        let uuid = self.uuid.clone();
        log::info!("Uuid {} to be scheduled.", self.uuid);
        Box::pin(async move {
            // CronSchedulerHolder::update_begin(&uuid).await;
            log::info!("trigger to invoke the time PythonJobInvoker .....");
            let ctx = Arc::new(Mutex::new(InvocationContext::new_userclaims(
                JwtUserClaims::username(&cs_simulate),
            )));
            match MxStoreService::invoke_return_one(inv_full_uri.clone(), ctx, vec![]).await {
                Ok(_) => {
                    log::info!("Compose Job {uuid} executed successfully.");
                    // CronSchedulerHolder::update_completed(&uuid, "SUCCESS", None).await;
                    Ok(())
                }
                Err(err) => {
                    //log::error!("schedule execute on error {:?}", err);
                    log::error!("Compose Job {uuid} execute failed by error {err:?}");
                    //CronSchedulerHolder::update_completed(&uuid, "FAILED", Some(err.to_string()))
                    //    .await;
                    Err(err)
                }
            }
        })
    }

    fn get_invoke_uri(&self) -> String {
        self.uri.clone()
    }

    fn get_invoke_params(&self) -> Option<Value> {
        None
    }

    fn get_description(&self) -> String {
        self.desc.clone()
    }
}
