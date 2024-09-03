use super::invoker::InvocationContext;
use super::sdk::InvokeUri;
use super::starter::MxStoreService;
use super::{registry::SchemaRegistry, sdk::Invocation};
use anyhow::{anyhow, Error};
use futures_lite::Future;
use rbatis::Page;
use serde_json::Value;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

// # Safety: call the schema registry by outside dylib
pub fn get_schema_registry() -> &'static mut SchemaRegistry {
    SchemaRegistry::get_mut()
}

// #[cfg(feature = "plugin")]
// #[link(name="store-server", kind="dylib")]
// extern "C" {}

pub struct PluginServiceInvocation(pub(crate) String);

impl PluginServiceInvocation {
    pub fn new(schema: &str) -> Self {
        Self(schema.to_owned())
    }

    pub fn get_protocol(&self) -> String {
        self.0.clone()
    }
}

impl Invocation for PluginServiceInvocation {
    fn invoke_return_option(
        &'static self,
        uri: &'_ InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &'_ [Value],
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, Error>> + Send>> {
        match MxStoreService::get_plugin_service(&uri.url_no_method()) {
            Some(nss) => nss.invoke_return_option(uri.clone(), ctx, args.to_vec()),
            None => Box::pin(async { Err(anyhow!("Not implemented")) }),
        }
    }

    fn invoke_return_vec(
        &'static self,
        uri: &InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>> {
        match MxStoreService::get_plugin_service(&uri.url_no_method()) {
            Some(nss) => nss.invoke_return_vec(uri.clone(), ctx, args.to_vec()),
            None => Box::pin(async { Err(anyhow!("Not implemented")) }),
        }
    }

    fn invoke_return_page(
        &'static self,
        uri: &InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, Error>> + Send>> {
        match MxStoreService::get_plugin_service(&uri.url_no_method()) {
            Some(nss) => nss.invoke_return_page(uri.clone(), ctx, args.to_vec()),
            None => Box::pin(async { Err(anyhow!("Not implemented")) }),
        }
    }
}
