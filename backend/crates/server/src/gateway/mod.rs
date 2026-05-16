//! WebSocket gateway. One task per connection. Inbound frames are parsed,
//! validated, and dispatched. Outbound events arrive from Redis pub/sub and
//! are forwarded to the client.

mod frame;
mod presence_loop;

pub use frame::dispatch;
pub use presence_loop::start_presence_heartbeat;

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::time::interval;
use uuid::Uuid;

use proto::ws::{
    encode, ErrPayload, Frame, HelloPayload, OP_ERR, OP_HEARTBEAT, OP_HEARTBEAT_ACK, OP_HELLO,
};

use crate::state::AppState;

pub const HEARTBEAT_INTERVAL_MS: u32 = 15_000;
pub const HEARTBEAT_TIMEOUT_MS:  u64 = 45_000;
pub const SEND_BUFFER:           usize = 1024;

#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(ws_upgrade))
}

async fn ws_upgrade(
    State(state): State<AppState>,
    Query(q): Query<WsAuthQuery>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    // Auth the JWT BEFORE upgrade so we can return 401 cleanly.
    let user_id = match state.tokens.verify_access(&q.token) {
        Ok(uid) => uid,
        Err(_) => {
            return (axum::http::StatusCode::UNAUTHORIZED, "invalid token").into_response();
        }
    };
    upgrade.on_upgrade(move |socket| run_connection(socket, state, user_id))
}

/// Per-connection runtime state. Each WS connection gets one of these.
pub struct ConnCtx {
    pub user_id:        domain::ids::UserId,
    pub connection_id:  Uuid,
    pub seq:            Arc<AtomicU64>,
    pub subs_channels:  Arc<tokio::sync::RwLock<HashSet<Uuid>>>,
    pub subs_dms:       Arc<tokio::sync::RwLock<HashSet<Uuid>>>,
    pub send:           mpsc::Sender<Vec<u8>>,
}

async fn run_connection(socket: WebSocket, state: AppState, user_id: domain::ids::UserId) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(SEND_BUFFER);
    let connection_id = Uuid::new_v4();
    let seq = Arc::new(AtomicU64::new(0));

    // 1. Send HELLO
    let hello = Frame {
        op: OP_HELLO,
        d: serde_json::to_value(HelloPayload {
            user_id: user_id.as_uuid(),
            server_version: env!("CARGO_PKG_VERSION").into(),
            heartbeat_interval_ms: HEARTBEAT_INTERVAL_MS,
            connection_id,
        }).unwrap_or(serde_json::Value::Null),
        nonce: None,
        seq: Some(seq.fetch_add(1, Ordering::Relaxed)),
    };
    if sink.send(WsMessage::Binary(encode(&hello))).await.is_err() {
        return;
    }

    let ctx = Arc::new(ConnCtx {
        user_id,
        connection_id,
        seq: seq.clone(),
        subs_channels: Arc::new(Default::default()),
        subs_dms:      Arc::new(Default::default()),
        send: tx,
    });

    // 2. Spawn the outbound pump (Redis → client).
    let outbound_state = state.clone();
    let outbound_ctx   = ctx.clone();
    let outbound = tokio::spawn(async move {
        pump_outbound(outbound_state, outbound_ctx).await;
    });

    // 3. Spawn the writer (mpsc → ws sink).
    let writer = tokio::spawn(async move {
        while let Some(bytes) = rx.recv().await {
            if sink.send(WsMessage::Binary(bytes)).await.is_err() {
                break;
            }
        }
    });

    // 4. Presence heartbeat: register user as online + refresh TTL.
    let presence_ctx = ctx.clone();
    let presence_state = state.clone();
    let presence_task = tokio::spawn(async move {
        presence_loop::run(presence_state, presence_ctx).await;
    });

    // 5. Inbound loop with heartbeat timeout.
    let mut hb = interval(Duration::from_millis(HEARTBEAT_TIMEOUT_MS));
    hb.tick().await; // skip first immediate tick
    let mut last_heard = std::time::Instant::now();

    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(WsMessage::Binary(bytes))) => {
                        last_heard = std::time::Instant::now();
                        match proto::ws::decode(&bytes) {
                            Ok(frame) => {
                                if frame.op == OP_HEARTBEAT {
                                    let _ = ctx.send.send(encode(&Frame {
                                        op: OP_HEARTBEAT_ACK, d: serde_json::Value::Null,
                                        nonce: frame.nonce, seq: None,
                                    })).await;
                                } else {
                                    dispatch(&state, &ctx, frame).await;
                                }
                            }
                            Err(e) => {
                                let _ = ctx.send.send(encode(&Frame {
                                    op: OP_ERR,
                                    d: serde_json::to_value(ErrPayload {
                                        code: "Invalid".into(),
                                        message: format!("frame decode: {e}"),
                                    }).unwrap_or(serde_json::Value::Null),
                                    nonce: None, seq: None,
                                })).await;
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None | Some(Err(_)) => break,
                    _ => { /* ignore Text / Ping / Pong */ }
                }
            }
            _ = hb.tick() => {
                if last_heard.elapsed() > Duration::from_millis(HEARTBEAT_TIMEOUT_MS) {
                    tracing::info!(?user_id, ?connection_id, "heartbeat timeout, closing");
                    break;
                }
            }
        }
    }

    presence_task.abort();
    outbound.abort();
    writer.abort();
}

/// Subscribe to Redis pub/sub channels for the connection's subscriptions, and
/// pump anything that arrives into the connection's outbound queue.
async fn pump_outbound(state: AppState, ctx: Arc<ConnCtx>) {
    use redis::AsyncCommands;
    let client = match redis::Client::open(
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into()),
    ) {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut pubsub = match client.get_async_pubsub().await {
        Ok(p) => p,
        Err(_) => return,
    };
    // Subscribe to user-pinned events (e.g., own presence echo, ack envelopes).
    let _ = pubsub.subscribe(format!("ws:fanout:user:{}", ctx.user_id)).await;

    // Periodically refresh subscriptions based on ctx.subs_channels / subs_dms.
    let mut last_subbed: HashSet<String> = HashSet::new();
    let mut refresh = interval(Duration::from_millis(500));

    loop {
        tokio::select! {
            msg = pubsub.on_message().next() => {
                if let Some(m) = msg {
                    if let Ok(bytes) = m.get_payload::<Vec<u8>>() {
                        if ctx.send.send(bytes).await.is_err() { break; }
                    }
                }
            }
            _ = refresh.tick() => {
                let cs = ctx.subs_channels.read().await.clone();
                let ds = ctx.subs_dms.read().await.clone();
                let mut want: HashSet<String> = HashSet::new();
                for c in &cs { want.insert(format!("ws:fanout:channel:{c}")); }
                for d in &ds { want.insert(format!("ws:fanout:dm:{d}")); }
                want.insert(format!("ws:fanout:user:{}", ctx.user_id));

                for n in want.difference(&last_subbed) {
                    let _ = pubsub.subscribe(n.clone()).await;
                }
                for n in last_subbed.difference(&want) {
                    let _ = pubsub.unsubscribe(n.clone()).await;
                }
                last_subbed = want;
            }
        }
    }
    // Suppress unused warning in some cargo configs.
    let _ = client;
}
