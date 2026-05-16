use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use domain::dm::DmThread;
use domain::error::{DomainError, DomainResult};
use domain::ids::{DmThreadId, UserId};
use domain::ports::DmRepo;

use super::{map_sqlx_err, PgPool};

#[derive(Clone)]
pub struct PgDmRepo(pub PgPool);

fn members_hash(members: &[UserId]) -> String {
    let mut sorted: Vec<Uuid> = members.iter().map(|u| u.as_uuid()).collect();
    sorted.sort();
    sorted.dedup();
    let mut h = Sha256::new();
    for u in &sorted {
        h.update(u.as_bytes());
    }
    hex::encode(h.finalize())
}

#[async_trait]
impl DmRepo for PgDmRepo {
    async fn upsert_for_members(&self, members: &[UserId], created_by: UserId) -> DomainResult<DmThread> {
        if members.len() < 2 {
            return Err(DomainError::Invalid("DM thread must have at least 2 members".into()));
        }
        let hash = members_hash(members);
        let mut tx = self.0 .0.begin().await.map_err(map_sqlx_err)?;

        // Try existing.
        if let Some(row) = sqlx::query(
            r#"
            SELECT id, members_hash, created_by, created_at
            FROM dm_threads WHERE members_hash = $1
            "#,
        )
        .bind(&hash)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_err)?
        {
            tx.commit().await.map_err(map_sqlx_err)?;
            return Ok(DmThread {
                id:           DmThreadId::from_uuid(row.try_get("id").map_err(map_sqlx_err)?),
                members_hash: row.try_get("members_hash").map_err(map_sqlx_err)?,
                created_by:   UserId::from_uuid(row.try_get("created_by").map_err(map_sqlx_err)?),
                created_at:   row.try_get::<DateTime<Utc>, _>("created_at").map_err(map_sqlx_err)?,
            });
        }

        // Create.
        let row = sqlx::query(
            r#"
            INSERT INTO dm_threads (members_hash, created_by)
            VALUES ($1, $2)
            RETURNING id, members_hash, created_by, created_at
            "#,
        )
        .bind(&hash)
        .bind(created_by.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let id: Uuid = row.try_get("id").map_err(map_sqlx_err)?;
        for m in members {
            sqlx::query(r#"INSERT INTO dm_members (dm_thread_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"#)
                .bind(id)
                .bind(m.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;
        }
        tx.commit().await.map_err(map_sqlx_err)?;

        Ok(DmThread {
            id:           DmThreadId::from_uuid(id),
            members_hash: row.try_get("members_hash").map_err(map_sqlx_err)?,
            created_by:   UserId::from_uuid(row.try_get("created_by").map_err(map_sqlx_err)?),
            created_at:   row.try_get::<DateTime<Utc>, _>("created_at").map_err(map_sqlx_err)?,
        })
    }

    async fn by_id(&self, id: DmThreadId) -> DomainResult<DmThread> {
        let row = sqlx::query(
            r#"SELECT id, members_hash, created_by, created_at FROM dm_threads WHERE id = $1"#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::NotFound("dm_thread".into()))?;

        Ok(DmThread {
            id:           DmThreadId::from_uuid(row.try_get("id").map_err(map_sqlx_err)?),
            members_hash: row.try_get("members_hash").map_err(map_sqlx_err)?,
            created_by:   UserId::from_uuid(row.try_get("created_by").map_err(map_sqlx_err)?),
            created_at:   row.try_get::<DateTime<Utc>, _>("created_at").map_err(map_sqlx_err)?,
        })
    }

    async fn is_member(&self, thread: DmThreadId, user: UserId) -> DomainResult<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM dm_members WHERE dm_thread_id = $1 AND user_id = $2"#,
        )
        .bind(thread.as_uuid())
        .bind(user.as_uuid())
        .fetch_one(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;
        Ok(count > 0)
    }

    async fn list_for_user(&self, user: UserId) -> DomainResult<Vec<DmThread>> {
        let rows = sqlx::query(
            r#"
            SELECT d.id, d.members_hash, d.created_by, d.created_at
            FROM dm_threads d
            JOIN dm_members m ON m.dm_thread_id = d.id
            WHERE m.user_id = $1
            ORDER BY d.created_at DESC
            "#,
        )
        .bind(user.as_uuid())
        .fetch_all(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;

        rows.iter().map(|row| Ok(DmThread {
            id:           DmThreadId::from_uuid(row.try_get("id").map_err(map_sqlx_err)?),
            members_hash: row.try_get("members_hash").map_err(map_sqlx_err)?,
            created_by:   UserId::from_uuid(row.try_get("created_by").map_err(map_sqlx_err)?),
            created_at:   row.try_get::<DateTime<Utc>, _>("created_at").map_err(map_sqlx_err)?,
        })).collect()
    }

    async fn list_members(&self, thread: DmThreadId) -> DomainResult<Vec<UserId>> {
        let rows = sqlx::query(r#"SELECT user_id FROM dm_members WHERE dm_thread_id = $1"#)
            .bind(thread.as_uuid())
            .fetch_all(&self.0 .0)
            .await
            .map_err(map_sqlx_err)?;
        Ok(rows.iter().map(|r| UserId::from_uuid(r.get("user_id"))).collect())
    }
}
