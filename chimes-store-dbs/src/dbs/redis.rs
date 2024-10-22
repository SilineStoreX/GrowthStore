use anyhow::anyhow;
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::sdk::Invocation;
use chimes_store_core::service::sdk::InvokeUri;
use chimes_store_core::utils::redis::redis_del;
use chimes_store_core::utils::redis::redis_get;
use chimes_store_core::utils::redis::redis_set;
use futures_lite::Future;
use serde_json::Value;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

pub struct RedisInvocation();

/**
 * TODO: 实现Redis的请求命令
 * 目前只实现了GET/SET/DEL三个命令，后续继续实现
 */

impl Invocation for RedisInvocation {
    fn invoke_return_option(
        &'static self,
        uri: &InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, anyhow::Error>> + Send>> {
        let res = match uri.method.as_str() {
            "get" => redis_get(&uri.namespace, &uri.query.clone().unwrap_or_default()),
            "set" => {
                let text = args[0].as_str();
                redis_set(
                    &uri.namespace,
                    &uri.query.clone().unwrap_or_default(),
                    text.unwrap_or_default(),
                )
            }
            "del" => redis_del(&uri.namespace, &uri.query.clone().unwrap_or_default()),
            _ => Err(anyhow!("Not implemented")),
        };

        Box::pin(async move {
            if let Some(text) = res? {
                match serde_json::from_str::<Value>(&text) {
                    Ok(t) => Ok(Some(t)),
                    Err(_) => Ok(Some(Value::String(text))),
                }
            } else {
                Ok(None)
            }
        })
    }

    fn invoke_return_vec(
        &'static self,
        uri: &InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, anyhow::Error>> + Send>> {
        let res = match uri.method.as_str() {
            "get" => redis_get(&uri.namespace, &uri.query.clone().unwrap_or_default()),
            "set" => {
                let text = args[0].as_str();
                redis_set(
                    &uri.namespace,
                    &uri.query.clone().unwrap_or_default(),
                    text.unwrap_or_default(),
                )
            }
            "del" => redis_del(&uri.namespace, &uri.query.clone().unwrap_or_default()),
            _ => Err(anyhow!("Not implemented")),
        };

        Box::pin(async move {
            if let Some(text) = res? {
                match serde_json::from_str::<Vec<Value>>(&text) {
                    Ok(t) => Ok(t),
                    Err(err) => Err(anyhow!(err)),
                }
            } else {
                Ok(vec![])
            }
        })
    }

    fn invoke_return_page(
        &'static self,
        uri: &InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<rbatis::Page<Value>, anyhow::Error>> + Send>> {
        let res = match uri.method.as_str() {
            "get" => redis_get(&uri.namespace, &uri.query.clone().unwrap_or_default()),
            "set" => {
                let text = args[0].as_str();
                redis_set(
                    &uri.namespace,
                    &uri.query.clone().unwrap_or_default(),
                    text.unwrap_or_default(),
                )
            }
            "del" => redis_del(&uri.namespace, &uri.query.clone().unwrap_or_default()),
            _ => Err(anyhow!("Not implemented")),
        };

        Box::pin(async move {
            if let Some(text) = res? {
                match serde_json::from_str::<rbatis::Page<Value>>(&text) {
                    Ok(t) => Ok(t),
                    Err(err) => Err(anyhow!(err)),
                }
            } else {
                Ok(rbatis::Page::new_total(0, 0, 0))
            }
        })
    }
}
