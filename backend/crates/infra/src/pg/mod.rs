//! Postgres adapters. The `PgPool` is a thin newtype over `sqlx::PgPool` so we
//! can attach helpers without polluting `sqlx` itself.

pub mod users;
pub mod workspaces;
pub mod channels;
pub mod dms;
pub mod messages;
pub mod reactions;

use sqlx::postgres::PgPoolOptions;
use domain::error::{DomainError, DomainResult};

#[derive(Clone)]
pub struct PgPool(pub sqlx::PgPool);

impl PgPool {
    pub async fn connect(url: &str) -> DomainResult<Self> {
        let inner = PgPoolOptions::new()
            .max_connections(8)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(url)
            .await
            .map_err(|e| DomainError::Internal(format!("postgres connect: {e}")))?;
        Ok(Self(inner))
    }

    pub async fn migrate(&self) -> DomainResult<()> {
        sqlx::migrate!("../../migrations")
            .run(&self.0)
            .await
            .map_err(|e| DomainError::Internal(format!("migrate: {e}")))?;
        Ok(())
    }
}

/// Convert a `sqlx::Error` into a `DomainError`. Centralised so handlers don't
/// need to know about the underlying driver.
pub fn map_sqlx_err(e: sqlx::Error) -> DomainError {
    use sqlx::Error::*;
    match &e {
        RowNotFound => DomainError::NotFound("row not found".into()),
        Database(db) => {
            // Postgres unique-violation -> Conflict; check_violation -> Invalid.
            match db.code().as_deref() {
                Some("23505") => DomainError::Conflict(db.message().to_string()),
                Some("23503") => DomainError::Invalid(format!("fk: {}", db.message())),
                Some("23514") => DomainError::Invalid(format!("check: {}", db.message())),
                _ => DomainError::Internal(format!("db: {e}")),
            }
        }
        _ => DomainError::Internal(format!("sqlx: {e}")),
    }
}
