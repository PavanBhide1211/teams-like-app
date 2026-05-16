//! Inbound WS frame dispatch.

use std::sync::Arc;

use proto::ws::{
    encode, ErrPayload, Frame, SubscribePayload, OP_ACK, OP_ERR, OP_MSG_SEND, OP_PRESENCE_SET,
    OP_SUBSCRIBE, OP_SUBSCRIBE_ADD, OP_SUBSCRIBE_REMOVE, OP_TYPING, OP_TYPING_EVENT,
};

use domain::message::{validate_body, validate_mentions, MessageTarget, NewMessage};
use domain::presence::PresenceStatus;
use domain::ids::{ChannelId, DmThreadId, MessageId, UserId};

use crate::state::AppState;

pub async fn dispatch(state: &AppState, ctx: &Arc<super::ConnCtx>, frame: Frame) {
    match frame.op {
        OP_SUBSCRIBE        => handle_subscribe(ctx, frame, false).await,
        OP_SUBSCRIBE_ADD    => handle_subscribe(ctx, frame, true).await,
        OP_SUBSCRIBE_REMOVE => handle_unsubscribe(ctx, frame).await,
        OP_MSG_SEND         => handle_msg_send(state, ctx, frame).await,
        OP_TYPING           => handle_typing(state, ctx, frame).await,
        OP_PRESENCE_SET     => handle_presence_set(state, ctx, frame).await,
        _ => {
            let _ = ctx.send.send(encode(&Frame {
                op: OP_ERR,
                d: serde_json::to_value(ErrPayload {
                    code: "Invalid".into(),
                    message: format!("unknown opcode {:#x}", frame.op),
                }).unwrap_or(serde_json::Value::Null),
                nonce: frame.nonce, seq: None,
            })).await;
        }
    }
}

async fn handle_subscribe(ctx: &Arc<super::ConnCtx>, frame: Frame, additive: bool) {
    let payload: SubscribePayload = match serde_json::from_value(frame.d.clone()) {
        Ok(p) => p,
        Err(_) => return,
    };
    {
        let mut cs = ctx.subs_channels.write().await;
        if !additive { cs.clear(); }
        for c in payload.channel_ids { cs.insert(c); }
    }
    {
        let mut ds = ctx.subs_dms.write().await;
        if !additive { ds.clear(); }
        for d in payload.dm_thread_ids { ds.insert(d); }
    }
    let _ = ctx.send.send(encode(&Frame {
        op: OP_ACK, d: serde_json::Value::Null, nonce: frame.nonce, seq: None,
    })).await;
}

async fn handle_unsubscribe(ctx: &Arc<super::ConnCtx>, frame: Frame) {
    let payload: SubscribePayload = match serde_json::from_value(frame.d.clone()) {
        Ok(p) => p,
        Err(_) => return,
    };
    {
        let mut cs = ctx.subs_channels.write().await;
        for c in payload.channel_ids { cs.remove(&c); }
    }
    {
        let mut ds = ctx.subs_dms.write().await;
        for d in payload.dm_thread_ids { ds.remove(&d); }
    }
    let _ = ctx.send.send(encode(&Frame {
        op: OP_ACK, d: serde_json::Value::Null, nonce: frame.nonce, seq: None,
    })).await;
}

#[derive(serde::Deserialize)]
struct MsgSendBody {
    channel_id:   Option<uuid::Uuid>,
    dm_thread_id: Option<uuid::Uuid>,
    parent_id:    Option<uuid::Uuid>,
    body:         String,
    #[serde(default)]
    mentions:     Vec<uuid::Uuid>,
}

async fn handle_msg_send(state: &AppState, ctx: &Arc<super::ConnCtx>, frame: Frame) {
    let body: MsgSendBody = match serde_json::from_value(frame.d.clone()) {
        Ok(b) => b,
        Err(_) => {
            return send_err(ctx, frame.nonce, "Invalid", "bad MSG_SEND payload").await;
        }
    };

    if validate_body(&body.body).is_err() {
        return send_err(ctx, frame.nonce, "Invalid", "body invalid").await;
    }
    let mentions: Vec<UserId> = body.mentions.into_iter().map(UserId::from_uuid).collect();
    if validate_mentions(&mentions).is_err() {
        return send_err(ctx, frame.nonce, "Invalid", "too many mentions").await;
    }

    let target = match (body.channel_id, body.dm_thread_id) {
        (Some(c), None) => {
            let cid = ChannelId::from_uuid(c);
            match state.channels.is_member(cid, ctx.user_id).await {
                Ok(true) => MessageTarget::Channel { channel_id: cid },
                _ => return send_err(ctx, frame.nonce, "Forbidden", "not a channel member").await,
            }
        }
        (None, Some(d)) => {
            let did = DmThreadId::from_uuid(d);
            match state.dms.is_member(did, ctx.user_id).await {
                Ok(true) => MessageTarget::Dm { dm_thread_id: did },
                _ => return send_err(ctx, frame.nonce, "Forbidden", "not in this DM").await,
            }
        }
        _ => return send_err(ctx, frame.nonce, "Invalid", "exactly one target required").await,
    };

    let m = match state.messages.create(NewMessage {
        target, parent_id: body.parent_id.map(MessageId::from_uuid),
        author_id: ctx.user_id, body: body.body, mentions,
    }).await {
        Ok(m) => m,
        Err(e) => return send_err(ctx, frame.nonce, e.code(), &e.to_string()).await,
    };

    // ACK to sender, with persisted id.
    let _ = ctx.send.send(encode(&Frame {
        op: OP_ACK,
        d: serde_json::json!({ "server_id": m.id.as_uuid() }),
        nonce: frame.nonce,
        seq: None,
    })).await;

    // Broadcast MSG_CREATED.
    let dto = proto::MessageDto::from(m.clone());
    let event = Frame {
        op: proto::ws::OP_MSG_CREATED,
        d: serde_json::to_value(dto).unwrap_or(serde_json::Value::Null),
        nonce: None, seq: None,
    };
    let bytes = encode(&event);
    match m.target {
        MessageTarget::Channel { channel_id } => {
            let _ = state.bus.publish_channel(channel_id, &bytes).await;
        }
        MessageTarget::Dm { dm_thread_id } => {
            let _ = state.bus.publish_dm(dm_thread_id, &bytes).await;
        }
    }
}

#[derive(serde::Deserialize)]
struct TypingBody {
    channel_id:   Option<uuid::Uuid>,
    dm_thread_id: Option<uuid::Uuid>,
}

async fn handle_typing(state: &AppState, ctx: &Arc<super::ConnCtx>, frame: Frame) {
    let body: TypingBody = match serde_json::from_value(frame.d.clone()) {
        Ok(b) => b, Err(_) => return,
    };
    let evt = Frame {
        op: OP_TYPING_EVENT,
        d: serde_json::json!({
            "user_id": ctx.user_id.as_uuid(),
            "channel_id":   body.channel_id,
            "dm_thread_id": body.dm_thread_id,
            "at": chrono::Utc::now(),
        }),
        nonce: None, seq: None,
    };
    let bytes = encode(&evt);
    match (body.channel_id, body.dm_thread_id) {
        (Some(c), _) => { let _ = state.bus.publish_channel(ChannelId::from_uuid(c), &bytes).await; }
        (_, Some(d)) => { let _ = state.bus.publish_dm(DmThreadId::from_uuid(d), &bytes).await; }
        _ => {}
    }
}

#[derive(serde::Deserialize)]
struct PresenceSetBody {
    status: String,
}

async fn handle_presence_set(state: &AppState, ctx: &Arc<super::ConnCtx>, frame: Frame) {
    let body: PresenceSetBody = match serde_json::from_value(frame.d.clone()) {
        Ok(b) => b, Err(_) => return,
    };
    let status = match body.status.as_str() {
        "online" => PresenceStatus::Online,
        "away"   => PresenceStatus::Away,
        "dnd"    => PresenceStatus::Dnd,
        _ => return,
    };
    let _ = state.presence.touch(ctx.user_id, status).await;
    let _ = ctx.send.send(encode(&Frame {
        op: OP_ACK, d: serde_json::Value::Null, nonce: frame.nonce, seq: None,
    })).await;
}

async fn send_err(ctx: &Arc<super::ConnCtx>, nonce: Option<u32>, code: &str, msg: &str) {
    let _ = ctx.send.send(encode(&Frame {
        op: OP_ERR,
        d: serde_json::to_value(ErrPayload {
            code: code.into(), message: msg.into(),
        }).unwrap_or(serde_json::Value::Null),
        nonce, seq: None,
    })).await;
}
