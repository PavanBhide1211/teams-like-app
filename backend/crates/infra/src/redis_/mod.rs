//! Redis adapters: presence store + event bus.
//! Underscore in module name avoids the redis crate identifier clash.

pub mod presence;
pub mod event_bus;

use redis::aio::ConnectionManager;
use domain::error::{DomainError, DomainResult};

#[derive(Clone)]
pub struct RedisClient {
    pub conn: ConnectionManager,
}

impl RedisClient {
    pub async fn connect(url: &str) -> DomainResult<Self> {
        let client = redis::Client::open(url)
            .map_err(|e| DomainError::Internal(format!("redis open: {e}")))?;
        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| DomainError::Internal(format!("redis connect: {e}")))?;
        Ok(Self { conn })
    }
}
