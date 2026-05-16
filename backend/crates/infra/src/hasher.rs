//! Argon2id password hashing. Parameters tuned to OWASP defaults; configurable
//! via env. The encoded hash carries its own params so rotation is per-user.

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use async_trait::async_trait;
use rand_core::OsRng;

use domain::error::{DomainError, DomainResult};
use domain::ports::Hasher;

#[derive(Clone)]
pub struct Argon2Hasher {
    inner: Argon2<'static>,
}

impl Default for Argon2Hasher {
    fn default() -> Self {
        let m_kb = std::env::var("ARGON2_M_KB").ok().and_then(|s| s.parse().ok()).unwrap_or(19_456u32);
        let t    = std::env::var("ARGON2_T")    .ok().and_then(|s| s.parse().ok()).unwrap_or(2u32);
        let p    = std::env::var("ARGON2_P")    .ok().and_then(|s| s.parse().ok()).unwrap_or(1u32);
        let params = Params::new(m_kb, t, p, None).expect("argon2 params");
        Self { inner: Argon2::new(Algorithm::Argon2id, Version::V0x13, params) }
    }
}

#[async_trait]
impl Hasher for Argon2Hasher {
    fn hash(&self, plaintext: &str) -> DomainResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        self.inner
            .hash_password(plaintext.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| DomainError::Internal(format!("argon2 hash: {e}")))
    }

    fn verify(&self, plaintext: &str, encoded: &str) -> DomainResult<bool> {
        let parsed = PasswordHash::new(encoded)
            .map_err(|e| DomainError::Internal(format!("argon2 parse: {e}")))?;
        Ok(self
            .inner
            .verify_password(plaintext.as_bytes(), &parsed)
            .is_ok())
    }
}
