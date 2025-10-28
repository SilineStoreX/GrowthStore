use chimes_store_core::config::auth::JwtUserClaims;
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::sdk::InvokeUri;
use chimes_store_core::service::starter::MxStoreService;
use futures_util::{FutureExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use substring::Substring;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

use salvo::prelude::*;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};

type ChatUser = RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, salvo::Error>>>>;

type UserInfo = RwLock<HashMap<usize, JwtUserClaims>>;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
static ONLINE_CHAT_USERS: LazyLock<ChatUser> = LazyLock::new(ChatUser::default);
static ONLINE_USERINFO: LazyLock<UserInfo> = LazyLock::new(UserInfo::default);
static WEBSCOKET_GRANT: LazyLock<tokio::sync::Mutex<bool>> =
    LazyLock::new(|| tokio::sync::Mutex::new(true));

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebSocketStandRequest {
    pub uri: Option<String>,
    pub params: Vec<Value>,
    pub resholder: Option<String>,
    pub sse: Option<bool>,
}

#[handler]
pub async fn websocket_connect(
    depot: &mut Depot,
    req: &mut Request,
    res: &mut Response,
) -> Result<(), StatusError> {
    match WEBSCOKET_GRANT.try_lock() {
        Ok(_) => {
            let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
            if depot.jwt_auth_state() == JwtAuthState::Authorized {
                if let Some(jwtdata) = depot.jwt_auth_data::<JwtUserClaims>() {
                    ONLINE_USERINFO
                        .write()
                        .await
                        .insert(my_id, jwtdata.claims.clone());
                }
            }

            match WebSocketUpgrade::new()
                .upgrade(req, res, handle_socket)
                .await
            {
                Ok(_) => {
                    // locked release
                    Ok(())
                }
                Err(err) => {
                    // locked release
                    Err(err)
                }
            }
        }
        Err(err) => {
            log::warn!("Unable to get the LOCK. Please try it later. {err}");
            Err(StatusError::locked())
        }
    }
}

async fn handle_socket(ws: WebSocket) {
    // Use a counter to assign a new unique ID for this user.
    // let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
    let my_id = NEXT_USER_ID.load(Ordering::Acquire);

    tracing::info!("new chat user: {}", my_id);

    // Split the socket into a sender and receive of messages.
    let (user_ws_tx, mut user_ws_rx) = ws.split();

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);
    let fut = rx.forward(user_ws_tx).map(|result| {
        if let Err(e) = result {
            tracing::error!(error = ?e, "websocket send error");
        }
    });
    tokio::task::spawn(fut);
    let fut = async move {
        ONLINE_CHAT_USERS.write().await.insert(my_id, tx.clone());

        while let Some(result) = user_ws_rx.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("websocket error(uid={my_id}): {e}");
                    break;
                }
            };

            websocket_message(my_id, msg, &tx).await;
        }

        webscoket_disconnected(my_id).await;
    };
    tokio::task::spawn(fut);
}

async fn websocket_message(
    my_id: usize,
    msg: Message,
    tx: &mpsc::UnboundedSender<Result<Message, salvo::Error>>,
) {
    if msg.is_ping() {
        if let Err(_disconnected) = tx.send(Ok(Message::pong("PONG"))) {
            // The tx is disconnected, our `user_disconnected` code
            // should be happening in another task, nothing more to
            // do here.
        }
        return;
    }

    let msg = if let Ok(s) = msg.as_str() {
        log::info!("received msg: {s}");
        match serde_json::from_str::<WebSocketStandRequest>(s) {
            Ok(t) => t,
            Err(err) => {
                log::error!("Could net parse the msg into WebSocketStandRequest format {err}.");
                if let Err(_disconnected) = tx.send(Ok(Message::pong("PONG"))) {
                    // The tx is disconnected, our `user_disconnected` code
                    // should be happening in another task, nothing more to
                    // do here.
                }
                return;
            }
        }
    } else {
        return;
    };

    let new_msg: String = format!("<User#{my_id}>: {msg:?}");

    log::info!("Reply msg: {new_msg}");

    // call the ctx
    let mut ctx_inner = InvocationContext::new();

    if let Some(usercliam) = ONLINE_USERINFO.read().await.get(&my_id) {
        ctx_inner.inject(usercliam.to_owned());
    }

    let ctx = Arc::new(Mutex::new(ctx_inner));

    if msg.sse.unwrap_or_default() {
        websocket_sse_message(ctx, my_id, &msg, tx).await;
        return;
    }

    let full_invoke_uri = msg.uri.unwrap_or_default();
    let holder = msg.resholder.unwrap_or("option".to_owned()).to_lowercase();

    let result = match holder.as_str() {
        "list" => match MxStoreService::invoke_return_vec(full_invoke_uri, ctx, msg.params).await {
            Ok(rs) => Value::Array(rs),
            Err(err) => Value::String(format!("ERROR: {err:?}")),
        },
        "paged" => {
            match MxStoreService::invoke_return_page(full_invoke_uri, ctx, msg.params).await {
                Ok(rs) => {
                    json!(rs)
                }
                Err(err) => Value::String(format!("ERROR: {err:?}")),
            }
        }
        _ => match MxStoreService::invoke_return_one(full_invoke_uri, ctx, msg.params).await {
            Ok(rs) => rs.unwrap_or(Value::Null),
            Err(err) => Value::String(format!("ERROR: {err:?}")),
        },
    };

    if let Err(err) = tx.send(Ok(Message::text(
        serde_json::to_string(&result).unwrap_or("ERR".to_owned()),
    ))) {
        // The tx is disconnected, our `user_disconnected` code
        // should be happening in another task, nothing more to
        // do here.
        log::warn!("disconnected: {err}");
    }

    // New message from this user, send it to everyone else (except same uid)...
    // for (&uid, tx) in ONLINE_CHAT_USERS.read().await.iter() {
    //     if my_id == uid {
    //         if let Err(_disconnected) = tx.send(Ok(Message::text(new_msg.clone()))) {
    //             // The tx is disconnected, our `user_disconnected` code
    //             // should be happening in another task, nothing more to
    //             // do here.
    //         }
    //     }
    // }
}

async fn websocket_sse_message(
    ctx: Arc<Mutex<InvocationContext>>,
    _id: usize,
    req: &WebSocketStandRequest,
    tx: &mpsc::UnboundedSender<Result<Message, salvo::Error>>,
) {
    let invoke_uri = match InvokeUri::parse(&req.uri.clone().unwrap_or_default()) {
        Ok(u) => u,
        Err(err) => {
            log::error!("Could not parse the InvokeURI {err}");
            if let Err(err) = tx.send(Ok(Message::text("Unable to parse URI".to_owned()))) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
                log::warn!("disconnected: {err}");
            }
            return;
        }
    };

    log::info!("INvoke_URI: {}", invoke_uri.url());

    match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
        Some(pls) => {
            if let Some(ssepls) = pls.to_sse_request() {
                match ssepls
                    .invoke_sse_request(invoke_uri, ctx, req.params.clone())
                    .await
                {
                    Ok(mut stream) => loop {
                        match stream.next().await {
                            Some(Ok(ssevt)) => {
                                let text = ssevt.to_string();
                                let nt = text
                                    .lines()
                                    .filter(|p| p.starts_with("data:"))
                                    .map(|l| l.substring(5, l.len()).to_string())
                                    .next_back();
                                if let Err(err) = tx.send(Ok(Message::text(nt.unwrap_or_default())))
                                {
                                    log::warn!("disconnected: {err}");
                                }
                            }
                            Some(Err(err)) => {
                                log::error!("Error for try next: {err}");
                                let text = format!("ERR to get sse event {err}");
                                if let Err(err) = tx.send(Ok(Message::text(text))) {
                                    log::warn!("disconnected: {err}");
                                }
                                break;
                            }
                            None => {
                                break;
                            }
                        }
                    },
                    Err(err) => {
                        log::error!("Could not execute {err}");
                        let text = format!("ERR to execute sse event {err}");
                        if let Err(err) = tx.send(Ok(Message::text(text))) {
                            log::warn!("disconnected: {err}");
                        }
                    }
                }
            }
        }
        None => {
            if let Err(err) = tx.send(Ok(Message::text("Unsupport SSE".to_owned()))) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
                log::warn!("disconnected: {err}");
            }
        }
    }
}

pub async fn webscoket_disconnected(my_id: usize) {
    eprintln!("good bye user: {my_id}");
    // Stream closed up, so remove from the user list
    ONLINE_CHAT_USERS.write().await.remove(&my_id);
    ONLINE_USERINFO.write().await.remove(&my_id);
}
