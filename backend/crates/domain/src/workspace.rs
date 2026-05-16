use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{UserId, WorkspaceId};
use crate::error::{DomainError, DomainResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id:         WorkspaceId,
    pub name:       String,
    pub slug:       String,         // lower-cased, hyphenated
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Owner,
    Admin,
    Member,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Membership {
    pub workspace_id: WorkspaceId,
    pub user_id:      UserId,
    pub role:         Role,
    pub joined_at:    DateTime<Utc>,
}

/// Slug validation rules. Same logic the schema's UNIQUE INDEX assumes.
pub fn validate_slug(s: &str) -> DomainResult<()> {
    if s.is_empty() || s.len() > 64 {
        return Err(DomainError::Invalid("slug length must be 1..=64".into()));
    }
    if !s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(DomainError::Invalid(
            "slug must be lower-case ASCII, digits, or '-'".into(),
        ));
    }
    if s.starts_with('-') || s.ends_with('-') {
        return Err(DomainError::Invalid("slug must not start or end with '-'".into()));
    }
    // Reserved-slug allow-list (workspace squat prevention; threat S-4).
    const RESERVED: &[&str] = &["admin", "api", "auth", "www", "app", "root", "system"];
    if RESERVED.contains(&s) {
        return Err(DomainError::Conflict(format!("slug '{s}' is reserved")));
    }
    Ok(())
}

impl Role {
    pub fn can_admin(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }
    pub fn can_demote_owner(&self) -> bool {
        matches!(self, Role::Owner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_slugs() {
        assert!(validate_slug("my-org").is_ok());
        assert!(validate_slug("acme").is_ok());
        assert!(validate_slug("acme-2026").is_ok());
    }

    #[test]
    fn invalid_slug_uppercase() {
        assert!(matches!(validate_slug("MyOrg"), Err(DomainError::Invalid(_))));
    }

    #[test]
    fn invalid_slug_edges() {
        assert!(matches!(validate_slug("-acme"), Err(DomainError::Invalid(_))));
        assert!(matches!(validate_slug("acme-"), Err(DomainError::Invalid(_))));
    }

    #[test]
    fn invalid_slug_reserved() {
        assert!(matches!(validate_slug("admin"), Err(DomainError::Conflict(_))));
    }

    #[test]
    fn role_can_admin() {
        assert!(Role::Owner.can_admin());
        assert!(Role::Admin.can_admin());
        assert!(!Role::Member.can_admin());
    }
}
