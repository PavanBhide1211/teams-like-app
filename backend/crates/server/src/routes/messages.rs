use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, patch, post}, Json, Router,
};

use domain::error::DomainError;
use domain::ids::{ChannelId, DmThreadId, MessageId};
use domain::message::{validate_body, validate_mentions, MessageTarget, NewMessage};
use proto::{CreateMessageRequest, EditMessageRequest, MessageDto, PageQuery};

use crate::{error::AppError, middleware::auth::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/messages",                                  post(create))
        .route("/messages/:id",                              patch(edit).delete(soft_delete))
        .route("/channels/:id/messages",                     get(page_channel))
        .route("/dms/:id/messages",                          get(page_dm))
        .route("/messages/:id/thread",                       get(list_thread))
}

async fn create(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(req): Json<CreateMessageRequest>,
) -> Result<Json<MessageDto>, AppError> {
    validate_body(&req.body)?;
    validate_mentions(&req.mentions)?;

    let target = match (req.channel_id, req.dm_thread_id) {
        (Some(c), None) => {
            if !s.channels.is_member(c, uid).await? {
                return Err(AppError(DomainError::Forbidden("not a channel member".into())));
            }
            MessageTarget::Channel { channel_id: c }
        }
        (None, Some(d)) => {
            if !s.dms.is_member(d, uid).await? {
                return Err(AppError(DomainError::Forbidden("not in this DM".into())));
            }
            MessageTarget::Dm { dm_thread_id: d }
        }
        _ => return Err(AppError(DomainError::Invalid(
            "exactly one of channel_id, dm_thread_id required".into()
        ))),
    };

    let m = s.messages.create(NewMessage {
        target, parent_id: req.parent_id, author_id: uid,
        body: req.body, mentions: req.mentions,
    }).await?;

    // Fire WS event (msgpack-encoded MessageDto).
    let dto = MessageDto::from(m.clone());
    if let Ok(buf) = rmp_serde::to_vec_named(&dto) {
        match m.target {
            MessageTarget::Channel { channel_id } => {
                let _ = s.bus.publish_channel(channel_id, &buf).await;
            }
            MessageTarget::Dm { dm_thread_id } => {
                let _ = s.bus.publish_dm(dm_thread_id, &buf).await;
            }
        }
    }
    Ok(Json(dto))
}

async fn edit(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<MessageId>,
    Json(req): Json<EditMessageRequest>,
) -> Result<Json<MessageDto>, AppError> {
    validate_body(&req.body)?;
    validate_mentions(&req.mentions)?;
    let m = s.messages.edit_body(id, uid, &req.body, &req.mentions).await?;
    Ok(Json(m.into()))
}

async fn soft_delete(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<MessageId>,
) -> Result<Json<MessageDto>, AppError> {
    let existing = s.messages.by_id(id).await?;
    if existing.author_id != uid {
        // Workspace-admin override would go here. Lean demo: author-only.
        return Err(AppError(DomainError::Forbidden("only the author can delete".into())));
    }
    let m = s.messages.soft_delete(id, uid).await?;
    Ok(Json(m.into()))
}

async fn page_channel(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<ChannelId>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Vec<MessageDto>>, AppError> {
    if !s.channels.is_member(id, uid).await? {
        return Err(AppError(DomainError::Forbidden("not a channel member".into())));
    }
    let ms = s.messages.page_channel(id, q.before, q.limit.unwrap_or(50)).await?;
    Ok(Json(ms.into_iter().map(Into::into).collect()))
}

async fn page_dm(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<DmThreadId>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Vec<MessageDto>>, AppError> {
    if !s.dms.is_member(id, uid).await? {
        return Err(AppError(DomainError::Forbidden("not in this DM".into())));
    }
    let ms = s.messages.page_dm(id, q.before, q.limit.unwrap_or(50)).await?;
    Ok(Json(ms.into_iter().map(Into::into).collect()))
}

async fn list_thread(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<MessageId>,
) -> Result<Json<Vec<MessageDto>>, AppError> {
    // Authorization: must be able to see the parent.
    let parent = s.messages.by_id(id).await?;
    match parent.target {
        MessageTarget::Channel { channel_id } => {
            if !s.channels.is_member(channel_id, uid).await? {
                return Err(AppError(DomainError::Forbidden("not a channel member".into())));
            }
        }
        MessageTarget::Dm { dm_thread_id } => {
            if !s.dms.is_member(dm_thread_id, uid).await? {
                return Err(AppError(DomainError::Forbidden("not in this DM".into())));
            }
        }
    }
    let ms = s.messages.list_thread(id).await?;
    Ok(Json(ms.into_iter().map(Into::into).collect()))
}

// (Reactions routes co-located here for brevity; the trait lives on `s.reactions`.)
pub fn reactions_router() -> Router<AppState> {
    Router::new()
        .route("/messages/:id/reactions",  post(add_reaction).get(list_reactions))
        .route("/messages/:id/reactions/:emoji", delete(remove_reaction))
}

async fn add_reaction(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<MessageId>,
    Json(req): Json<proto::ReactionRequest>,
) -> Result<Json<proto::ReactionDto>, AppError> {
    domain::reaction::validate_emoji(&req.emoji)?;
    let r = s.reactions.add(id, uid, &req.emoji).await?;
    Ok(Json(r.into()))
}

async fn remove_reaction(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Path((id, emoji)): Path<(MessageId, String)>,
) -> Result<axum::http::StatusCode, AppError> {
    s.reactions.remove(id, uid, &emoji).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn list_reactions(
    State(s): State<AppState>,
    AuthUser(_uid): AuthUser,
    Path(id): Path<MessageId>,
) -> Result<Json<Vec<proto::ReactionDto>>, AppError> {
    let rs = s.reactions.list_for_message(id).await?;
    Ok(Json(rs.into_iter().map(Into::into).collect()))
}
