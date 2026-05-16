-- Cowork Chat — Initial schema
-- Postgres 15+. Soft deletes, UTC timestamps, UUIDv7 ids.
--
-- This file is reference DDL. The sqlx migration in
-- backend/migrations/0001_initial.sql (Day 2) is the production source of truth.

-- Extensions ------------------------------------------------------------------
CREATE EXTENSION IF NOT EXISTS "pgcrypto";   -- gen_random_uuid() fallback
CREATE EXTENSION IF NOT EXISTS "citext";     -- case-insensitive email
-- pg_uuidv7 is preferred but not always available; if present, prefer it:
-- CREATE EXTENSION IF NOT EXISTS "pg_uuidv7";

-- Helper ----------------------------------------------------------------------
-- Use uuid_v7() if available, otherwise gen_random_uuid().
-- The migration will choose at run time; this file defers to gen_random_uuid()
-- so it parses everywhere.

-- =============================================================================
-- USERS
-- =============================================================================
CREATE TABLE users (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    email           CITEXT      NOT NULL UNIQUE,
    display_name    TEXT        NOT NULL,
    password_hash   TEXT        NOT NULL,
    avatar_url      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ
);

CREATE INDEX idx_users_active
    ON users (id)
    WHERE deleted_at IS NULL;

-- =============================================================================
-- WORKSPACES
-- =============================================================================
CREATE TABLE workspaces (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT        NOT NULL,
    slug            TEXT        NOT NULL UNIQUE,
    created_by      UUID        NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ
);

CREATE INDEX idx_workspaces_active
    ON workspaces (id)
    WHERE deleted_at IS NULL;

-- =============================================================================
-- MEMBERSHIPS (user ↔ workspace, with role)
-- =============================================================================
CREATE TABLE memberships (
    workspace_id    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id)      ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member')),
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_read_at    TIMESTAMPTZ,  -- per-workspace read cursor (channels use separate row in channel_reads)
    PRIMARY KEY (workspace_id, user_id)
);

CREATE INDEX idx_memberships_user ON memberships (user_id);

-- =============================================================================
-- CHANNELS
-- =============================================================================
CREATE TABLE channels (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id    UUID        NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    topic           TEXT        NOT NULL DEFAULT '',
    kind            TEXT        NOT NULL CHECK (kind IN ('public', 'private')),
    created_by      UUID        NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_channels_name_per_workspace
    ON channels (workspace_id, lower(name));

CREATE INDEX idx_channels_workspace_active
    ON channels (workspace_id)
    WHERE deleted_at IS NULL;

-- =============================================================================
-- CHANNEL_MEMBERS (required for private channels only)
-- =============================================================================
CREATE TABLE channel_members (
    channel_id      UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_read_at    TIMESTAMPTZ,
    PRIMARY KEY (channel_id, user_id)
);

CREATE INDEX idx_channel_members_user ON channel_members (user_id);

-- =============================================================================
-- DM_THREADS
-- =============================================================================
CREATE TABLE dm_threads (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    members_hash    TEXT        NOT NULL UNIQUE,  -- sha256 of sorted member ids
    created_by      UUID        NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- =============================================================================
-- DM_MEMBERS
-- =============================================================================
CREATE TABLE dm_members (
    dm_thread_id    UUID NOT NULL REFERENCES dm_threads(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id)      ON DELETE CASCADE,
    last_read_at    TIMESTAMPTZ,
    PRIMARY KEY (dm_thread_id, user_id)
);

CREATE INDEX idx_dm_members_user ON dm_members (user_id);

-- =============================================================================
-- MESSAGES (channel + DM; one table)
-- =============================================================================
CREATE TABLE messages (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id      UUID        REFERENCES channels(id)   ON DELETE CASCADE,
    dm_thread_id    UUID        REFERENCES dm_threads(id) ON DELETE CASCADE,
    parent_id       UUID        REFERENCES messages(id)   ON DELETE SET NULL,
    author_id       UUID        NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    body            TEXT        NOT NULL,
    mentions        UUID[]      NOT NULL DEFAULT '{}',
    edited_at       TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ,

    -- Exactly one of channel_id, dm_thread_id is non-null:
    CONSTRAINT chk_messages_one_target
        CHECK ((channel_id IS NOT NULL) <> (dm_thread_id IS NOT NULL))
);

-- Primary read patterns:
CREATE INDEX idx_messages_channel_recent
    ON messages (channel_id, created_at DESC)
    WHERE deleted_at IS NULL AND channel_id IS NOT NULL;

CREATE INDEX idx_messages_dm_recent
    ON messages (dm_thread_id, created_at DESC)
    WHERE deleted_at IS NULL AND dm_thread_id IS NOT NULL;

-- Thread replies:
CREATE INDEX idx_messages_thread
    ON messages (parent_id, created_at ASC)
    WHERE deleted_at IS NULL AND parent_id IS NOT NULL;

-- Mentions of user X:
CREATE INDEX idx_messages_mentions
    ON messages USING GIN (mentions);

-- Author's own messages:
CREATE INDEX idx_messages_author_recent
    ON messages (author_id, created_at DESC)
    WHERE deleted_at IS NULL;

-- =============================================================================
-- REACTIONS
-- =============================================================================
CREATE TABLE reactions (
    message_id      UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    emoji           TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, user_id, emoji)
);

CREATE INDEX idx_reactions_message ON reactions (message_id);

-- =============================================================================
-- Trigger to keep updated_at fresh on UPDATE
-- =============================================================================
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_users_updated_at      BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_workspaces_updated_at BEFORE UPDATE ON workspaces
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_channels_updated_at   BEFORE UPDATE ON channels
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
