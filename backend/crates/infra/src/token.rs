//! JWT issuance and verification (HS256).
//!
//! Access tokens: short-lived (24 h default). Refresh tokens: 30 d default,
//! rotated on use. We do not embed workspace memberships in the token; the
//! application layer re-fetches them per request so revocation takes effect
//! immediately.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use domain::error::{DomainError, DomainResult};
use domain::ids::UserId;
use domain::ports::TokenIssuer;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: Uuid,         // user id
    exp: i64,          // unix seconds
    typ: &'static str, // "access" or "refresh"
    iat: i64,
}

#[derive(Clone)]
pub struct JwtIssuer {
    encoding:    EncodingKey,
    decoding:    DecodingKey,
    access_ttl:  Duration,
    refresh_ttl: Duration,
}

impl JwtIssuer {
    pub fn from_env() -> DomainResult<Self> {
        let secret = std::env::var("JWT_SIGNING_KEY")
            .map_err(|_| DomainError::Internal("JWT_SIGNING_KEY must be set".into()))?;
        if secret.len() < 32 {
            return Err(DomainError::Internal(
                "JWT_SIGNING_KEY must be at least 32 bytes".into(),
            ));
        }
        let access_ttl = Duration::seconds(
            std::env::var("JWT_ACCESS_TTL_S").ok().and_then(|s| s.parse().ok()).unwrap_or(86_400),
        );
        let refresh_ttl = Duration::seconds(
            std::env::var("JWT_REFRESH_TTL_S").ok().and_then(|s| s.parse().ok()).unwrap_or(2_592_000),
        );
        Ok(Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            access_ttl,
            refresh_ttl,
        })
    }

    fn issue(&self, user: UserId, typ: &'static str, ttl: Duration)
        -> DomainResult<(String, DateTime<Utc>)>
    {
        let now = Utc::now();
        let exp = now + ttl;
        let claims = Claims {
            sub: user.as_uuid(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            typ,
        };
        let token = encode(&Header::default(), &claims, &self.encoding)
            .map_err(|e| DomainError::Internal(format!("jwt encode: {e}")))?;
        Ok((token, exp))
    }

    fn verify(&self, token: &str, expected_typ: &'static str) -> DomainResult<UserId> {
        let data = decode::<Claims>(token, &self.decoding, &Validation::default())
            .map_err(|_| DomainError::Forbidden("invalid token".into()))?;
        if data.claims.typ != expected_typ {
            return Err(DomainError::Forbidden("wrong token type".into()));
        }
        Ok(UserId::from_uuid(data.claims.sub))
    }
}

#[async_trait]
impl TokenIssuer for JwtIssuer {
    fn issue_access(&self, user: UserId) -> DomainResult<(String, DateTime<Utc>)> {
        self.issue(user, "access", self.access_ttl)
    }
    fn issue_refresh(&self, user: UserId) -> DomainResult<(String, DateTime<Utc>)> {
        self.issue(user, "refresh", self.refresh_ttl)
    }
    fn verify_access(&self, token: &str) -> DomainResult<UserId> {
        self.verify(token, "access")
    }
    fn verify_refresh(&self, token: &str) -> DomainResult<UserId> {
        self.verify(token, "refresh")
    }
}
