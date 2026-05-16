use axum::{
    extract::{Path, State},
    routing::{get, post}, Json, Router,
};

use domain::channel::validate_name;
use domain::error::DomainError;
use domain::ids::{ChannelId, WorkspaceId};
use proto::{ChannelDto, CreateChannelRequest};

use crate::{error::AppError, middleware::auth::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/workspaces/:wid/channels",          post(create).get(list))
        .route("/channels/:id",                      get(by_id))
}

async fn create(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(wid): Path<WorkspaceId>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<Json<ChannelDto>, AppError> {
    let _ = s.workspaces.membership(wid, uid).await?;
    validate_name(&req.name)?;
    let ch = s.channels.create(wid, &req.name, &req.topic, req.kind, uid).await?;
    Ok(Json(ch.into()))
}

async fn list(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(wid): Path<WorkspaceId>,
) -> Result<Json<Vec<ChannelDto>>, AppError> {
    let _ = s.workspaces.membership(wid, uid).await?;
    let all = s.channels.list_for_workspace(wid).await?;
    // Filter private channels the user isn't a member of.
    let mut visible = Vec::with_capacity(all.len());
    for c in all {
        match c.kind {
            domain::channel::ChannelKind::Public => visible.push(c),
            domain::channel::ChannelKind::Private => {
                if s.channels.is_member(c.id, uid).await? {
                    visible.push(c);
                }
            }
        }
    }
    Ok(Json(visible.into_iter().map(Into::into).collect()))
}

async fn by_id(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<ChannelId>,
) -> Result<Json<ChannelDto>, AppError> {
    let ch = s.channels.by_id(id).await?;
    if !s.channels.is_member(id, uid).await? {
        return Err(AppError(DomainError::Forbidden("not a member".into())));
    }
    Ok(Json(ch.into()))
}
