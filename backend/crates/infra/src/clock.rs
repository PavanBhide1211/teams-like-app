use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::time::Clock;

#[derive(Default, Clone)]
pub struct SystemClock;

#[async_trait]
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> { Utc::now() }
}
