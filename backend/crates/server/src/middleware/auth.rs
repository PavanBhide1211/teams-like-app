//! Bearer-JWT extractor. Adds the authenticated UserId to request extensions.

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::IntoResponse,
};

use domain::ids::UserId;
use crate::state::AppState;

pub struct AuthUser(pub UserId);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header = parts.headers.get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "missing Authorization"))?;
        let token = header.strip_prefix("Bearer ")
            .ok_or((StatusCode::UNAUTHORIZED, "expected 'Bearer ' prefix"))?;
        let id = state.tokens.verify_access(token)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid token"))?;
        Ok(AuthUser(id))
    }
}

pub fn unauthorized<E>(_: E) -> (StatusCode, &'static str) {
    (StatusCode::UNAUTHORIZED, "unauthorized")
}

impl IntoResponse for AuthUser {
    fn into_response(self) -> axum::response::Response {
        // Not used directly; type implements FromRequestParts only.
        (StatusCode::OK, self.0.to_string()).into_response()
    }
}
