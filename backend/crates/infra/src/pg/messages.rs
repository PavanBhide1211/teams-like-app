use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use domain::error::{DomainError, DomainResult};
use domain::ids::{ChannelId, DmThreadId, MessageId, UserId};
use domain::message::{Message, MessageTarget, NewMessage};
use domain::ports::MessageRepo;

use super::{map_sqlx_err, PgPool};

#[derive(Clone)]
pub struct PgMessageRepo(pub PgPool);

fn row_to_message(row: &sqlx::postgres::PgRow) -> DomainResult<Message> {
    let channel_id: Option<Uuid> = row.try_get("channel_id").map_err(map_sqlx_err)?;
    let dm_thread_id: Option<Uuid> = row.try_get("dm_thread_id").map_err(map_sqlx_err)?;
    let target = match (channel_id, dm_thread_id) {
        (Some(c), None) => MessageTarget::Channel { channel_id: ChannelId::from_uuid(c) },
        (None, Some(d)) => MessageTarget::Dm      { dm_thread_id: DmThreadId::from_uuid(d) },
        _ => return Err(DomainError::Internal("message has invalid target shape".into())),
    };
    let parent: Option<Uuid> = row.try_get("parent_id").map_err(map_sqlx_err)?;
    let mentions: Vec<Uuid> = row.try_get("mentions").map_err(map_sqlx_err)?;
    Ok(Message {
        id:         MessageId::from_uuid(row.try_get("id").map_err(map_sqlx_err)?),
        target,
        parent_id:  parent.map(MessageId::from_uuid),
        author_id:  UserId::from_uuid(row.try_get("author_id").map_err(map_sqlx_err)?),
        body:       row.try_get("body").map_err(map_sqlx_err)?,
        mentions:   mentions.into_iter().map(UserId::from_uuid).collect(),
        edited_at:  row.try_get::<Option<DateTime<Utc>>, _>("edited_at").map_err(map_sqlx_err)?,
        created_at: row.try_get::<DateTime<Utc>, _>("created_at").map_err(map_sqlx_err)?,
        deleted_at: row.try_get::<Option<DateTime<Utc>>, _>("deleted_at").map_err(map_sqlx_err)?,
    })
}

#[async_trait]
impl MessageRepo for PgMessageRepo {
    async fn create(&self, new: NewMessage) -> DomainResult<Message> {
        let (channel_id, dm_thread_id) = match new.target {
            MessageTarget::Channel { channel_id } => (Some(channel_id.as_uuid()), None),
            MessageTarget::Dm      { dm_thread_id } => (None, Some(dm_thread_id.as_uuid())),
        };
        let mentions_uuid: Vec<Uuid> = new.mentions.iter().map(|u| u.as_uuid()).collect();

        let row = sqlx::query(
            r#"
            INSERT INTO messages (channel_id, dm_thread_id, parent_id, author_id, body, mentions)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, channel_id, dm_thread_id, parent_id, author_id, body, mentions,
                      edited_at, created_at, deleted_at
            "#,
        )
        .bind(channel_id)
        .bind(dm_thread_id)
        .bind(new.parent_id.map(|p| p.as_uuid()))
        .bind(new.author_id.as_uuid())
        .bind(&new.body)
        .bind(&mentions_uuid)
        .fetch_one(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;
        row_to_message(&row)
    }

    async fn by_id(&self, id: MessageId) -> DomainResult<Message> {
        let row = sqlx::query(
            r#"
            SELECT id, channel_id, dm_thread_id, parent_id, author_id, body, mentions,
                   edited_at, created_at, deleted_at
            FROM messages WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::NotFound("message".into()))?;
        row_to_message(&row)
    }

    async fn page_channel(&self, channel: ChannelId, before: Option<DateTime<Utc>>, limit: u32) -> DomainResult<Vec<Message>> {
        let limit = limit.clamp(1, 200) as i64;
        let rows = sqlx::query(
            r#"
            SELECT id, channel_id, dm_thread_id, parent_id, author_id, body, mentions,
                   edited_at, created_at, deleted_at
            FROM messages
            WHERE channel_id = $1 AND deleted_at IS NULL
              AND ($2::timestamptz IS NULL OR created_at < $2)
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(channel.as_uuid())
        .bind(before)
        .bind(limit)
        .fetch_all(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_message).collect()
    }

    async fn page_dm(&self, thread: DmThreadId, before: Option<DateTime<Utc>>, limit: u32) -> DomainResult<Vec<Message>> {
        let limit = limit.clamp(1, 200) as i64;
        let rows = sqlx::query(
            r#"
            SELECT id, channel_id, dm_thread_id, parent_id, author_id, body, mentions,
                   edited_at, created_at, deleted_at
            FROM messages
            WHERE dm_thread_id = $1 AND deleted_at IS NULL
              AND ($2::timestamptz IS NULL OR created_at < $2)
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(thread.as_uuid())
        .bind(before)
        .bind(limit)
        .fetch_all(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_message).collect()
    }

    async fn edit_body(&self, id: MessageId, by: UserId, body: &str, mentions: &[UserId]) -> DomainResult<Message> {
        let mentions_uuid: Vec<Uuid> = mentions.iter().map(|u| u.as_uuid()).collect();
        let row = sqlx::query(
            r#"
            UPDATE messages
            SET body = $3, mentions = $4, edited_at = now()
            WHERE id = $1 AND author_id = $2 AND deleted_at IS NULL
            RETURNING id, channel_id, dm_thread_id, parent_id, author_id, body, mentions,
                      edited_at, created_at, deleted_at
            "#,
        )
        .bind(id.as_uuid())
        .bind(by.as_uuid())
        .bind(body)
        .bind(&mentions_uuid)
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::Forbidden("not author or message gone".into()))?;
        row_to_message(&row)
    }

    async fn soft_delete(&self, id: MessageId, by: UserId) -> DomainResult<Message> {
        // Authorisation (author OR workspace admin) is enforced at the
        // application layer; this repo trusts its caller.
        let _ = by;
        let row = sqlx::query(
            r#"
            UPDATE messages SET deleted_at = now()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING id, channel_id, dm_thread_id, parent_id, author_id, body, mentions,
                      edited_at, created_at, deleted_at
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::NotFound("message".into()))?;
        row_to_message(&row)
    }

    async fn list_thread(&self, parent: MessageId) -> DomainResult<Vec<Message>> {
        let rows = sqlx::query(
            r#"
            SELECT id, channel_id, dm_thread_id, parent_id, author_id, body, mentions,
                   edited_at, created_at, deleted_at
            FROM messages
            WHERE parent_id = $1 AND deleted_at IS NULL
            ORDER BY created_at ASC
            "#,
        )
        .bind(parent.as_uuid())
        .fetch_all(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_message).collect()
    }
}
