//! Map domain errors → HTTP responses with stable JSON shape.

use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use domain::error::DomainError;
use proto::ErrorBody;

pub struct AppError(pub DomainError);

impl From<DomainError> for AppError {
    fn from(e: DomainError) -> Self { Self(e) }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self.0 {
            DomainError::NotFound(m)  => (StatusCode::NOT_FOUND,  "NotFound",  m.clone()),
            DomainError::Forbidden(m) => (StatusCode::FORBIDDEN,  "Forbidden", m.clone()),
            DomainError::Conflict(m)  => (StatusCode::CONFLICT,   "Conflict",  m.clone()),
            DomainError::Invalid(m)   => (StatusCode::BAD_REQUEST,"Invalid",   m.clone()),
            DomainError::RateLimited  => (StatusCode::TOO_MANY_REQUESTS, "RateLimited", "rate limited".into()),
            DomainError::Internal(m)  => {
                tracing::error!(error = %m, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal", "internal error".into())
            }
        };
        let body = ErrorBody { code: code.into(), message };
        (status, Json(body)).into_response()
    }
}
