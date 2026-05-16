use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;

use domain::error::{DomainError, DomainResult};
use domain::ids::{UserId, WorkspaceId};
use domain::ports::WorkspaceRepo;
use domain::workspace::{Membership, Role, Workspace};

use super::{map_sqlx_err, PgPool};

#[derive(Clone)]
pub struct PgWorkspaceRepo(pub PgPool);

fn row_to_workspace(row: &sqlx::postgres::PgRow) -> DomainResult<Workspace> {
    Ok(Workspace {
        id:         WorkspaceId::from_uuid(row.try_get("id").map_err(map_sqlx_err)?),
        name:       row.try_get("name").map_err(map_sqlx_err)?,
        slug:       row.try_get("slug").map_err(map_sqlx_err)?,
        created_by: UserId::from_uuid(row.try_get("created_by").map_err(map_sqlx_err)?),
        created_at: row.try_get::<DateTime<Utc>, _>("created_at").map_err(map_sqlx_err)?,
        updated_at: row.try_get::<DateTime<Utc>, _>("updated_at").map_err(map_sqlx_err)?,
    })
}

fn parse_role(s: &str) -> DomainResult<Role> {
    match s {
        "owner"  => Ok(Role::Owner),
        "admin"  => Ok(Role::Admin),
        "member" => Ok(Role::Member),
        other    => Err(DomainError::Internal(format!("unknown role: {other}"))),
    }
}

fn role_str(r: Role) -> &'static str {
    match r {
        Role::Owner  => "owner",
        Role::Admin  => "admin",
        Role::Member => "member",
    }
}

#[async_trait]
impl WorkspaceRepo for PgWorkspaceRepo {
    async fn create(&self, name: &str, slug: &str, owner: UserId) -> DomainResult<Workspace> {
        let mut tx = self.0 .0.begin().await.map_err(map_sqlx_err)?;
        let row = sqlx::query(
            r#"
            INSERT INTO workspaces (name, slug, created_by)
            VALUES ($1, $2, $3)
            RETURNING id, name, slug, created_by, created_at, updated_at
            "#,
        )
        .bind(name)
        .bind(slug)
        .bind(owner.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // Creator is automatically the owner of the new workspace.
        sqlx::query(
            r#"
            INSERT INTO memberships (workspace_id, user_id, role)
            VALUES ($1, $2, 'owner')
            "#,
        )
        .bind(row.try_get::<uuid::Uuid, _>("id").map_err(map_sqlx_err)?)
        .bind(owner.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        row_to_workspace(&row)
    }

    async fn by_id(&self, id: WorkspaceId) -> DomainResult<Workspace> {
        let row = sqlx::query(
            r#"
            SELECT id, name, slug, created_by, created_at, updated_at
            FROM workspaces WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::NotFound("workspace".into()))?;
        row_to_workspace(&row)
    }

    async fn by_slug(&self, slug: &str) -> DomainResult<Workspace> {
        let row = sqlx::query(
            r#"
            SELECT id, name, slug, created_by, created_at, updated_at
            FROM workspaces WHERE slug = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::NotFound("workspace".into()))?;
        row_to_workspace(&row)
    }

    async fn list_for_user(&self, user: UserId) -> DomainResult<Vec<Workspace>> {
        let rows = sqlx::query(
            r#"
            SELECT w.id, w.name, w.slug, w.created_by, w.created_at, w.updated_at
            FROM workspaces w
            JOIN memberships m ON m.workspace_id = w.id
            WHERE m.user_id = $1 AND w.deleted_at IS NULL
            ORDER BY w.created_at ASC
            "#,
        )
        .bind(user.as_uuid())
        .fetch_all(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_workspace).collect()
    }

    async fn membership(&self, workspace: WorkspaceId, user: UserId) -> DomainResult<Membership> {
        let row = sqlx::query(
            r#"
            SELECT workspace_id, user_id, role, joined_at
            FROM memberships WHERE workspace_id = $1 AND user_id = $2
            "#,
        )
        .bind(workspace.as_uuid())
        .bind(user.as_uuid())
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::Forbidden("not a member".into()))?;

        Ok(Membership {
            workspace_id: WorkspaceId::from_uuid(row.try_get("workspace_id").map_err(map_sqlx_err)?),
            user_id:      UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_err)?),
            role:         parse_role(row.try_get::<&str, _>("role").map_err(map_sqlx_err)?)?,
            joined_at:    row.try_get::<DateTime<Utc>, _>("joined_at").map_err(map_sqlx_err)?,
        })
    }

    async fn add_member(&self, workspace: WorkspaceId, user: UserId, role: Role) -> DomainResult<Membership> {
        let row = sqlx::query(
            r#"
            INSERT INTO memberships (workspace_id, user_id, role)
            VALUES ($1, $2, $3)
            RETURNING workspace_id, user_id, role, joined_at
            "#,
        )
        .bind(workspace.as_uuid())
        .bind(user.as_uuid())
        .bind(role_str(role))
        .fetch_one(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;

        Ok(Membership {
            workspace_id: WorkspaceId::from_uuid(row.try_get("workspace_id").map_err(map_sqlx_err)?),
            user_id:      UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_err)?),
            role:         parse_role(row.try_get::<&str, _>("role").map_err(map_sqlx_err)?)?,
            joined_at:    row.try_get::<DateTime<Utc>, _>("joined_at").map_err(map_sqlx_err)?,
        })
    }

    async fn change_role(&self, workspace: WorkspaceId, user: UserId, role: Role) -> DomainResult<Membership> {
        // Refuse to demote the last owner.
        if !matches!(role, Role::Owner) {
            let owners: i64 = sqlx::query_scalar(
                r#"SELECT COUNT(*) FROM memberships WHERE workspace_id = $1 AND role = 'owner'"#,
            )
            .bind(workspace.as_uuid())
            .fetch_one(&self.0 .0)
            .await
            .map_err(map_sqlx_err)?;

            let target_is_owner: Option<String> = sqlx::query_scalar(
                r#"SELECT role FROM memberships WHERE workspace_id = $1 AND user_id = $2"#,
            )
            .bind(workspace.as_uuid())
            .bind(user.as_uuid())
            .fetch_optional(&self.0 .0)
            .await
            .map_err(map_sqlx_err)?;

            if owners <= 1 && target_is_owner.as_deref() == Some("owner") {
                return Err(DomainError::Conflict("cannot demote the last owner".into()));
            }
        }

        let row = sqlx::query(
            r#"
            UPDATE memberships SET role = $3
            WHERE workspace_id = $1 AND user_id = $2
            RETURNING workspace_id, user_id, role, joined_at
            "#,
        )
        .bind(workspace.as_uuid())
        .bind(user.as_uuid())
        .bind(role_str(role))
        .fetch_optional(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| DomainError::NotFound("membership".into()))?;

        Ok(Membership {
            workspace_id: WorkspaceId::from_uuid(row.try_get("workspace_id").map_err(map_sqlx_err)?),
            user_id:      UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_err)?),
            role:         parse_role(row.try_get::<&str, _>("role").map_err(map_sqlx_err)?)?,
            joined_at:    row.try_get::<DateTime<Utc>, _>("joined_at").map_err(map_sqlx_err)?,
        })
    }

    async fn list_members(&self, workspace: WorkspaceId) -> DomainResult<Vec<Membership>> {
        let rows = sqlx::query(
            r#"
            SELECT workspace_id, user_id, role, joined_at
            FROM memberships WHERE workspace_id = $1
            "#,
        )
        .bind(workspace.as_uuid())
        .fetch_all(&self.0 .0)
        .await
        .map_err(map_sqlx_err)?;

        rows.iter().map(|row| {
            Ok(Membership {
                workspace_id: WorkspaceId::from_uuid(row.try_get("workspace_id").map_err(map_sqlx_err)?),
                user_id:      UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_err)?),
                role:         parse_role(row.try_get::<&str, _>("role").map_err(map_sqlx_err)?)?,
                joined_at:    row.try_get::<DateTime<Utc>, _>("joined_at").map_err(map_sqlx_err)?,
            })
        }).collect()
    }
}
