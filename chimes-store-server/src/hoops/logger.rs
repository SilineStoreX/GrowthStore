use std::time::Instant;

use anyhow::anyhow;
use chimes_store_core::{config::auth::JwtUserClaims, service::{perfs::InvokeCounter, registry::SchemaRegistry}};
use itertools::Itertools;
use reqwest::header::CONTENT_TYPE;
use salvo::{
    async_trait, http::{ResBody, StatusCode}, prelude::JwtAuthDepotExt, Depot, FlowCtrl, Handler, Request, Response
};
use tracing::{Instrument, Level};


trait ReqBodyHandle {
    fn is_json_request(&mut self) -> bool;

    fn get_remote_addr(&self) -> Option<String>;
}

impl ReqBodyHandle for Request {
    fn is_json_request(&mut self) -> bool {
        
        if let Some(ct) = self.headers_mut().get(CONTENT_TYPE.as_str()) {
            ct.to_str().map(|s| s.contains("json") && !s.contains("multipart")).unwrap_or_default()
        } else {
            false
        }
    }

    fn get_remote_addr(&self) -> Option<String> {
        if let Some(ip) = self.header::<&str>("X-Forwarded-For") {
            if ip.is_empty() || ip == "unknown" {
                if let Some(nip) = self.header::<&str>("X-Real-IP") {
                    let sip = nip.split(",").collect_vec();
                    if sip.is_empty() {
                        Some(self.remote_addr().to_string())
                    } else {
                        Some(sip[0].to_string())
                    }
                } else {
                    Some(self.remote_addr().to_string())
                }
            } else {
                let sip = ip.split(",").collect_vec();
                if sip.is_empty() {
                    Some(self.remote_addr().to_string())
                } else {
                    Some(sip[0].to_string())
                }
            }
        } else {
            Some(self.remote_addr().to_string())
        }
    }
}

/// A simple logger middleware.
#[derive(Default, Debug)]
pub struct StoreXLogger {
    enabled_perf: bool,
}

impl StoreXLogger {
    /// Create new `Logger` middleware.
    #[inline]
    pub fn new(perfs: bool) -> Self {
        StoreXLogger {
            enabled_perf: perfs,
        }
    }
}

#[async_trait]
impl Handler for StoreXLogger {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        let bodyval = if req.is_json_request() {
            req.parse_json::<serde_json::Value>().await.map(Some).unwrap_or(None)
        } else {
            None
        };

        let query = req.queries();
        let pathparams = req.params().clone();
        let remote_addr = req.get_remote_addr().unwrap_or_default();
        let span = tracing::span!(
            Level::INFO,
            "Request",
            client_addr = %remote_addr,
            version = ?req.version(),
            method = %req.method(),
            path = %req.uri(),
        );
        
        let jwt = if let Some(jwtdata) = depot.jwt_auth_data::<JwtUserClaims>() {
            Some(jwtdata.claims.clone())
        } else {
            None
        };

        let span_id = query.get::<String>(&"span_id".to_string()).map(|s| s.to_string());

        async move {
            let mut ict = InvokeCounter::from_uri(req.uri(), &pathparams, span_id)
                                            .with_remote_addr(&remote_addr.clone())
                                            .with_user(&jwt)
                                            .with_payload(bodyval);
            depot.insert("__SPAN_ID", ict.span_id.clone().unwrap_or_default());
            let now = Instant::now();
            ctrl.call_next(req, depot, res).await;
            let duration = now.elapsed();
            if jwt.is_none() {
                if let Some(jwtdata) = depot.jwt_auth_data::<JwtUserClaims>() {
                    ict = ict.with_user(&Some(jwtdata.claims.clone()));
                }
            }
            let status = res.status_code.unwrap_or(match &res.body {
                ResBody::None => {
                    if self.enabled_perf {
                        SchemaRegistry::send_invoke_err(&mut ict, &anyhow!("Not-Found"));
                    }
                    StatusCode::NOT_FOUND
                }
                ResBody::Error(e) => {
                    if self.enabled_perf {
                        SchemaRegistry::send_invoke_err(&mut ict, &anyhow!("{}", e));
                    }
                    e.code
                }
                _ => {
                    if self.enabled_perf {
                        SchemaRegistry::send_invoke_count(&mut ict);
                    }
                    StatusCode::OK
                }
            });

            tracing::info!(
                %status,
                ?duration,
                "Response"
            );
        }
        .instrument(span)
        .await
    }
}
