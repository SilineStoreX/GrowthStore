use anyhow::anyhow;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE},
    Client, Method,
};
use serde_json::Value;
use std::{str::FromStr, time::Duration};

#[derive(Clone)]
pub struct RestHttpClient {
    _uri: String,
}

impl RestHttpClient {
    pub fn build_http_client(option: &Option<Value>) -> Result<Client, anyhow::Error> {
        let mut builder = Client::builder();
        if let Some(opt) = option {
            if let Some(map) = opt.as_object() {
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
                        "cookie" => {
                            if let Some(cookie) = val.as_str() {
                                let mut hm = HeaderMap::new();
                                if let Ok(hv) = HeaderValue::from_str(cookie) {
                                    hm.insert("Cookie", hv);
                                }
                                if !hm.is_empty() {
                                    builder = builder.default_headers(hm);
                                }
                            }
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

                                if let Some(ckv) = map.get("cookie") {
                                    if let Some(ckvstr) = ckv.as_str() {
                                        if let Ok(hv) = HeaderValue::from_str(ckvstr) {
                                            hm.insert("Cookie", hv);
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
                            log::info!("accept_invalid_certs {accept}");
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
        data: &str,
        fmt: &str,
        option: &Option<Value>,
    ) -> Result<Option<Value>, anyhow::Error> {
        let client = Self::build_http_client(option)?;
        let mut builder = client.request(method.clone(), url);
        log::info!("send request to {} {}", method.clone(), url);
        log::info!("data: {fmt} === {data}");
        if method == Method::POST || method == Method::PUT {
            if fmt == "xml" {
                builder = builder.body(data.to_string());
                builder = builder.header(CONTENT_TYPE, HeaderValue::from_static("text/xml"));
            } else if fmt == "json" {
                let obj = serde_json::from_str::<Value>(data).unwrap_or(Value::Null);
                builder = builder.json(&obj);
            } else {
                let obj = serde_json::from_str::<Value>(data).unwrap_or(Value::Null);
                builder = builder.form(&obj);
            }
        } else {
            let obj = serde_json::from_str::<Value>(data).unwrap_or(Value::Null);
            builder = builder.query(&obj);
        }

        builder
            .send()
            .await
            .unwrap()
            .json::<Option<Value>>()
            .await
            .map_err(|err| anyhow!(err))
    }
}
