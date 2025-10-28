use std::{future::Future, pin::Pin, sync::{Arc, Mutex, OnceLock}};

use anyhow::anyhow;
use chimes_store_core::{
    config::PluginConfig, dbs::{init_growth_invoke_holder, GrowthInvokeHolder}, service::{invoker::InvocationContext, plugin::get_schema_registry, sched::SchedulerHolder, starter::MxStoreService}
};
use engine::python_plugin::PythonPluginService;
use salvo::Router;
use libloading::Library;
use serde_json::Value;

use crate::job::PythonJobInvoker;

mod job;
mod engine;
mod api;

type FnPythonExecute = unsafe fn(
    ctx: Arc<Mutex<InvocationContext>>,
    script: &str,
    args: Vec<Value>,
    options: Option<Value>,
) -> Result<Option<Value>, anyhow::Error>;

type FnSetupGrowthInvokeHolder = unsafe fn(growth: GrowthInvokeHolder);

#[derive(Clone)]
pub struct PythonExtensionRountine {
    pub fn_python_execute: Option<FnPythonExecute>,
    pub fn_growth_holder_setup: Option<FnSetupGrowthInvokeHolder>
}




#[allow(dead_code)]
pub struct PythonExtionRegistry {
    lib: Mutex<Option<Library>>,
    registry: Mutex<PythonExtensionRountine>,
}

impl PythonExtionRegistry {
    pub fn get() -> &'static PythonExtionRegistry {
        // 使用MaybeUninit延迟初始化
        static PYTHON_EXT_REGISTRY: OnceLock<PythonExtionRegistry> = OnceLock::new();

        PYTHON_EXT_REGISTRY.get_or_init(|| PythonExtionRegistry {
            registry: Mutex::new(PythonExtensionRountine {
                fn_python_execute: None,
                fn_growth_holder_setup: None
            }),
            lib: Mutex::new(None),
        })
    }

    pub fn load(&self) -> Result<(), anyhow::Error> {
        let not_initialized = self.lib.lock().unwrap().is_none();
        if not_initialized {
            unsafe {
                let lib = match libloading::Library::new("plugin_python_executor") {
                    Ok(l) => l,
                    Err(err) => {
                        return Err(anyhow!(err));
                    }
                };

                let fn_setup: Option<FnSetupGrowthInvokeHolder> = match lib.get(b"setup_growth_invoke_holder") {
                    Ok(s) => Some(*s),
                    Err(_err) => None,
                };
                
                let fn_exec: Option<FnPythonExecute> = match lib.get(b"execute") {
                    Ok(s) => Some(*s),
                    Err(_err) => None,
                };
                
                *self.registry.lock().unwrap() = PythonExtensionRountine {
                    fn_growth_holder_setup: fn_setup,
                    fn_python_execute: fn_exec
                };

                if let Some(fn_setup_holder) = fn_setup {
                    fn_setup_holder(init_growth_invoke_holder());
                }

                self.lib.lock().unwrap().replace(lib);
            }
        }
        Ok(())
    }

    pub fn eval(&self, script: &str, ctx: Arc<Mutex<InvocationContext>>, args: Vec<Value>, conf: Option<Value>) -> Result<Option<Value>, anyhow::Error> {
        if let Some(python_execute) = self.registry.lock().unwrap().fn_python_execute {
            unsafe { python_execute(ctx, script, args, conf) }
        } else {
            Ok(None)
        }
    }
}


/**
 * Python是类似于Compose的插件
 * 可以使用该插件调用指定的位置（环境）Python程序
 * 在该Python程序中，可以使用GrowthStore的Invoke执行机制，以及相应的配置
 */
pub fn get_plugin_name() -> &'static str {
    "python"
}

//#[no_mangle]
pub fn plugin_router_register() -> Vec<Router> {
    vec![
        Router::with_path("/python/{ns}/{name}/{method}/schema").get(api::get_schema),
        Router::with_path("/python/{ns}/{name}/{method}/single").get(api::execute_single_request),
        Router::with_path("/python/{ns}/{name}/{method}/single").post(api::execute_single_request),
        Router::with_path("/python/{ns}/{name}/{method}/single").put(api::execute_single_request),
        Router::with_path("/python/{ns}/{name}/{method}/list").get(api::execute_vec_request),
        Router::with_path("/python/{ns}/{name}/{method}/list").post(api::execute_vec_request),
        Router::with_path("/python/{ns}/{name}/{method}/list").put(api::execute_vec_request),
        Router::with_path("/python/{ns}/{name}/{method}/page").get(api::execute_paged_request),
        Router::with_path("/python/{ns}/{name}/{method}/page").post(api::execute_paged_request),
        Router::with_path("/python/{ns}/{name}/{method}/upload").post(api::execute_upload_request),
        Router::with_path("/python/{ns}/{name}/{method}/page").put(api::execute_paged_request),
    ]
}

//#[no_mangle]
pub fn plugin_anonymous_router_register() -> Vec<Router> {
    vec![
        Router::with_path("/python/{ns}/{name}/{method}/schema").get(api::get_schema),
        Router::with_path("/python/{ns}/{name}/{method}/single").get(api::execute_single_request),
        Router::with_path("/python/{ns}/{name}/{method}/single").post(api::execute_single_request),
        Router::with_path("/python/{ns}/{name}/{method}/single").put(api::execute_single_request),
        Router::with_path("/python/{ns}/{name}/{method}/list").get(api::execute_vec_request),
        Router::with_path("/python/{ns}/{name}/{method}/list").post(api::execute_vec_request),
        Router::with_path("/python/{ns}/{name}/{method}/list").put(api::execute_vec_request),
        Router::with_path("/python/{ns}/{name}/{method}/page").get(api::execute_paged_request),
        Router::with_path("/python/{ns}/{name}/{method}/page").post(api::execute_paged_request),
        Router::with_path("/python/{ns}/{name}/{method}/upload").post(api::execute_upload_request),
        Router::with_path("/python/{ns}/{name}/{method}/page").put(api::execute_paged_request),
    ]
}



/**
 * 初始化插件
 */
pub fn plugin_init(ns: &str, conf: &PluginConfig) -> Pin<Box<dyn Future<Output = ()> + Send>> {

if let Err(err) = PythonExtionRegistry::get().load() {
    log::error!("could not load python extention. {err:?}");
    return Box::pin(async{});
}

match PythonPluginService::new(ns, conf) {
        Ok(wplc) => {
            log::info!(
                "Process the  config of Python plugin and init the plugin for {}.",
                conf.name
            );

            let services = wplc.get_services();
            let name_ = conf.name.clone();
            let namespace_ = ns.to_string();
            let nsuri = format!("{}://{}/{}", conf.protocol, ns, conf.name);
            // let services = wplc.get_services();
            MxStoreService::register_plugin(&nsuri, Box::new(wplc));
            get_schema_registry().register_plugin_invocation("python");
            Box::pin(async move {
                for cs in services {
                    let jobid = format!("job-{}#{}", &nsuri, cs.name);
                    SchedulerHolder::get().removejob(&jobid);                    
                    if cs.schedule_on {
                        let cronexpress = cs.cron_express.clone().unwrap_or_default();
                        let duration_sec = cs.interval_second.map(|s| Some(s as u64)).unwrap_or(None);
                        SchedulerHolder::get().addjob(&jobid, &cronexpress, duration_sec, Box::new(PythonJobInvoker::new(&jobid, &namespace_, &name_, &cs)));
                    }
                }
            })
        }
        Err(err) => {
            log::warn!(
                "Plugin python plugin was not be apply to {ns}. The config of this plugin was not be parsed. The error is {err:?}"
            );
            Box::pin(async move {})
        }
    }
}

/**
 * Init the current extension
 * create the eval context of this lang script engine
 */
pub fn extension_init() {

}
