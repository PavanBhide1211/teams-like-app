//! WS frame schema. Each frame is a msgpack map with `op` and `d`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Frame {
    pub op: u8,
    #[serde(default)]
    pub d:  serde_json::Value,   // payload (untyped at this level)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq:   Option<u64>,
}

// Session control
pub const OP_HELLO:          u8 = 0x00;
pub const OP_HEARTBEAT:      u8 = 0x01;
pub const OP_HEARTBEAT_ACK:  u8 = 0x02;
pub const OP_RESUME:         u8 = 0x03;
pub const OP_RESUMED:        u8 = 0x04;
pub const OP_CLOSE:          u8 = 0x05;

// Subscriptions
pub const OP_SUBSCRIBE:        u8 = 0x10;
pub const OP_SUBSCRIBE_ADD:    u8 = 0x11;
pub const OP_SUBSCRIBE_REMOVE: u8 = 0x12;

// Actions
pub const OP_MSG_SEND:    u8 = 0x20;
pub const OP_MSG_EDIT:    u8 = 0x21;
pub const OP_MSG_DELETE:  u8 = 0x22;
pub const OP_REACT_ADD:   u8 = 0x23;
pub const OP_REACT_REMOVE: u8 = 0x24;
pub const OP_TYPING:      u8 = 0x25;
pub const OP_PRESENCE_SET: u8 = 0x26;

// Events
pub const OP_MSG_CREATED:    u8 = 0x30;
pub const OP_MSG_EDITED:     u8 = 0x31;
pub const OP_MSG_DELETED:    u8 = 0x32;
pub const OP_REACT_CREATED:  u8 = 0x33;
pub const OP_REACT_REMOVED:  u8 = 0x34;
pub const OP_TYPING_EVENT:   u8 = 0x35;
pub const OP_PRESENCE_EVENT: u8 = 0x36;

// Acks and errors
pub const OP_ACK: u8 = 0xF0;
pub const OP_ERR: u8 = 0xF1;

#[derive(Debug, Serialize, Deserialize)]
pub struct HelloPayload {
    pub user_id: Uuid,
    pub server_version: String,
    pub heartbeat_interval_ms: u32,
    pub connection_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribePayload {
    #[serde(default)]
    pub channel_ids:   Vec<Uuid>,
    #[serde(default)]
    pub dm_thread_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrPayload {
    pub code:    String,
    pub message: String,
}

pub fn encode(frame: &Frame) -> Vec<u8> {
    rmp_serde::to_vec_named(frame).unwrap_or_default()
}

pub fn decode(bytes: &[u8]) -> Result<Frame, rmp_serde::decode::Error> {
    rmp_serde::from_slice(bytes)
}
