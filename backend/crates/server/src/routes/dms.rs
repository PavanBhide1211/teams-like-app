use axum::{extract::State, routing::post, Json, Router};

use domain::error::DomainError;
use proto::{CreateDmRequest, DmThreadDto};

use crate::{error::AppError, middleware::auth::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dms", post(create_or_get).get(list_mine))
}

async fn create_or_get(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(req): Json<CreateDmRequest>,
) -> Result<Json<DmThreadDto>, AppError> {
    // Caller is always implicitly a member.
    let mut members = req.members;
    if !members.contains(&uid) { members.push(uid); }
    if members.len() < 2 {
        return Err(AppError(DomainError::Invalid("DM needs >= 2 distinct members".into())));
    }
    let t = s.dms.upsert_for_members(&members, uid).await?;
    Ok(Json(t.into()))
}

async fn list_mine(
    State(s): State<AppState>,
    AuthUser(uid): AuthUser,
) -> Result<Json<Vec<DmThreadDto>>, AppError> {
    let ts = s.dms.list_for_user(uid).await?;
    Ok(Json(ts.into_iter().map(Into::into).collect()))
}
