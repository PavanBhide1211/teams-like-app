use axum::{extract::State, routing::post, Json, Router};

use domain::error::DomainError;
use domain::user::NewUser;
use proto::{AuthResponse, LoginRequest, RefreshRequest, RegisterRequest, UserDto};

use crate::{error::AppError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login",    post(login))
        .route("/auth/refresh",  post(refresh))
}

async fn register(
    State(s): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    if req.password.len() < 12 {
        return Err(AppError(DomainError::Invalid(
            "password must be at least 12 characters".into(),
        )));
    }
    if !req.email.contains('@') {
        return Err(AppError(DomainError::Invalid("invalid email".into())));
    }
    let hash = s.hasher.hash(&req.password)?;
    let user = s.users.create(NewUser {
        email: req.email,
        display_name: req.display_name,
        password_hash: hash,
    }).await?;
    issue_tokens(&s, user.into()).await
}

async fn login(
    State(s): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Constant-ish-time pattern: always verify against a hash, even on miss.
    let creds = match s.users.credentials_by_email(&req.email).await {
        Ok(c) => c,
        Err(_) => {
            // Run a dummy verify to keep timing similar; ignore result.
            let _ = s.hasher.verify(
                &req.password,
                "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            );
            return Err(AppError(DomainError::Forbidden("invalid credentials".into())));
        }
    };
    if !s.hasher.verify(&req.password, &creds.password_hash)? {
        return Err(AppError(DomainError::Forbidden("invalid credentials".into())));
    }
    let user = s.users.by_id(creds.user_id).await?;
    issue_tokens(&s, user.into()).await
}

async fn refresh(
    State(s): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user_id = s.tokens.verify_refresh(&req.refresh_token)?;
    let user = s.users.by_id(user_id).await?;
    issue_tokens(&s, user.into()).await
}

async fn issue_tokens(s: &AppState, user: UserDto) -> Result<Json<AuthResponse>, AppError> {
    let (access, access_exp)   = s.tokens.issue_access(user.id)?;
    let (refresh, refresh_exp) = s.tokens.issue_refresh(user.id)?;
    Ok(Json(AuthResponse {
        access_token: access,
        refresh_token: refresh,
        access_expires_at: access_exp,
        refresh_expires_at: refresh_exp,
        user,
    }))
}
