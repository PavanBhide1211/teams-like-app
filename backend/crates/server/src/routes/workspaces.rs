use axum::{
    extract::{Path, State},
    routing::{get, post}, Json, Router,
};

use domain::error::DomainError;
use domain::ids::WorkspaceId;
use domain::workspace::{Role, validate_slug};
use proto::{CreateWorkspaceRequest, MembershipDto, WorkspaceDto};

use crate::{error::AppError, middleware::auth::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/workspaces",                 post(create).get(list_mine))
        .route("/workspaces/:id",             get(by_id))
        .route("/workspaces/:id/members",     get(list_members))
}

async fn create(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Result<Json<WorkspaceDto>, AppError> {
    validate_slug(&req.slug)?;
    let ws = s.workspaces.create(&req.name, &req.slug, uid).await?;
    Ok(Json(ws.into()))
}

async fn list_mine(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
) -> Result<Json<Vec<WorkspaceDto>>, AppError> {
    let ws = s.workspaces.list_for_user(uid).await?;
    Ok(Json(ws.into_iter().map(Into::into).collect()))
}

async fn by_id(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<WorkspaceId>,
) -> Result<Json<WorkspaceDto>, AppError> {
    // Membership check.
    let _ = s.workspaces.membership(id, uid).await?;
    let ws = s.workspaces.by_id(id).await?;
    Ok(Json(ws.into()))
}

async fn list_members(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<WorkspaceId>,
) -> Result<Json<Vec<MembershipDto>>, AppError> {
    let me = s.workspaces.membership(id, uid).await?;
    let _ = me; // any member can read
    let ms = s.workspaces.list_members(id).await?;
    Ok(Json(ms.into_iter().map(Into::into).collect()))
}

// (Role-change endpoint omitted in this scaffold for brevity; the repo method
// already enforces the "last-owner" rule. Add a PATCH /workspaces/:id/members/:user
// route on Day 3 polish if needed.)
#[allow(dead_code)]
fn _unused(_: Role) {}
