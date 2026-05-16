//! Domain layer — pure types, business rules, and ports.
//!
//! INVARIANT: this crate must never depend on tokio, sqlx, axum, redis, or any
//! other IO library. Doing so is an architecture violation. The whole point of
//! the hexagonal layout is that this crate is unit-testable in milliseconds
//! with no fixtures.

pub mod error;
pub mod ids;
pub mod time;

pub mod user;
pub mod workspace;
pub mod channel;
pub mod dm;
pub mod message;
pub mod reaction;
pub mod presence;

pub mod ports;

pub use error::*;
pub use ids::*;
