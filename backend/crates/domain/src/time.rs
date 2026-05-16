//! Time port. The domain never reads the wall clock directly; that's an IO
//! concern. Tests inject a fake `Clock` and assert deterministic timestamps.

use chrono::{DateTime, Utc};
use async_trait::async_trait;

#[async_trait]
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> DateTime<Utc>;
}
