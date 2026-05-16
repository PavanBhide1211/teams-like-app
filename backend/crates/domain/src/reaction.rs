use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{MessageId, UserId};
use crate::error::{DomainError, DomainResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reaction {
    pub message_id: MessageId,
    pub user_id:    UserId,
    pub emoji:      String,
    pub created_at: DateTime<Utc>,
}

/// Reaction emoji validation. We allow a small allow-list of typical
/// single-codepoint emoji plus a max length to defend against arbitrary text.
pub fn validate_emoji(emoji: &str) -> DomainResult<()> {
    if emoji.is_empty() || emoji.chars().count() > 8 {
        return Err(DomainError::Invalid("emoji length must be 1..=8 chars".into()));
    }
    if emoji.chars().any(|c| c.is_ascii_control()) {
        return Err(DomainError::Invalid("emoji must not contain control chars".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typical_emoji_ok() {
        assert!(validate_emoji("👍").is_ok());
        assert!(validate_emoji("❤️").is_ok());
        assert!(validate_emoji("🎉").is_ok());
    }

    #[test]
    fn too_long_rejected() {
        assert!(matches!(validate_emoji(&"x".repeat(9)), Err(DomainError::Invalid(_))));
    }

    #[test]
    fn empty_rejected() {
        assert!(matches!(validate_emoji(""), Err(DomainError::Invalid(_))));
    }
}
