//! Domain errors. Every operation returns a `Result<T, DomainError>`. The
//! variants map cleanly to HTTP status codes (in `proto`) and to WS error
//! frames.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("invalid input: {0}")]
    Invalid(String),

    #[error("rate limited")]
    RateLimited,

    #[error("internal error: {0}")]
    Internal(String),
}

impl DomainError {
    /// Stable error code used over the wire. The client switches on this code,
    /// never on the message text.
    pub fn code(&self) -> &'static str {
        match self {
            DomainError::NotFound(_)    => "NotFound",
            DomainError::Forbidden(_)   => "Forbidden",
            DomainError::Conflict(_)    => "Conflict",
            DomainError::Invalid(_)     => "Invalid",
            DomainError::RateLimited    => "RateLimited",
            DomainError::Internal(_)    => "Internal",
        }
    }
}

pub type DomainResult<T> = Result<T, DomainError>;
