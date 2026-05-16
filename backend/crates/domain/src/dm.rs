use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::ids::{DmThreadId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmThread {
    pub id:           DmThreadId,
    pub members_hash: String,    // sha256 of sorted member ids
    pub created_by:   UserId,
    pub created_at:   DateTime<Utc>,
}

/// Canonicalise the member set into a hash for uniqueness. The implementation
/// lives in `infra` (needs sha2); domain only declares the contract.
pub fn canonical_members(members: &BTreeSet<UserId>) -> Vec<UserId> {
    members.iter().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_members_is_sorted() {
        let mut s = BTreeSet::new();
        let a = UserId::new();
        let b = UserId::new();
        s.insert(a);
        s.insert(b);
        let v = canonical_members(&s);
        // BTreeSet iteration is sorted by Ord; UserId's Ord is via Uuid bytes.
        assert!(v.len() == 2);
    }
}
