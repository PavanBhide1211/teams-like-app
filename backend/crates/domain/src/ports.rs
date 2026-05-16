//! Ports — the traits that the application layer depends on.
//!
//! Adapters in the `infra` crate implement these. Tests can plug in in-memory
//! fakes without any IO.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::channel::{Channel, ChannelKind};
use crate::dm::DmThread;
use crate::error::DomainResult;
use crate::ids::{ChannelId, DmThreadId, MessageId, UserId, WorkspaceId};
use crate::message::{Message, NewMessage};
use crate::presence::{Presence, PresenceStatus};
use crate::reaction::Reaction;
use crate::user::{Credentials, NewUser, User};
use crate::workspace::{Membership, Role, Workspace};

// ---------- users ----------
#[async_trait]
pub trait UserRepo: Send + Sync + 'static {
    async fn create(&self, new: NewUser) -> DomainResult<User>;
    async fn by_id(&self, id: UserId) -> DomainResult<User>;
    async fn by_email(&self, email: &str) -> DomainResult<User>;
    /// Used only by the auth flow; the returned hash never leaves the backend.
    async fn credentials_by_email(&self, email: &str) -> DomainResult<Credentials>;
}

// ---------- workspaces ----------
#[async_trait]
pub trait WorkspaceRepo: Send + Sync + 'static {
    async fn create(&self, name: &str, slug: &str, owner: UserId) -> DomainResult<Workspace>;
    async fn by_id(&self, id: WorkspaceId) -> DomainResult<Workspace>;
    async fn by_slug(&self, slug: &str) -> DomainResult<Workspace>;
    async fn list_for_user(&self, user: UserId) -> DomainResult<Vec<Workspace>>;

    async fn membership(&self, workspace: WorkspaceId, user: UserId) -> DomainResult<Membership>;
    async fn add_member(&self, workspace: WorkspaceId, user: UserId, role: Role) -> DomainResult<Membership>;
    async fn change_role(&self, workspace: WorkspaceId, user: UserId, role: Role) -> DomainResult<Membership>;
    async fn list_members(&self, workspace: WorkspaceId) -> DomainResult<Vec<Membership>>;
}

// ---------- channels ----------
#[async_trait]
pub trait ChannelRepo: Send + Sync + 'static {
    async fn create(&self, workspace: WorkspaceId, name: &str, topic: &str, kind: ChannelKind, created_by: UserId) -> DomainResult<Channel>;
    async fn by_id(&self, id: ChannelId) -> DomainResult<Channel>;
    async fn list_for_workspace(&self, workspace: WorkspaceId) -> DomainResult<Vec<Channel>>;

    async fn is_member(&self, channel: ChannelId, user: UserId) -> DomainResult<bool>;
    async fn add_member(&self, channel: ChannelId, user: UserId) -> DomainResult<()>;
    async fn remove_member(&self, channel: ChannelId, user: UserId) -> DomainResult<()>;
    async fn list_members(&self, channel: ChannelId) -> DomainResult<Vec<UserId>>;
}

// ---------- DMs ----------
#[async_trait]
pub trait DmRepo: Send + Sync + 'static {
    /// Find-or-create a DM thread for a canonical (sorted, deduped) member set.
    async fn upsert_for_members(&self, members: &[UserId], created_by: UserId) -> DomainResult<DmThread>;
    async fn by_id(&self, id: DmThreadId) -> DomainResult<DmThread>;
    async fn is_member(&self, thread: DmThreadId, user: UserId) -> DomainResult<bool>;
    async fn list_for_user(&self, user: UserId) -> DomainResult<Vec<DmThread>>;
    async fn list_members(&self, thread: DmThreadId) -> DomainResult<Vec<UserId>>;
}

// ---------- messages ----------
#[async_trait]
pub trait MessageRepo: Send + Sync + 'static {
    async fn create(&self, new: NewMessage) -> DomainResult<Message>;
    async fn by_id(&self, id: MessageId) -> DomainResult<Message>;

    /// Latest N messages in a channel before `before` (exclusive).
    async fn page_channel(&self, channel: ChannelId, before: Option<DateTime<Utc>>, limit: u32) -> DomainResult<Vec<Message>>;
    async fn page_dm(&self, thread: DmThreadId, before: Option<DateTime<Utc>>, limit: u32) -> DomainResult<Vec<Message>>;

    async fn edit_body(&self, id: MessageId, by: UserId, body: &str, mentions: &[UserId]) -> DomainResult<Message>;
    async fn soft_delete(&self, id: MessageId, by: UserId) -> DomainResult<Message>;
    async fn list_thread(&self, parent: MessageId) -> DomainResult<Vec<Message>>;
}

// ---------- reactions ----------
#[async_trait]
pub trait ReactionRepo: Send + Sync + 'static {
    async fn add(&self, message: MessageId, user: UserId, emoji: &str) -> DomainResult<Reaction>;
    async fn remove(&self, message: MessageId, user: UserId, emoji: &str) -> DomainResult<()>;
    async fn list_for_message(&self, message: MessageId) -> DomainResult<Vec<Reaction>>;
}

// ---------- presence ----------
#[async_trait]
pub trait PresenceStore: Send + Sync + 'static {
    /// Refresh the user's TTL. Called on connect and on every heartbeat.
    async fn touch(&self, user: UserId, status: PresenceStatus) -> DomainResult<()>;
    async fn get(&self, user: UserId) -> DomainResult<Presence>;
    async fn list(&self, users: &[UserId]) -> DomainResult<Vec<Presence>>;
}

// ---------- event bus ----------
/// Cross-node pub/sub. The WS gateway publishes domain events; every gateway
/// node subscribes and pushes events to its connected clients.
#[async_trait]
pub trait EventBus: Send + Sync + 'static {
    async fn publish_channel(&self, channel: ChannelId, payload: &[u8]) -> DomainResult<()>;
    async fn publish_dm(&self, thread: DmThreadId, payload: &[u8]) -> DomainResult<()>;
    async fn publish_user(&self, user: UserId, payload: &[u8]) -> DomainResult<()>;
}

// ---------- auth ----------
#[async_trait]
pub trait Hasher: Send + Sync + 'static {
    fn hash(&self, plaintext: &str) -> DomainResult<String>;
    fn verify(&self, plaintext: &str, encoded: &str) -> DomainResult<bool>;
}

#[async_trait]
pub trait TokenIssuer: Send + Sync + 'static {
    fn issue_access(&self, user: UserId) -> DomainResult<(String, DateTime<Utc>)>;
    fn issue_refresh(&self, user: UserId) -> DomainResult<(String, DateTime<Utc>)>;
    fn verify_access(&self, token: &str) -> DomainResult<UserId>;
    fn verify_refresh(&self, token: &str) -> DomainResult<UserId>;
}
