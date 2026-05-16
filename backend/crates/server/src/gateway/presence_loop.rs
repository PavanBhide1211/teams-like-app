//! Presence heartbeat — refreshes the user's Redis TTL while the WS is up.

use std::sync::Arc;
use std::time::Duration;

use domain::presence::PresenceStatus;
use tokio::time::interval;

use crate::state::AppState;

pub async fn run(state: AppState, ctx: Arc<super::ConnCtx>) {
    let _ = state.presence.touch(ctx.user_id, PresenceStatus::Online).await;
    let mut tick = interval(Duration::from_secs(15));
    tick.tick().await; // skip initial
    loop {
        tick.tick().await;
        if state.presence.touch(ctx.user_id, PresenceStatus::Online).await.is_err() {
            break;
        }
    }
}

/// Public starter so `main.rs` can call this without seeing internal types.
pub fn start_presence_heartbeat() { /* no-op; kept for API symmetry */ }
