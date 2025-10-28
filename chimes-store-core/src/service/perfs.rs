use std::{
    collections::{HashMap, VecDeque},
    sync::{atomic::AtomicBool, Arc, Condvar, Mutex},
    time::Duration,
};

use super::{invoker::InvocationContext, registry::SchemaRegistry, sdk::InvokeUri};
use crate::{config::auth::JwtUserClaims, pin_spawnthread, utils::get_local_timestamp_micros};
use anyhow::anyhow;
use chimes_store_utils::algorithm::snowflake_id;
use itertools::Itertools;
use salvo::{hyper::Uri, routing::PathParams};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use substring::Substring;

pub trait PerformanceQueue {
    fn add_invoke_counter(&mut self, ict: &InvokeCounter);
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PerformanceSummary {
    pub full_url: String,
    pub namespace: String,
    pub protocol: String,
    pub refname: String,
    pub method: String,
    pub success_count: u64,
    pub failure_count: u64,
    pub success_elapse: u64, // total success elapse
    pub failure_elapse: u64, // total failure elapse
    pub max_elapse: u64,     // only calc success
    pub min_elapse: u64,     // only calc success
    pub avg_elapse: u64,     // only calc success
}

impl PerformanceSummary {
    pub fn create_from(ict: &InvokeCounter) -> Self {
        Self {
            full_url: ict.uri.url(),
            namespace: ict.uri.namespace.clone(),
            protocol: ict.uri.schema.clone(),
            refname: ict.uri.object.clone(),
            method: ict.uri.method.clone(),
            success_count: if ict.error { 0 } else { 1 },
            failure_count: if ict.error { 1 } else { 0 },
            success_elapse: if ict.error { 0 } else { ict.elapse },
            failure_elapse: if ict.error { ict.elapse } else { 0 },
            max_elapse: ict.elapse,
            min_elapse: ict.elapse,
            avg_elapse: ict.elapse,
        }
    }

    pub fn calc(&mut self, ict: &InvokeCounter) -> &Self {
        if ict.error {
            self.failure_count += 1;
            self.failure_elapse += ict.elapse;
        } else {
            self.success_count += 1;
            self.success_elapse += ict.elapse;
            self.max_elapse = if self.max_elapse < ict.elapse {
                ict.elapse
            } else {
                self.max_elapse
            };
            self.min_elapse = if self.min_elapse > ict.elapse {
                ict.elapse
            } else {
                self.min_elapse
            };

            if self.success_count > 0 {
                self.avg_elapse = self.success_elapse / self.success_count;
            }
        }
        self
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct InvokePerformanceInfo {
    pub full_url: String,
    pub remote_addr: String,
    pub namespace: String,
    pub protocol: String,
    pub refname: String,
    pub method: String,
    pub start_time: u64,
    pub end_time: u64,
    pub elapse: u64,
    pub error: bool,
    pub msg: Option<String>,
}

unsafe impl Send for InvokePerformanceInfo {}

unsafe impl Sync for InvokePerformanceInfo {}

#[derive(Clone, Debug)]
pub struct InvokeCounter {
    pub span_id: Option<String>,
    pub parent_span: Option<String>,
    pub uri: InvokeUri,
    pub request_url: Option<String>,
    pub remote_addr: Option<String>,
    pub elapse: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub error: bool,
    pub msg: Option<String>,
    pub ref_name: Option<String>,
    pub ref_detail: Option<String>,
    pub payload: Option<Value>,
    pub username: Option<String>,
    pub user_id: Option<String>,
    pub domain: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct InvokeAuditInfo {
    pub span_id: Option<String>,
    pub parent_id: Option<String>,
    pub uri: Option<String>,
    pub request_url: Option<String>,
    pub namespace: Option<String>,
    pub method: Option<String>,
    pub protocol: Option<String>,
    pub remote_addr: Option<String>,
    pub elapse: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub error: bool,
    pub message: Option<String>,
    pub ref_name: Option<String>,
    pub ref_detail: Option<String>,
    pub payload: Option<String>,
    pub username: Option<String>,
    pub user_id: Option<String>,
    pub domain: Option<String>,
}

unsafe impl Send for InvokeCounter {}

unsafe impl Sync for InvokeCounter {}

impl InvokeCounter {
    pub fn from_uri(uri: &Uri, pathps: &PathParams, span: Option<String>) -> Self {
        let (key_, last_) = match pathps.last() {
            Some((key, val)) => (key.to_owned(), val.to_owned()),
            None => ("".to_owned(), "".to_owned()),
        };

        let mut ivk_uri = match InvokeUri::parse(&uri.to_string()) {
            Ok(u) => u,
            Err(_) => InvokeUri {
                schema: uri.scheme_str().unwrap_or_default().to_owned(),
                namespace: uri.host().unwrap_or_default().to_owned(),
                object: uri.path().to_owned(),
                method: "".to_owned(),
                query: uri.query().map(|f| f.to_owned()),
                query_pairs: None,
            },
        };

        // fix the end part of path params
        if !last_.is_empty() {
            let q = ivk_uri.object.clone();
            if q.ends_with(&last_) {
                let mut nstr = q.substring(0, q.len() - last_.len()).to_string();
                nstr.push_str(&format!(":{key_}"));
                ivk_uri.object = nstr;
            }
        }

        // here to try to parse the namespace for HTTP(S) URL
        if ivk_uri.schema == "http".to_string() || ivk_uri.schema == "https" {
            let objpath = ivk_uri.object.clone();
            let pathspl = objpath.split("/").collect_vec();
            if pathspl.len() > 3 {
                if pathspl[0] == "api" {
                    ivk_uri.namespace = pathspl[2].to_string();
                }
            }
        }

        Self {
            parent_span: span,
            span_id: Some(format!("REQ{}", snowflake_id())),
            uri: ivk_uri,
            elapse: 0,
            request_url: Some(uri.to_string()),
            start_time: get_local_timestamp_micros(),
            end_time: 0,
            error: false,
            msg: None,
            remote_addr: None,
            ref_detail: None,
            ref_name: None,
            payload: None,
            username: None,
            user_id: None,
            domain: None,
        }
    }

    pub fn new(uri: &InvokeUri, ctx: Arc<Mutex<InvocationContext>>, payload: &[Value]) -> Self {
        let (parent_sp, userid, username, domain) = if let Ok(ctxgrt) = ctx.lock() {
            (ctxgrt.get_span_id(), ctxgrt.get_current_userid(), ctxgrt.get_current_user(), ctxgrt.get_domain())
        } else {
            (None, None, None, None)
        };

        Self {
            parent_span: parent_sp,
            span_id: Some(format!("IVK{}", snowflake_id())),
            uri: uri.clone(),
            elapse: 0,
            request_url: None,
            start_time: get_local_timestamp_micros(),
            end_time: 0,
            error: false,
            msg: None,
            remote_addr: None,
            payload: Some(Value::Array(payload.to_vec())),
            username: username,
            user_id: userid,
            ref_detail: None,
            ref_name: None,
            domain: domain,
        }
    }

    pub fn with_remote_addr(mut self, addr: &str) -> Self {
        self.remote_addr = Some(addr.to_owned());
        self
    }

    pub fn with_user(mut self, jwt: &Option<JwtUserClaims>) -> Self {
        if let Some(jwtus) = jwt {
            self.username = Some(jwtus.username.clone());
            self.user_id = Some(jwtus.userid.clone());
            self.domain = Some(jwtus.domain.clone());
        }
        
        self
    }

    pub fn with_payload(mut self, payload: Option<Value>) -> Self {
        self.payload = payload;
        self
    }

    pub fn finalized(&mut self) -> &Self {
        let et = get_local_timestamp_micros();
        self.end_time = et;
        self.elapse = et - self.start_time;
        self
    }

    pub fn finalize_error(&mut self, err: &anyhow::Error) -> &Self {
        let et = get_local_timestamp_micros();
        self.end_time = et;
        self.elapse = et - self.start_time;
        self.error = true;
        self.msg = Some(err.to_string());
        self
    }

    pub fn set_payload(&mut self, payload: &[Value]) {
        self.payload = Some(Value::Array(payload.to_vec()));
    }

    pub fn to_performance_info(&self) -> InvokePerformanceInfo {
        InvokePerformanceInfo {
            full_url: if self.request_url.is_none() {
                self.uri.url()
            } else {
                self.request_url.clone().unwrap_or_default()
            },
            remote_addr: self.remote_addr.clone().unwrap_or_default(),
            namespace: self.uri.namespace.clone(),
            protocol: self.uri.schema.clone(),
            refname: self.uri.object.clone(),
            method: self.uri.method.clone(),
            start_time: self.start_time,
            end_time: self.end_time,
            elapse: self.elapse,
            error: self.error,
            msg: self.msg.clone(),
        }
    }

    pub fn to_audit_info(&self) -> InvokeAuditInfo {
        InvokeAuditInfo {
            span_id: self.span_id.clone(),
            parent_id: self.parent_span.clone(),
            uri: Some(self.uri.url()),
            request_url: self.request_url.clone(),
            remote_addr: self.remote_addr.clone(),
            namespace: Some(self.uri.namespace.clone()),
            protocol: Some(self.uri.schema.clone()),
            ref_name: Some(self.uri.object.clone()),
            method: Some(self.uri.method.clone()),
            start_time: self.start_time,
            end_time: self.end_time,
            elapse: self.elapse,
            error: self.error,
            username: self.username.clone(),
            user_id: self.user_id.clone(),
            domain: self.domain.clone(),
            message: self.msg.clone(),
            payload: self
                .payload
                .clone()
                .and_then(|s| serde_json::to_string(&s).map(Some).unwrap_or(None)),
            ..Default::default()
        }
    }
}

pub struct PerformanceQueueHolder {
    queue: Mutex<VecDeque<InvokeCounter>>,
    cond: Condvar,
    consumer: String,
    running: Arc<AtomicBool>,
    summary_map: Mutex<HashMap<String, PerformanceSummary>>,
}

unsafe impl Send for PerformanceQueueHolder {}

unsafe impl Sync for PerformanceQueueHolder {}

impl Default for PerformanceQueueHolder {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceQueueHolder {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            cond: Condvar::new(),
            consumer: String::new(),
            running: Arc::new(AtomicBool::new(false)),
            summary_map: Mutex::new(HashMap::new()),
        }
    }

    pub fn stop(&mut self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Release);
        self.notify_all();
    }

    pub fn notify_all(&mut self) {
        self.cond.notify_all();
    }

    pub fn get_performance_summary(&self) -> Vec<PerformanceSummary> {
        self.summary_map
            .lock()
            .unwrap()
            .values()
            .map(|f| f.to_owned())
            .collect_vec()
    }

    pub fn update_consumer(&mut self, cs: &str) {
        self.consumer = cs.to_string();
    }

    pub fn pop(&mut self) -> Result<Option<InvokeCounter>, anyhow::Error> {
        Ok(self.queue.lock().unwrap().pop_front().clone())
    }

    pub fn queue_len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    pub fn wait_for(&self) -> Result<(), anyhow::Error> {
        if let Err(err) = self
            .cond
            .wait_timeout(self.queue.lock().unwrap(), Duration::from_millis(2000))
        {
            Err(anyhow!(err.to_string()))
        } else {
            Ok(())
        }
    }

    pub fn get_perf(&self, key: &str) -> Option<PerformanceSummary> {
        self.summary_map
            .lock()
            .unwrap()
            .get(key)
            .map(|f| f.to_owned())
    }

    pub fn insert_perf(&self, key: &str, perf: &PerformanceSummary) {
        self.summary_map
            .lock()
            .unwrap()
            .insert(key.to_owned(), perf.to_owned());
    }

    pub async fn write(&self, ict: &InvokePerformanceInfo) {
        if !self.consumer.is_empty() {
            let consumer_uri = self.consumer.clone();
            let ctx = Arc::new(Mutex::new(InvocationContext::new()));
            if let Ok(perf) = serde_json::to_value(ict) {
                if let Err(err) = SchemaRegistry::get()
                    .direct_invoke_return_one(&consumer_uri, ctx, &[perf])
                    .await
                {
                    log::debug!("error to send the performance info into consumer {err}");
                }
            }
        }
    }

    // 启动落库线程
    pub fn start_performance_consume_thread(&'static mut self) {
        let is_started = self.running.load(std::sync::atomic::Ordering::Acquire);

        if is_started {
            return;
        }

        // pin_spawnthread!()
        pin_spawnthread!(async move {
            self.running
                .store(true, std::sync::atomic::Ordering::Release);

            loop {
                let started = self.running.load(std::sync::atomic::Ordering::Acquire);
                if !started {
                    log::info!("exit the performance consumer loop.");
                    break;
                }
                // calling the method

                // log::debug!("try to pop the InvokeCounter object ... {}", self.queue_len());
                if let Ok(Some(ts)) = self.pop() {
                    // lookup the writer
                    let full_key = ts.uri.url();
                    let perf = self
                        .get_perf(&full_key)
                        .map(|f| {
                            let mut fown = f.to_owned();
                            fown.calc(&ts).clone()
                        })
                        .unwrap_or(PerformanceSummary::create_from(&ts));

                    self.insert_perf(&full_key, &perf);

                    self.write(&ts.to_performance_info()).await
                } else if let Err(err) = self.wait_for() {
                    log::debug!("tracking to wait for {err}");
                }
            }
        });
    }
}

impl PerformanceQueue for PerformanceQueueHolder {
    fn add_invoke_counter(&mut self, ict: &InvokeCounter) {
        if let Ok(mut tmap) = self.queue.lock() {
            tmap.push_back(ict.clone());
            self.cond.notify_all();
        }
    }
}
