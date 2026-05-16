use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{ChannelId, DmThreadId, MessageId, UserId};
use crate::error::{DomainError, DomainResult};

/// Either-or target: a message belongs to exactly one of a channel or a DM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum MessageTarget {
    Channel { channel_id: ChannelId },
    Dm      { dm_thread_id: DmThreadId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id:            MessageId,
    pub target:        MessageTarget,
    pub parent_id:     Option<MessageId>,
    pub author_id:     UserId,
    pub body:          String,
    pub mentions:      Vec<UserId>,
    pub edited_at:     Option<DateTime<Utc>>,
    pub created_at:    DateTime<Utc>,
    pub deleted_at:    Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewMessage {
    pub target:    MessageTarget,
    pub parent_id: Option<MessageId>,
    pub author_id: UserId,
    pub body:      String,
    /// Server *re-parses* mentions from `body` and intersects with the channel
    /// membership; the array passed in here is treated as a hint. See
    /// `docs/03-threat-model.md` § T-2.
    pub mentions:  Vec<UserId>,
}

// --- validation ---

pub const MAX_BODY_BYTES:        usize = 8 * 1024;
pub const MAX_MENTIONS:          usize = 50;

pub fn validate_body(body: &str) -> DomainResult<()> {
    if body.trim().is_empty() {
        return Err(DomainError::Invalid("body must not be empty".into()));
    }
    if body.len() > MAX_BODY_BYTES {
        return Err(DomainError::Invalid(format!(
            "body too long: {} > {} bytes",
            body.len(),
            MAX_BODY_BYTES
        )));
    }
    Ok(())
}

pub fn validate_mentions(mentions: &[UserId]) -> DomainResult<()> {
    if mentions.len() > MAX_MENTIONS {
        return Err(DomainError::Invalid(format!(
            "too many mentions: {} > {}",
            mentions.len(),
            MAX_MENTIONS
        )));
    }
    Ok(())
}

impl Message {
    pub fn is_thread_reply(&self) -> bool {
        self.parent_id.is_some()
    }
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_body_rejected() {
        assert!(matches!(validate_body(""), Err(DomainError::Invalid(_))));
        assert!(matches!(validate_body("   "), Err(DomainError::Invalid(_))));
    }

    #[test]
    fn long_body_rejected() {
        let big = "x".repeat(MAX_BODY_BYTES + 1);
        assert!(matches!(validate_body(&big), Err(DomainError::Invalid(_))));
    }

    #[test]
    fn too_many_mentions_rejected() {
        let m: Vec<UserId> = (0..MAX_MENTIONS + 1).map(|_| UserId::new()).collect();
        assert!(matches!(validate_mentions(&m), Err(DomainError::Invalid(_))));
    }

    #[test]
    fn normal_body_ok() {
        assert!(validate_body("hello").is_ok());
    }
}
