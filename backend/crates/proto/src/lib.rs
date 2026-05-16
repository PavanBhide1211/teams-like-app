//! Wire types — REST DTOs and WS frame schema.
//!
//! Kept separate from `server` so a CLI / benchmark binary can speak the same
//! protocol without dragging in axum.

pub mod ws;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use domain::channel::{Channel, ChannelKind};
use domain::dm::DmThread;
use domain::ids::{ChannelId, DmThreadId, MessageId, UserId, WorkspaceId};
use domain::message::{Message, MessageTarget};
use domain::presence::PresenceStatus;
use domain::reaction::Reaction;
use domain::user::User;
use domain::workspace::{Membership, Role, Workspace};

// ===== Auth =====

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token:        String,
    pub refresh_token:       String,
    pub access_expires_at:   DateTime<Utc>,
    pub refresh_expires_at:  DateTime<Utc>,
    pub user:                UserDto,
}

#[derive(Debug, Serialize)]
pub struct UserDto {
    pub id: UserId,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

impl From<User> for UserDto {
    fn from(u: User) -> Self {
        Self { id: u.id, email: u.email, display_name: u.display_name, avatar_url: u.avatar_url }
    }
}

// ===== Workspaces =====

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceDto {
    pub id: WorkspaceId,
    pub name: String,
    pub slug: String,
}

impl From<Workspace> for WorkspaceDto {
    fn from(w: Workspace) -> Self { Self { id: w.id, name: w.name, slug: w.slug } }
}

#[derive(Debug, Serialize)]
pub struct MembershipDto {
    pub workspace_id: WorkspaceId,
    pub user_id: UserId,
    pub role: Role,
}

impl From<Membership> for MembershipDto {
    fn from(m: Membership) -> Self {
        Self { workspace_id: m.workspace_id, user_id: m.user_id, role: m.role }
    }
}

// ===== Channels =====

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    #[serde(default)]
    pub topic: String,
    pub kind: ChannelKind,
}

#[derive(Debug, Serialize)]
pub struct ChannelDto {
    pub id: ChannelId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub topic: String,
    pub kind: ChannelKind,
}

impl From<Channel> for ChannelDto {
    fn from(c: Channel) -> Self {
        Self {
            id: c.id, workspace_id: c.workspace_id,
            name: c.name, topic: c.topic, kind: c.kind,
        }
    }
}

// ===== DMs =====

#[derive(Debug, Deserialize)]
pub struct CreateDmRequest {
    pub members: Vec<UserId>,
}

#[derive(Debug, Serialize)]
pub struct DmThreadDto {
    pub id: DmThreadId,
    pub members_hash: String,
}

impl From<DmThread> for DmThreadDto {
    fn from(d: DmThread) -> Self { Self { id: d.id, members_hash: d.members_hash } }
}

// ===== Messages =====

#[derive(Debug, Deserialize)]
pub struct CreateMessageRequest {
    /// One of channel_id / dm_thread_id must be set.
    pub channel_id:   Option<ChannelId>,
    pub dm_thread_id: Option<DmThreadId>,
    pub parent_id:    Option<MessageId>,
    pub body:         String,
    #[serde(default)]
    pub mentions:     Vec<UserId>,
}

#[derive(Debug, Deserialize)]
pub struct EditMessageRequest {
    pub body: String,
    #[serde(default)]
    pub mentions: Vec<UserId>,
}

#[derive(Debug, Serialize)]
pub struct MessageDto {
    pub id:          MessageId,
    pub target:      MessageTarget,
    pub parent_id:   Option<MessageId>,
    pub author_id:   UserId,
    pub body:        String,
    pub mentions:    Vec<UserId>,
    pub edited_at:   Option<DateTime<Utc>>,
    pub created_at:  DateTime<Utc>,
    pub deleted_at:  Option<DateTime<Utc>>,
}

impl From<Message> for MessageDto {
    fn from(m: Message) -> Self {
        Self {
            id: m.id, target: m.target, parent_id: m.parent_id, author_id: m.author_id,
            body: m.body, mentions: m.mentions, edited_at: m.edited_at,
            created_at: m.created_at, deleted_at: m.deleted_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    pub before: Option<DateTime<Utc>>,
    pub limit:  Option<u32>,
}

// ===== Reactions =====

#[derive(Debug, Deserialize)]
pub struct ReactionRequest {
    pub emoji: String,
}

#[derive(Debug, Serialize)]
pub struct ReactionDto {
    pub message_id: MessageId,
    pub user_id: UserId,
    pub emoji: String,
    pub created_at: DateTime<Utc>,
}

impl From<Reaction> for ReactionDto {
    fn from(r: Reaction) -> Self {
        Self {
            message_id: r.message_id, user_id: r.user_id,
            emoji: r.emoji, created_at: r.created_at,
        }
    }
}

// ===== Presence =====

#[derive(Debug, Serialize, Deserialize)]
pub struct PresenceDto {
    pub user_id: UserId,
    pub status:  PresenceStatus,
    pub last_heartbeat: DateTime<Utc>,
}

// ===== Errors =====

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}
