use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;

use domain::error::{DomainError, DomainResult};
use domain::ids::{MessageId, UserId};
use domain::ports::ReactionRepo;
use domain::reaction::Reaction;

use super::{map_sqlx_err, PgPool};

#[derive(Clone)]
pub struct PgReactionRepo(pub PgPool);

#[async_trait]
impl ReactionRepo for PgReactionRepo {
    async fn add(&self, message: MessageId, user: UserId, emoji: &str) -> DomainResult<Reaction> {
        let row = sqlx::query(
            r#"
            INSERT INTO reactions (message_id, user_id, emoji)
            VALUES ($1, $2, $3)
            ON CONFLICT (message_id, user_id, emoji) DO UPDATE SET created_at = reactions.created_at
            RETURNING message_id, user_id, emoji, created_at
            "#,
        )
        .bind(message.as_uuid())
        .bind(user.as_uuid())
        .bind(emoji)
        .fetch_one(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;

        Ok(Reaction {
            message_id: MessageId::from_uuid(row.try_get("message_id").map_err(map_sqlx_err)?),
            user_id:    UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_err)?),
            emoji:      row.try_get("emoji").map_err(map_sqlx_err)?,
            created_at: row.try_get::<DateTime<Utc>, _>("created_at").map_err(map_sqlx_err)?,
        })
    }

    async fn remove(&self, message: MessageId, user: UserId, emoji: &str) -> DomainResult<()> {
        let res = sqlx::query(
            r#"DELETE FROM reactions WHERE message_id = $1 AND user_id = $2 AND emoji = $3"#,
        )
        .bind(message.as_uuid())
        .bind(user.as_uuid())
        .bind(emoji)
        .execute(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound("reaction".into()));
        }
        Ok(())
    }

    async fn list_for_message(&self, message: MessageId) -> DomainResult<Vec<Reaction>> {
        let rows = sqlx::query(
            r#"SELECT message_id, user_id, emoji, created_at FROM reactions WHERE message_id = $1"#,
        )
        .bind(message.as_uuid())
        .fetch_all(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;

        rows.iter().map(|row| Ok(Reaction {
            message_id: MessageId::from_uuid(row.try_get("message_id").map_err(map_sqlx_err)?),
            user_id:    UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_err)?),
            emoji:      row.try_get("emoji").map_err(map_sqlx_err)?,
            created_at: row.try_get::<DateTime<Utc>, _>("created_at").map_err(map_sqlx_err)?,
        })).collect()
    }
}
