//! Shared application state. Composition root for the dependency graph.

use std::sync::Arc;

use domain::ports::{
    ChannelRepo, DmRepo, EventBus, Hasher, MessageRepo, PresenceStore, ReactionRepo,
    TokenIssuer, UserRepo, WorkspaceRepo,
};
use infra::{
    clock::SystemClock,
    hasher::Argon2Hasher,
    pg::{
        channels::PgChannelRepo, dms::PgDmRepo, messages::PgMessageRepo,
        reactions::PgReactionRepo, users::PgUserRepo, workspaces::PgWorkspaceRepo,
    },
    redis_::{event_bus::RedisEventBus, presence::RedisPresenceStore, RedisClient},
    token::JwtIssuer,
    PgPool,
};

#[derive(Clone)]
pub struct AppState {
    pub users:     Arc<dyn UserRepo>,
    pub workspaces: Arc<dyn WorkspaceRepo>,
    pub channels:  Arc<dyn ChannelRepo>,
    pub dms:       Arc<dyn DmRepo>,
    pub messages:  Arc<dyn MessageRepo>,
    pub reactions: Arc<dyn ReactionRepo>,
    pub presence:  Arc<dyn PresenceStore>,
    pub bus:       Arc<dyn EventBus>,
    pub hasher:    Arc<dyn Hasher>,
    pub tokens:    Arc<dyn TokenIssuer>,
    pub clock:     Arc<SystemClock>,
}

impl AppState {
    pub fn build(pg: PgPool, redis: RedisClient, tokens: JwtIssuer) -> Self {
        Self {
            users:      Arc::new(PgUserRepo(pg.clone())),
            workspaces: Arc::new(PgWorkspaceRepo(pg.clone())),
            channels:   Arc::new(PgChannelRepo(pg.clone())),
            dms:        Arc::new(PgDmRepo(pg.clone())),
            messages:   Arc::new(PgMessageRepo(pg.clone())),
            reactions:  Arc::new(PgReactionRepo(pg)),
            presence:   Arc::new(RedisPresenceStore(redis.clone())),
            bus:        Arc::new(RedisEventBus(redis)),
            hasher:     Arc::new(Argon2Hasher::default()),
            tokens:     Arc::new(tokens),
            clock:      Arc::new(SystemClock),
        }
    }
}
