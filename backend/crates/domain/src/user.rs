use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::UserId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id:            UserId,
    pub email:         String,        // canonicalised to lowercase
    pub display_name:  String,
    pub avatar_url:    Option<String>,
    pub created_at:    DateTime<Utc>,
    pub updated_at:    DateTime<Utc>,
}

/// Internal-only type used by the auth flow. NEVER leaves the backend.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub user_id:       UserId,
    pub password_hash: String,
}

/// Input to `UserRepo::create`.
#[derive(Debug, Clone)]
pub struct NewUser {
    pub email:         String,
    pub display_name:  String,
    pub password_hash: String,
}

impl User {
    /// Display fallback when no avatar is set: first two letters of name.
    pub fn initials(&self) -> String {
        self.display_name
            .split_whitespace()
            .take(2)
            .filter_map(|w| w.chars().next())
            .collect::<String>()
            .to_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(name: &str) -> User {
        User {
            id: UserId::new(),
            email: "x@y.eu".into(),
            display_name: name.into(),
            avatar_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn initials_two_word_name() {
        assert_eq!(make("Pavan Bhide").initials(), "PB");
    }

    #[test]
    fn initials_single_name() {
        assert_eq!(make("Pavan").initials(), "P");
    }

    #[test]
    fn initials_three_words_keeps_first_two() {
        assert_eq!(make("Pavan Madhukar Bhide").initials(), "PM");
    }
}
