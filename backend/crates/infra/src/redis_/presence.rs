use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use redis::AsyncCommands;

use domain::error::{DomainError, DomainResult};
use domain::ids::UserId;
use domain::ports::PresenceStore;
use domain::presence::{Presence, PresenceStatus};

use super::RedisClient;

const TTL_SECONDS: u64 = 45;

fn key(user: UserId) -> String { format!("presence:{user}") }

fn status_str(s: PresenceStatus) -> &'static str {
    match s {
        PresenceStatus::Online  => "online",
        PresenceStatus::Away    => "away",
        PresenceStatus::Dnd     => "dnd",
        PresenceStatus::Offline => "offline",
    }
}

fn parse_status(s: &str) -> PresenceStatus {
    match s {
        "online" => PresenceStatus::Online,
        "away"   => PresenceStatus::Away,
        "dnd"    => PresenceStatus::Dnd,
        _        => PresenceStatus::Offline,
    }
}

#[derive(Clone)]
pub struct RedisPresenceStore(pub RedisClient);

#[async_trait]
impl PresenceStore for RedisPresenceStore {
    async fn touch(&self, user: UserId, status: PresenceStatus) -> DomainResult<()> {
        let mut c = self.0.conn.clone();
        let now = Utc::now().timestamp();
        let value = format!("{}|{}", status_str(status), now);
        let _: () = c.set_ex(key(user), value, TTL_SECONDS)
            .await
            .map_err(|e| DomainError::Internal(format!("redis set_ex: {e}")))?;
        Ok(())
    }

    async fn get(&self, user: UserId) -> DomainResult<Presence> {
        let mut c = self.0.conn.clone();
        let v: Option<String> = c.get(key(user))
            .await
            .map_err(|e| DomainError::Internal(format!("redis get: {e}")))?;
        match v {
            Some(s) => {
                let (st, ts) = s.split_once('|').unwrap_or(("offline", "0"));
                let secs: i64 = ts.parse().unwrap_or(0);
                let last = Utc.timestamp_opt(secs, 0).single().unwrap_or_else(Utc::now);
                Ok(Presence { user_id: user, status: parse_status(st), last_heartbeat: last })
            }
            None => Ok(Presence {
                user_id: user,
                status: PresenceStatus::Offline,
                last_heartbeat: DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default(),
            }),
        }
    }

    async fn list(&self, users: &[UserId]) -> DomainResult<Vec<Presence>> {
        let mut out = Vec::with_capacity(users.len());
        for u in users {
            out.push(self.get(*u).await?);
        }
        Ok(out)
    }
}
