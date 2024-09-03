use std::{any::Any, str::FromStr, time::Duration};

use anyhow::anyhow;
use chimes_store_core::pin_blockon_async;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method,
};
use rhai::{CustomType, Dynamic, EvalAltResult, Position, TypeBuilder};
use serde_json::Value;

#[derive(Clone, CustomType)]
pub struct RhaiHttpClient {
    uri: String,
}

impl RhaiHttpClient {
    pub fn build_http_client(option: &Option<Value>) -> Result<Client, anyhow::Error> {
        let mut builder = Client::builder();
        if option.is_some() {
            if let Some(map) = option.clone().unwrap().as_object() {
                for (key, val) in map {
                    match key.as_str().to_lowercase().as_str() {
                        "keepalive" => {
                            builder = builder.tcp_keepalive(Some(Duration::from_millis(
                                val.as_i64().unwrap_or(30000) as u64,
                            )));
                        }
                        "tcp_nodealy" => {
                            builder = builder.tcp_nodelay(val.as_bool().unwrap_or(false));
                        }
                        "user_agent" => {
                            if val.is_string() {
                                builder = builder.user_agent(val.as_str().unwrap());
                            }
                        }
                        "header" => {
                            // setup the default header
                            if let Some(headers) = val.as_object() {
                                let mut hm = HeaderMap::new();
                                for (hk, hval) in headers {
                                    if let Ok(hdname) = HeaderName::from_str(hk.as_str()) {
                                        let header_val = hval.to_string();
                                        if let Ok(hdv) = HeaderValue::from_str(&header_val) {
                                            hm.append(hdname, hdv);
                                        }
                                    }
                                }

                                if !hm.is_empty() {
                                    builder = builder.default_headers(hm);
                                }
                            }
                        }
                        "timeout" => {
                            builder = builder.connect_timeout(Duration::from_millis(
                                val.as_i64().unwrap_or(10000) as u64,
                            ));
                            builder = builder.timeout(Duration::from_millis(
                                val.as_i64().unwrap_or(10000) as u64,
                            ));
                        },
                        "accept_invalid_certs" => {
                            let accept = val.as_bool().unwrap_or_default();
                            builder = builder.danger_accept_invalid_certs(accept)
                                             .danger_accept_invalid_hostnames(accept);
                        },
                        _ => {}
                    }
                }
            }
        }
        builder.build().map_err(|err| anyhow!(err))
    }

    // execute a http request
    pub async fn send_http_request(
        url: &str,
        method: reqwest::Method,
        data: Value,
        option: Option<Value>,
    ) -> Result<Option<Value>, anyhow::Error> {
        let client = Self::build_http_client(&option)?;
        let mut builder = client.request(method.clone(), url);
        if method == Method::POST || method == Method::PUT {
            builder = builder.json(&data);
        } else if !data.is_null() {
            builder = builder.query(&data);
        }

        builder
            .send()
            .await
            .unwrap()
            .json::<Option<Value>>()
            .await
            .map_err(|err| anyhow!(err))
    }

    pub fn sync_http_request(
        url: &str,
        method: reqwest::Method,
        data: Value,
        option: Option<Value>,
    ) -> Result<Option<Value>, Box<EvalAltResult>> {
        let url_text = url.to_owned();
        pin_blockon_async!(async move {
            let ret = match Self::send_http_request(&url_text, method, data, option).await {
                Ok(ret) => Ok(ret),
                Err(err) => Err(Box::new(EvalAltResult::ErrorRuntime(
                    Dynamic::from_str(&err.to_string()).unwrap(),
                    Position::new(1, 1),
                ))),
            };
            Box::new(ret) as Box<dyn Any + Send + Sync>
        })
        .unwrap_or(Ok(None))
    }
}
