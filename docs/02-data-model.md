# Data Model — Cowork Chat

> *Audience: backend engineers, DBAs. The DDL is in `db/schema.sql`; this file explains the entities, relationships, lifecycles, and the indexes that matter.*

## Entities, at a glance

```
        ┌─────────┐         ┌─────────────┐
        │  users  │◀────────│ memberships │
        └────┬────┘ M     N └──────┬──────┘
             │                     │
             │ 1                   │ N
             │                     ▼
             │              ┌────────────┐
             │              │ workspaces │
             │              └─────┬──────┘
             │                    │ 1
             │                    │
             │                    ▼ N
             │            ┌────────────┐
             │            │  channels  │ (kind: public|private)
             │            └─────┬──────┘
             │                  │ 1
             │                  │
             │                  ▼ N
             │            ┌────────────┐         ┌────────────┐
             └──────────▶│  messages  │◀────────│ reactions  │
                          └─────┬──────┘     N 1 └────────────┘
                                │
                                │ 0..1 (parent)
                                ▼ N
                          ┌────────────┐
                          │  messages  │  (threaded replies — self-FK)
                          └────────────┘

        ┌─────────────┐         ┌──────────────┐
        │  dm_threads │◀────M─N│ dm_members   │
        └──────┬──────┘         └──────────────┘
               │ 1
               ▼ N
        ┌────────────┐
        │  messages  │ (DM messages — channel_id null, dm_thread_id set)
        └────────────┘

        Redis (ephemeral):
          presence:{user_id}  → status, last_heartbeat
          ws:fanout:channel:{id}, ws:fanout:dm:{id}  → pub/sub channels
```

## Identity and time conventions

- **All ids are UUIDv7** (time-ordered, lexicographically sortable). `uuid_v7()` is provided by the `pg_uuidv7` extension; if unavailable, we fall back to `gen_random_uuid()` (UUIDv4) and add a `created_at` index. UUIDv7 is preferred because the primary key sort order matches insertion order, which keeps Postgres' B-tree pages dense and makes pagination by id efficient.
- **All timestamps are `TIMESTAMPTZ`** stored in UTC. We never store a naive `TIMESTAMP`.
- **All updatable rows carry `created_at` and `updated_at`**. Tombstoned rows additionally carry `deleted_at` (nullable); we use **soft deletes** for messages, channels, and workspaces to support audit and recovery. Hard deletes happen only via a sweeper job that respects retention policy (out of scope for the lean demo).
- **All free-text columns are `TEXT`** (not `VARCHAR(N)`). Length limits are enforced in the application layer where they can be coordinated with i18n character-count semantics. Postgres `TEXT` and `VARCHAR(N)` have the same storage cost.

## Entities — detail

### `users`
The natural person of the system.

| Column | Type | Notes |
|---|---|---|
| `id` | UUID | PK |
| `email` | TEXT | unique, case-insensitive (via `citext` extension or lower-cased index) |
| `display_name` | TEXT | what other people see |
| `password_hash` | TEXT | argon2id-encoded string |
| `avatar_url` | TEXT NULL | nullable; client falls back to initials |
| `created_at`, `updated_at`, `deleted_at` | TIMESTAMPTZ | soft delete |

**Indexes**: unique on `lower(email)`; partial index on `id WHERE deleted_at IS NULL` for the common "find active user" path.

### `workspaces`
The top-level container. One organisation = one workspace.

| Column | Type | Notes |
|---|---|---|
| `id` | UUID | PK |
| `name` | TEXT | display name |
| `slug` | TEXT | unique, lowercase, hyphenated, used in URLs |
| `created_by` | UUID | FK → users.id |
| `created_at`, `updated_at`, `deleted_at` | TIMESTAMPTZ | |

**Indexes**: unique on `slug`.

### `memberships`
Many-to-many between users and workspaces, with role.

| Column | Type | Notes |
|---|---|---|
| `workspace_id` | UUID | FK → workspaces.id |
| `user_id` | UUID | FK → users.id |
| `role` | TEXT | `'owner' \| 'admin' \| 'member'` |
| `joined_at` | TIMESTAMPTZ | |

**PK**: (workspace_id, user_id).
**Indexes**: `(user_id)` for "what workspaces am I in?".

### `channels`
A topic-scoped conversation inside a workspace.

| Column | Type | Notes |
|---|---|---|
| `id` | UUID | PK |
| `workspace_id` | UUID | FK |
| `name` | TEXT | slug-ish, unique per workspace |
| `topic` | TEXT | one-line description |
| `kind` | TEXT | `'public' \| 'private'` |
| `created_by` | UUID | FK |
| `created_at`, `updated_at`, `deleted_at` | TIMESTAMPTZ | |

**Indexes**: unique `(workspace_id, lower(name))`; index `(workspace_id) WHERE deleted_at IS NULL`.

### `channel_members`
Required only for `kind='private'` channels. Public channels are implicitly joined by all workspace members.

| Column | Type | Notes |
|---|---|---|
| `channel_id` | UUID | FK |
| `user_id` | UUID | FK |
| `joined_at` | TIMESTAMPTZ | |

**PK**: (channel_id, user_id). **Index**: `(user_id)`.

### `dm_threads`
A direct-message conversation between two or more users. Each DM thread is canonicalised: members are stored sorted; uniqueness across the membership set is enforced.

| Column | Type | Notes |
|---|---|---|
| `id` | UUID | PK |
| `members_hash` | TEXT | sha256 of sorted `user_id`s — uniqueness key |
| `created_by` | UUID | FK |
| `created_at` | TIMESTAMPTZ | |

**Indexes**: unique on `members_hash`.

### `dm_members`
Membership rows for a DM thread.

| Column | Type | Notes |
|---|---|---|
| `dm_thread_id` | UUID | FK |
| `user_id` | UUID | FK |

**PK**: (dm_thread_id, user_id). **Index**: `(user_id)`.

### `messages`
The central entity. A single table serves both channel messages and DM messages by virtue of which FK is set.

| Column | Type | Notes |
|---|---|---|
| `id` | UUID | PK (UUIDv7 → time-ordered) |
| `channel_id` | UUID NULL | FK; non-null for channel messages |
| `dm_thread_id` | UUID NULL | FK; non-null for DM messages |
| `parent_id` | UUID NULL | FK → messages.id; non-null for thread replies |
| `author_id` | UUID | FK → users.id |
| `body` | TEXT | message text; rendered as markdown subset on the client |
| `mentions` | UUID[] | array of mentioned user ids (denormalised for fast notification fan-out) |
| `edited_at` | TIMESTAMPTZ NULL | non-null if edited |
| `created_at` | TIMESTAMPTZ | |
| `deleted_at` | TIMESTAMPTZ NULL | soft delete |

**Constraints**: `CHECK ((channel_id IS NOT NULL) <> (dm_thread_id IS NOT NULL))` — exactly one is set. **Indexes**:
- `(channel_id, created_at DESC) WHERE deleted_at IS NULL` — primary read pattern: "latest N messages in this channel".
- `(dm_thread_id, created_at DESC) WHERE deleted_at IS NULL` — same for DMs.
- `(parent_id, created_at ASC) WHERE deleted_at IS NULL AND parent_id IS NOT NULL` — "messages in this thread".
- GIN on `mentions` — fast "mentions of user X" lookups.
- Partial index `(author_id, created_at DESC) WHERE deleted_at IS NULL` — for the user's own message history.

### `reactions`
A user-emoji-per-message tuple.

| Column | Type | Notes |
|---|---|---|
| `message_id` | UUID | FK |
| `user_id` | UUID | FK |
| `emoji` | TEXT | the emoji codepoint(s); validated against an allow-list on the application side |
| `created_at` | TIMESTAMPTZ | |

**PK**: (message_id, user_id, emoji). **Index**: `(message_id)` for the read pattern "show reactions on this message".

## Lifecycles

### Message lifecycle
```
                created                       edited               deleted (soft)
 (none) ────▶ ACTIVE ──────────────▶ ACTIVE ──────────▶ ACTIVE ──────────────▶ DELETED
                │   (edited_at NULL)    (edited_at set)         (deleted_at set)
                │
                └─────────────▶ deleted (soft) ─▶ DELETED
                                                  (deleted_at set, visible only to author and admins
                                                   with a "deleted" placeholder rendered)
```

A deleted message remains in the table for audit. Reactions and thread replies on a deleted message remain accessible.

### Channel lifecycle
- Created by any workspace member (default).
- Renamed by the creator or any workspace admin.
- Soft-deleted by the creator or any workspace admin. After soft-delete, the channel disappears from member channel lists; messages remain queryable to admins via a special endpoint (out of scope for the lean demo UI; the data path is open).

### Presence lifecycle (Redis)
- On WS connect: `presence:{user_id}` set to `{status: "online", last_heartbeat: now}` with TTL = 45 s.
- Every 15 s of an active WS: heartbeat refreshes the TTL.
- On WS disconnect: TTL allowed to expire; ~30 s later the user appears "offline" to others.
- User-requested status changes (away, dnd, online) update the `status` field directly; offline is determined by TTL only.

## Read patterns and their indexes

The model is shaped by what we read most. Top three reads, by frequency:

1. **"give me the last N messages in this channel/DM"** — backed by `(channel_id, created_at DESC) WHERE deleted_at IS NULL` and `(dm_thread_id, created_at DESC) WHERE deleted_at IS NULL`. Both filtered on `deleted_at` to skip tombstones.
2. **"give me the channels I belong to in this workspace"** — joins `memberships`/`channel_members` to `channels` filtered by workspace. Backed by the membership PKs and the `(workspace_id) WHERE deleted_at IS NULL` partial index on channels.
3. **"are there any unread mentions for me?"** — uses the GIN index on `messages.mentions` combined with a `last_read_at` value (stored on `memberships` for channels, on `dm_members` for DMs). The lean demo computes mention counts on the fly; production would maintain a counter.

## Why these specific shapes

### A single `messages` table for channels and DMs
We use one table with a CHECK that exactly one of `channel_id`, `dm_thread_id` is set. Alternatives — separate `channel_messages` and `dm_messages` tables — duplicate the message lifecycle, the reactions table, the threads, and every supporting index. The CHECK keeps both kinds of message in a single physical structure that can be paginated, searched, and indexed once. The cost is a slightly more complex query on the partial indexes (always with the right filter); the saving is one table instead of two and one ladder of triggers instead of two.

### Mentions as a `UUID[]` on the message
This is a denormalisation against the textbook third-normal-form answer (a separate `message_mentions` table). The justification: 99% of reads of mentions are in the form "for this message, who is mentioned?" — a single column read with no join. The GIN index gives us "all messages mentioning user X" without a join either. The cost is that the message row is slightly fatter; for typical messages the impact is sub-row-size, and Postgres' TOAST kicks in only for very large rows.

### Soft delete, not hard delete
Hard deletes break two things on a chat product: (i) thread replies whose parent vanishes, and (ii) audit trails that a real organisation will eventually demand. Soft delete keeps the row, renders a tombstone in the UI, preserves the relational integrity of threads and reactions, and leaves the door open for a regulator-friendly retention policy.

### Argon2 for password hashing
OWASP's recommended modern default. We use the argon2id variant (memory-hard + side-channel-resistant) with `m=19 MB, t=2, p=1`. These constants are configurable via env; the demo defaults are conservative and match OWASP guidance.

## Migrations

`sqlx` migrations under `backend/migrations/`. Day 2 will produce `0001_initial.sql` (essentially the DDL in `db/schema.sql`) and subsequent migrations for any additive changes. **No destructive migrations on the messages table** — the chat history is the product.

## What's deferred

- Full-text search (`tsvector` column + GIN). Out of scope for the lean demo.
- File attachments (`files` table + S3-compatible store). Out of scope.
- Message edits history (separate `message_revisions` table). Lean demo just sets `edited_at`.
- Last-read-at counters and unread-count materialisation. Lean demo computes on read.

Day 2 implements this schema in Rust against sqlx, with repository ports defined in `domain/` and impls in `infra/`.
