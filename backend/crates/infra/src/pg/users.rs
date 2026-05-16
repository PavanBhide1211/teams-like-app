use async_trait::async_trait;
use domain::error::{DomainError, DomainResult};
use domain::ids::UserId;
use domain::ports::UserRepo;
use domain::user::{Credentials, NewUser, User};

use super::{map_sqlx_err, PgPool};

#[derive(Clone)]
pub struct PgUserRepo(pub PgPool);

#[async_trait]
impl UserRepo for PgUserRepo {
    async fn create(&self, new: NewUser) -> DomainResult<User> {
        let row = sqlx::query!(
            r#"
            INSERT INTO users (email, display_name, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id, email::text AS "email!", display_name, avatar_url,
                      created_at, updated_at
            "#,
            new.email.to_lowercase(),
            new.display_name,
            new.password_hash,
        )
        .fetch_one(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;

        Ok(User {
            id: UserId::from_uuid(row.id),
            email: row.email,
            display_name: row.display_name,
            avatar_url: row.avatar_url,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn by_id(&self, id: UserId) -> DomainResult<User> {
        let row = sqlx::query!(
            r#"
            SELECT id, email::text AS "email!", display_name, avatar_url,
                   created_at, updated_at
            FROM users
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            id.as_uuid(),
        )
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::NotFound("user".into()))?;

        Ok(User {
            id: UserId::from_uuid(row.id),
            email: row.email,
            display_name: row.display_name,
            avatar_url: row.avatar_url,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn by_email(&self, email: &str) -> DomainResult<User> {
        let row = sqlx::query!(
            r#"
            SELECT id, email::text AS "email!", display_name, avatar_url,
                   created_at, updated_at
            FROM users
            WHERE email = $1::citext AND deleted_at IS NULL
            "#,
            email,
        )
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::NotFound("user".into()))?;

        Ok(User {
            id: UserId::from_uuid(row.id),
            email: row.email,
            display_name: row.display_name,
            avatar_url: row.avatar_url,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn credentials_by_email(&self, email: &str) -> DomainResult<Credentials> {
        let row = sqlx::query!(
            r#"
            SELECT id, password_hash
            FROM users
            WHERE email = $1::citext AND deleted_at IS NULL
            "#,
            email,
        )
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::NotFound("user".into()))?;

        Ok(Credentials {
            user_id: UserId::from_uuid(row.id),
            password_hash: row.password_hash,
        })
    }
}
