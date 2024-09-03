use anyhow::Result;
use chimes_store_core::service::starter::MxStoreService;
use flume::{Receiver, Sender};
use serde_json::{json, Value};

use crate::utils::AppConfig;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ManagementRequest {
    Reload(Sender<Value>, Vec<String>),
    Save(Sender<Value>, Vec<String>),
    /// Unload the runtime.
    Unload,
}

#[derive(Clone)]
pub struct ManagementState {
    pub sender: Sender<ManagementRequest>,
}

#[tokio::main]
pub async fn management_route(receiver: Receiver<ManagementRequest>) -> Result<()> {
    // let sender = {
    //    let (sender, _) = flume::unbounded();
    //    sender
    // };
    println!("enter the management_route");
    loop {
        // let sender = sender.clone();
        let listen = async {
            match receiver.recv_async().await.unwrap() {
                ManagementRequest::Reload(mgr_sender, paths) => {
                    tokio::spawn(async move {
                        // do the async load process to load all configuration for store-service
                        for path in paths {
                            if path == *"*" {
                                let webconf = AppConfig::config();
                                MxStoreService::load_all(webconf.model_path);
                            } else {
                                MxStoreService::load(path);
                            }
                        }
                        let _ = mgr_sender.send(json!({"status": "done"}));
                    });
                }
                ManagementRequest::Save(mgr_sender, namespaces) => {
                    tokio::spawn(async move {
                        // do the async load process to load all configuration for store-service
                        let webconf = AppConfig::config();
                        for ns in namespaces {
                            if let Err(err) = if ns == *"*" {
                                MxStoreService::save_all(webconf.model_path.clone())
                            } else {
                                MxStoreService::save(&ns, webconf.model_path.clone())
                            } {
                                let _ = mgr_sender
                                    .send(json!({"status": "err", "msg": format!("{:?}", err)}));
                                return;
                            }
                        }
                        let _ = mgr_sender.send(json!({"status": "done"}));
                    });
                }
                ManagementRequest::Unload => {}
            };
            anyhow::Ok(())
        };

        if let Err(err) = listen.await {
            log::error!("{err}");
        }
    }
}
