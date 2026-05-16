use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use domain::channel::{Channel, ChannelKind};
use domain::error::{DomainError, DomainResult};
use domain::ids::{ChannelId, UserId, WorkspaceId};
use domain::ports::ChannelRepo;

use super::{map_sqlx_err, PgPool};

#[derive(Clone)]
pub struct PgChannelRepo(pub PgPool);

fn parse_kind(s: &str) -> DomainResult<ChannelKind> {
    match s {
        "public"  => Ok(ChannelKind::Public),
        "private" => Ok(ChannelKind::Private),
        other     => Err(DomainError::Internal(format!("bad channel kind: {other}"))),
    }
}

fn kind_str(k: ChannelKind) -> &'static str {
    match k { ChannelKind::Public => "public", ChannelKind::Private => "private" }
}

fn row_to_channel(row: &sqlx::postgres::PgRow) -> DomainResult<Channel> {
    Ok(Channel {
        id:           ChannelId::from_uuid(row.try_get("id").map_err(map_sqlx_err)?),
        workspace_id: WorkspaceId::from_uuid(row.try_get("workspace_id").map_err(map_sqlx_err)?),
        name:         row.try_get("name").map_err(map_sqlx_err)?,
        topic:        row.try_get("topic").map_err(map_sqlx_err)?,
        kind:         parse_kind(row.try_get::<&str, _>("kind").map_err(map_sqlx_err)?)?,
        created_by:   UserId::from_uuid(row.try_get("created_by").map_err(map_sqlx_err)?),
        created_at:   row.try_get::<DateTime<Utc>, _>("created_at").map_err(map_sqlx_err)?,
        updated_at:   row.try_get::<DateTime<Utc>, _>("updated_at").map_err(map_sqlx_err)?,
    })
}

#[async_trait]
impl ChannelRepo for PgChannelRepo {
    async fn create(&self, workspace: WorkspaceId, name: &str, topic: &str, kind: ChannelKind, created_by: UserId) -> DomainResult<Channel> {
        let row = sqlx::query(
            r#"
            INSERT INTO channels (workspace_id, name, topic, kind, created_by)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, workspace_id, name, topic, kind, created_by, created_at, updated_at
            "#,
        )
        .bind(workspace.as_uuid())
        .bind(name)
        .bind(topic)
        .bind(kind_str(kind))
        .bind(created_by.as_uuid())
        .fetch_one(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;
        row_to_channel(&row)
    }

    async fn by_id(&self, id: ChannelId) -> DomainResult<Channel> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, name, topic, kind, created_by, created_at, updated_at
            FROM channels WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::NotFound("channel".into()))?;
        row_to_channel(&row)
    }

    async fn list_for_workspace(&self, workspace: WorkspaceId) -> DomainResult<Vec<Channel>> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, name, topic, kind, created_by, created_at, updated_at
            FROM channels WHERE workspace_id = $1 AND deleted_at IS NULL
            ORDER BY name ASC
            "#,
        )
        .bind(workspace.as_uuid())
        .fetch_all(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_channel).collect()
    }

    async fn is_member(&self, channel: ChannelId, user: UserId) -> DomainResult<bool> {
        // Public channels: implicit membership via workspace.
        // Private channels: explicit row required.
        let kind: Option<String> = sqlx::query_scalar(
            r#"SELECT kind FROM channels WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(channel.as_uuid())
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;

        let Some(kind) = kind else {
            return Err(DomainError::NotFound("channel".into()));
        };

        if kind == "public" {
            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM memberships m
                JOIN channels c ON c.workspace_id = m.workspace_id
                WHERE c.id = $1 AND m.user_id = $2
                "#,
            )
            .bind(channel.as_uuid())
            .bind(user.as_uuid())
            .fetch_one(&self.0 .0)
            .await
            .map_err(map_sqlx_err)?;
            Ok(count > 0)
        } else {
            let count: i64 = sqlx::query_scalar(
                r#"SELECT COUNT(*) FROM channel_members WHERE channel_id = $1 AND user_id = $2"#,
            )
            .bind(channel.as_uuid())
            .bind(user.as_uuid())
            .fetch_one(&self.0 .0)
            .await
            .map_err(map_sqlx_err)?;
            Ok(count > 0)
        }
    }

    async fn add_member(&self, channel: ChannelId, user: UserId) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO channel_members (channel_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(channel.as_uuid())
        .bind(user.as_uuid())
        .execute(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn remove_member(&self, channel: ChannelId, user: UserId) -> DomainResult<()> {
        sqlx::query(
            r#"DELETE FROM channel_members WHERE channel_id = $1 AND user_id = $2"#,
        )
        .bind(channel.as_uuid())
        .bind(user.as_uuid())
        .execute(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn list_members(&self, channel: ChannelId) -> DomainResult<Vec<UserId>> {
        let rows = sqlx::query(
            r#"SELECT user_id FROM channel_members WHERE channel_id = $1"#,
        )
        .bind(channel.as_uuid())
        .fetch_all(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.iter().map(|r| {
            let id: Uuid = r.get("user_id");
            UserId::from_uuid(id)
        }).collect())
    }
}
