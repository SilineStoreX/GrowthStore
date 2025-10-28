use std::sync::{Arc, Mutex, OnceLock};
use anyhow::{anyhow};
use rbatis::Page;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::{pin_blockon_async_v2, service::{invoker::{release_all_connections, InvocationContext}, registry::SchemaRegistry, starter::MxStoreService}};

pub mod probe;

static REF_GROWTH_ENV_PATH: OnceLock<Mutex<Option<String>>> = OnceLock::new();

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PythonEnvConfig {
    pub python_path: Option<String>,
    pub site_package: Option<String>,
    pub workspace: Option<String>,
    pub return_item: Option<String>,
}


pub type FnGetNamespaceConfig = unsafe extern "Rust" fn(
    uri: &str,
) -> Result<Value, anyhow::Error>;

pub type FnGetPluginConfig = unsafe extern "Rust" fn(
    uri: &str,
) -> Result<Value, anyhow::Error>;

pub type FnInvokeReturnList = unsafe extern "Rust" fn(
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Vec<Value>, anyhow::Error>;

pub type FnDirectInvoke = unsafe extern "Rust" fn(
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    query: &str,
    args: &[Value],
) -> Result<Vec<Value>, anyhow::Error>;

pub type FnDirectInvokeV2 = unsafe extern "Rust" fn(
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    query: Value,
    args: &[Value],
) -> Result<Vec<Value>, anyhow::Error>;

pub type FnInvokeReturnOption = unsafe extern "Rust" fn(
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Option<Value>, anyhow::Error>;

pub type FnInvokeReturnPaged = unsafe extern "Rust" fn(
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Page<Value>, anyhow::Error>;

pub type FnReleaseConnection = unsafe extern "Rust" fn(ctx: Arc<Mutex<InvocationContext>>);


fn update_ref_sys_path() {
    let refpath = REF_GROWTH_ENV_PATH.get_or_init(|| Mutex::new(None));
    if let Ok(path) = std::env::var("PATH") {
        refpath.lock().unwrap().replace(path);
    }
}

pub fn get_sys_path() -> Option<String> {
    REF_GROWTH_ENV_PATH.get_or_init(|| Mutex::new(None)).lock().unwrap().clone()
}


#[derive(Clone, Default, Debug)]
pub struct GrowthInvokeHolder {
  invoke_list: Option<FnInvokeReturnList>,
  invoke_option: Option<FnInvokeReturnOption>,
  invoke_paged: Option<FnInvokeReturnPaged>,
  direct_invoke: Option<FnDirectInvoke>,
  direct_invoke_v2: Option<FnDirectInvokeV2>,
  fn_get_namespace_config:  Option<FnGetNamespaceConfig>,
  fn_get_plugin_config: Option<FnGetPluginConfig>,
  fn_releaseconnection: Option<FnReleaseConnection>,
}


impl GrowthInvokeHolder {

  pub fn direct_invoke(
    &self,
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    query: &str,
    args: &[Value],
  ) -> Result<Vec<Value>, anyhow::Error> {
    if let Some(invk) = self.direct_invoke {
      unsafe { invk(uri, ctx, query, args) }
    } else {
      Err(anyhow!("Not set direct_invoke"))
    }
  }

  pub fn direct_invoke_v2(
    &self,
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    query: Value,
    args: &[Value],
  ) -> Result<Vec<Value>, anyhow::Error> {
    if let Some(invk) = self.direct_invoke_v2 {
      unsafe { invk(uri, ctx, query, args) }
    } else {
      Err(anyhow!("Not set direct_invoke_v2"))
    }
  }

  pub fn get_namespace_config(&self, ns: &str) -> Result<Value, anyhow::Error> {
    if let Some(invk) = self.fn_get_namespace_config {
      unsafe { invk(ns) }
    } else {
      Err(anyhow!("Not set get_namespace_config"))
    }
  }

  pub fn get_plugin_config(&self, uri: &str) -> Result<Value, anyhow::Error> {
    if let Some(invk) = self.fn_get_plugin_config {
      unsafe { invk(uri) }
    } else {
      Err(anyhow!("Not set get_plugin_config"))
    }
  }  


  pub fn invoke_return_paged(
    &self,
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
  ) -> Result<Page<Value>, anyhow::Error> {
    if let Some(invk) = self.invoke_paged {
      unsafe { invk(uri, ctx, args) }
    } else {
      Err(anyhow!("Not set invoke_paged"))
    }
  }


  pub fn invoke_return_vec(
    &self,
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
  ) -> Result<Vec<Value>, anyhow::Error> {
    if let Some(invk) = self.invoke_list {
      unsafe { invk(uri, ctx, args) }
    } else {
      Err(anyhow!("Not set invoke_list"))
    }
  }

  pub fn invoke_return_option(
    &self,
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
  ) -> Result<Option<Value>, anyhow::Error> {
    if let Some(invk) = self.invoke_option {
      unsafe { invk(uri, ctx, args) }
    } else {
      Err(anyhow!("Not set invoke_option"))
    }
  }

  pub fn release_connections(&self, ctx: Arc<Mutex<InvocationContext>>) {
    if let Some(fn_release) = self.fn_releaseconnection {
      unsafe { fn_release(ctx) }
    }
  }
}


fn proxy_invoke_return_vec(
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Vec<Value>, anyhow::Error> {
  let str_uri = uri.to_string();
  let vt_args = args.to_vec();
  pin_blockon_async_v2!(async move {
    SchemaRegistry::get().invoke_return_vec(&str_uri, ctx, &vt_args).await
  }).unwrap_or(Err(anyhow!("Error to unwrap")))
}

fn proxy_invoke_return_option(
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Option<Value>, anyhow::Error> {
  let str_uri = uri.to_string();
  let vt_args = args.to_vec();
  pin_blockon_async_v2!(async move {
    SchemaRegistry::get().invoke_return_option(&str_uri, ctx, &vt_args).await
  }).unwrap_or(Err(anyhow!("Error to unwrap")))
}

fn proxy_invoke_return_page(
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Page<Value>, anyhow::Error> {
  let str_uri = uri.to_string();
  let vt_args = args.to_vec();
  pin_blockon_async_v2!(async move {
    SchemaRegistry::get().invoke_return_page(&str_uri, ctx, &vt_args).await
  }).unwrap_or(Err(anyhow!("Error to unwrap")))
}

fn proxy_direct_invoke(
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    query: &str,
    args: &[Value],
) -> Result<Vec<Value>, anyhow::Error> {
  // let str_uri = uri.to_string();
  // let vt_args = args.to_vec();
  pin_blockon_async_v2!(async move {
    SchemaRegistry::get().invoke_direct_query(uri, ctx, query, args).await
  }).unwrap_or(Err(anyhow!("Error to unwrap")))
}


fn proxy_direct_invoke_v2(
    uri: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    query: Value,
    args: &[Value],
) -> Result<Vec<Value>, anyhow::Error> {
  pin_blockon_async_v2!(async move {
    SchemaRegistry::get().invoke_direct_query_v2(uri, ctx, query, args).await
  }).unwrap_or(Err(anyhow!("Error to unwrap")))
}


pub fn proxy_get_namesapce_config(ns: &str) -> Result<Value, anyhow::Error> {
  let ret = MxStoreService::get(ns).map(|f| {
    let conf = f.get_config();
    json!({
      "filename": conf.filename,
      "aes_solt": conf.aes_solt,
      "aes_key": conf.aes_key,
      "rsa_public_key": conf.rsa_public_key,
      "rsa_private_key": conf.rsa_private_key,
      "datetime_format": conf.datetime_format,
      "dict_uri": conf.dict_uri,
      "range_uri": conf.range_uri,
      "relaxy_timezone": conf.relaxy_timezone,
      "max_filesize": conf.max_filesize,
      "upload_filepath": conf.upload_filepath,
      "subfolder_bydate": conf.subfolder_bydate,
      "subfolder_bytype": conf.subfolder_bytype,
      "download_prefix": conf.download_prefix,
      "download_direct": conf.download_direct,
      "download_query": conf.download_query,
      "download_file_name": conf.download_file_name,
      "download_file_path": conf.download_file_path,
      "upload_to_oss": conf.upload_to_oss,
      "oss_endpoint": conf.oss_endpoint,
      "oss_bulk_id": conf.oss_bulk_id,
      "oss_auth": conf.oss_auth,

      "namespace": conf.namespace,
    })
  }).unwrap_or(Value::Null);
  Ok(ret)
}


pub fn proxy_get_plugin_config(ns: &str) -> Result<Value, anyhow::Error> {
  let ret = MxStoreService::get_plugin_service(ns).map(|ps| ps.get_config().unwrap_or(Value::Null)).unwrap_or(Value::Null);
  Ok(ret)
}

fn proxy_release_connection(ctx: Arc<Mutex<InvocationContext>>) {
  let _ = pin_blockon_async_v2!(async move {
    release_all_connections(&ctx).await;
  }).is_ok();
}


static REF_GROWTH_INVOKER_HOLDER: OnceLock<Mutex<GrowthInvokeHolder>> = OnceLock::new();

pub fn init_growth_invoke_holder() -> GrowthInvokeHolder {
  GrowthInvokeHolder {
    invoke_list: Some(proxy_invoke_return_vec),
    invoke_option: Some(proxy_invoke_return_option),
    invoke_paged: Some(proxy_invoke_return_page),
    direct_invoke: Some(proxy_direct_invoke),
    direct_invoke_v2: Some(proxy_direct_invoke_v2),
    fn_get_plugin_config: Some(proxy_get_plugin_config),
    fn_get_namespace_config: Some(proxy_get_namesapce_config),
    fn_releaseconnection: Some(proxy_release_connection),
  }
}

#[no_mangle]
pub fn setup_growth_invoke_holder(growth: GrowthInvokeHolder) {
  // if let Ok(rt) = tokio::runtime::Builder::new_multi_thread().enable_all().build() {
  //  init_global_runtime(rt);
  // }

  update_ref_sys_path();

  let t = REF_GROWTH_INVOKER_HOLDER.get_or_init(|| {
    Mutex::new(GrowthInvokeHolder { 
        invoke_list: None, 
        invoke_option: None, 
        invoke_paged: None,
        direct_invoke: None,
        direct_invoke_v2: None,
        fn_get_namespace_config: None,
        fn_get_plugin_config: None, 
        fn_releaseconnection: None,
    })
  });

  *t.lock().unwrap() = growth;
}

pub fn get_growth_invoke_holder() -> GrowthInvokeHolder {
  let t = REF_GROWTH_INVOKER_HOLDER.get_or_init(|| {
    Mutex::new(GrowthInvokeHolder { 
        invoke_list: None, 
        invoke_option: None, 
        invoke_paged: None,
        direct_invoke: None,
        direct_invoke_v2: None,
        fn_get_namespace_config: None,
        fn_get_plugin_config: None,
        fn_releaseconnection: None
    })
  });
  
  t.lock().unwrap().clone()
}
