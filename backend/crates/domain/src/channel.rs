use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{ChannelId, UserId, WorkspaceId};
use crate::error::{DomainError, DomainResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelKind {
    Public,
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    pub id:           ChannelId,
    pub workspace_id: WorkspaceId,
    pub name:         String,
    pub topic:        String,
    pub kind:         ChannelKind,
    pub created_by:   UserId,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}

/// Channel-name validation. Looser than workspace-slug but still constrained
/// so URLs and tab labels stay sane.
pub fn validate_name(s: &str) -> DomainResult<()> {
    if s.is_empty() || s.len() > 80 {
        return Err(DomainError::Invalid("channel name length must be 1..=80".into()));
    }
    if s.chars().any(|c| c == '\n' || c == '\r' || c == '\t') {
        return Err(DomainError::Invalid("channel name must not contain newlines or tabs".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_name() {
        assert!(validate_name("general").is_ok());
        assert!(validate_name("Project Falcon").is_ok());
        assert!(validate_name("🚀-launch-2026").is_ok());
    }

    #[test]
    fn invalid_name_empty() {
        assert!(matches!(validate_name(""), Err(DomainError::Invalid(_))));
    }

    #[test]
    fn invalid_name_with_newline() {
        assert!(matches!(validate_name("a\nb"), Err(DomainError::Invalid(_))));
    }
}
