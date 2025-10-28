use anyhow::anyhow;
use chimes_store_utils::algorithm::base64_encode;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE},
    Client, Method,
};
use reqwest_eventsource::EventSource;
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
                                        let header_val = match hval {
                                            Value::String(strval) => strval.to_owned(),
                                            _ => hval.to_string(),
                                        };

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

                                log::warn!("Header {hm:?}");

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
                        }
                        "accept_invalid_certs" => {
                            let accept = val.as_bool().unwrap_or_default();
                            log::info!("accept_invalid_certs {accept}");
                            builder = builder
                                .danger_accept_invalid_certs(accept)
                                .danger_accept_invalid_hostnames(accept);
                        }
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
        retbytes: bool,
        retxml: bool,
    ) -> Result<(Option<Value>, Option<HeaderMap>), anyhow::Error> {
        let client = Self::build_http_client(option)?;
        let mut builder = client.request(method.clone(), url);
        let fmt_ = fmt.to_lowercase();
        log::info!("send request to {} {}", method.clone(), url);
        log::info!("data: {fmt} === {data}");
        if method == Method::POST || method == Method::PUT {
            if fmt_ == "xml" {
                builder = builder.body(data.to_string());
                builder = builder.header(CONTENT_TYPE, HeaderValue::from_static("text/xml"));
            } else if fmt_ == "json" {
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

        if retbytes {
            match builder.send().await {
                Ok(resp) => {
                    let headers = resp.headers().clone();
                    match resp
                        .bytes()
                        .await
                        .map(|f| {
                            let tx = base64_encode(&f);
                            Some(Value::String(tx))
                        })
                        .map_err(|err| anyhow!(err))
                    {
                        Ok(ts) => Ok((ts, Some(headers))),
                        Err(err) => Err(err),
                    }
                }
                Err(err) => Err(anyhow!(err)),
            }
        } else if retxml {
            match builder.send().await {
                Ok(resp) => {
                    let headers = resp.headers().clone();
                    let rt = resp
                        .text()
                        .await
                        .map(|t| serde_xml_rs::from_str(&t).unwrap_or(None))
                        .map_err(|err| anyhow!(err))?;
                    Ok((rt, Some(headers)))
                }
                Err(err) => Err(anyhow!(err)),
            }
        } else {
            match builder.send().await {
                Ok(resp) => {
                    let headers = resp.headers().clone();
                    let ret = resp.text().await.map_err(|err| anyhow!(err))?;
                    let rst = serde_json::from_str::<Value>(&ret)
                        .map(Some)
                        .unwrap_or(None);
                    Ok((rst, Some(headers)))
                }
                Err(err) => Err(anyhow!(err)),
            }
        }
    }

    /**
     * 发送HTTP SSE请求
     */
    pub async fn send_sse_request(
        url: &str,
        method: reqwest::Method,
        data: &str,
        fmt: &str,
        option: &Option<Value>,
    ) -> Result<EventSource, anyhow::Error> {
        let client = Self::build_http_client(option)?;
        let mut builder = client.request(method.clone(), url);
        let fmt_ = fmt.to_lowercase();
        log::info!("send request to {} {}", method.clone(), url);
        log::info!("data: {fmt} === {data}");
        if method == Method::POST || method == Method::PUT {
            if fmt_ == "xml" {
                builder = builder.body(data.to_string());
                builder = builder.header(CONTENT_TYPE, HeaderValue::from_static("text/xml"));
            } else if fmt_ == "json" {
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

        match reqwest_eventsource::EventSource::new(builder) {
            Ok(tt) => Ok(tt),
            Err(err) => Err(anyhow!("Could not clone the request {err}")),
        }

        // match builder.send().await {
        //         Ok(resp) => {
        //             // let mut events = resp.bytes_stream();
        //             let event = resp.bytes_stream();

        //             Ok(Box::new(resp.bytes_stream()))
        //         },
        //         Err(err) => Err(anyhow!(err)),
        // }
    }
}
