use async_trait::async_trait;
use redis::AsyncCommands;

use domain::error::{DomainError, DomainResult};
use domain::ids::{ChannelId, DmThreadId, UserId};
use domain::ports::EventBus;

use super::RedisClient;

#[derive(Clone)]
pub struct RedisEventBus(pub RedisClient);

#[async_trait]
impl EventBus for RedisEventBus {
    async fn publish_channel(&self, channel: ChannelId, payload: &[u8]) -> DomainResult<()> {
        let mut c = self.0.conn.clone();
        let _: i64 = c
            .publish(format!("ws:fanout:channel:{channel}"), payload)
            .await
            .map_err(|e| DomainError::Internal(format!("redis publish: {e}")))?;
        Ok(())
    }

    async fn publish_dm(&self, thread: DmThreadId, payload: &[u8]) -> DomainResult<()> {
        let mut c = self.0.conn.clone();
        let _: i64 = c
            .publish(format!("ws:fanout:dm:{thread}"), payload)
            .await
            .map_err(|e| DomainError::Internal(format!("redis publish: {e}")))?;
        Ok(())
    }

    async fn publish_user(&self, user: UserId, payload: &[u8]) -> DomainResult<()> {
        let mut c = self.0.conn.clone();
        let _: i64 = c
            .publish(format!("ws:fanout:user:{user}"), payload)
            .await
            .map_err(|e| DomainError::Internal(format!("redis publish: {e}")))?;
        Ok(())
    }
}
