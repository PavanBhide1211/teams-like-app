use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresenceStatus {
    Online,
    Away,
    Dnd,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Presence {
    pub user_id:        UserId,
    pub status:         PresenceStatus,
    pub last_heartbeat: DateTime<Utc>,
}

impl PresenceStatus {
    /// Status the client may set explicitly. `Offline` is implicit via TTL
    /// expiry on the Redis key — clients cannot set it directly.
    pub fn is_client_settable(&self) -> bool {
        matches!(self, PresenceStatus::Online | PresenceStatus::Away | PresenceStatus::Dnd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_is_not_client_settable() {
        assert!(!PresenceStatus::Offline.is_client_settable());
    }
    #[test]
    fn online_is_client_settable() {
        assert!(PresenceStatus::Online.is_client_settable());
    }
}
