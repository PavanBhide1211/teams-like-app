# Realtime Protocol — Cowork Chat

> *Audience: anyone implementing or debugging the WebSocket gateway, the WS client, or anything that exchanges realtime frames.*

## Goals

- Single WebSocket connection per client.
- Bidirectional, binary, low-overhead frames.
- Server-authoritative: client never asserts identity beyond the initial handshake.
- Crash-safe: the connection can disappear at any moment; both sides handle reconnect and out-of-order delivery.
- Horizontally-scalable on the server side without sticky sessions.

## Encoding — msgpack, not JSON

Every frame is a single msgpack-encoded map. msgpack is chosen for the reasons in ADR-003: ~30–50 % smaller than equivalent JSON on chat-sized payloads, ~3× faster to encode and decode in both Rust (`rmp-serde`) and TypeScript (`@msgpack/msgpack`), and well-typed.

Two top-level fields are present on every frame:

| Field | Type | Notes |
|---|---|---|
| `op` | u8 | opcode (see table below) |
| `d`  | map (any) | frame-specific payload; may be empty `{}` |

Frames that expect an acknowledgement also carry:

| Field | Type | Notes |
|---|---|---|
| `nonce` | u32 | client-chosen; server echoes it in the `ACK` frame |

## Endpoint and handshake

`GET /ws?token=<jwt>`  →  101 Switching Protocols → WebSocket open.

The bearer JWT is passed as a query parameter (necessary because browsers cannot set custom headers on a WS handshake). The server validates the token; if invalid, it closes with code **4001** (custom: `auth_failed`). On success, the server immediately sends a `HELLO` frame.

### `HELLO` (op = 0x00, server → client)

Sent once, right after the connection upgrades, before any other frame.

```
{
  "op": 0x00,
  "d": {
    "user_id": "uuid",
    "server_version": "string",
    "heartbeat_interval_ms": 15000,
    "connection_id": "uuid"
  }
}
```

`connection_id` is a server-issued opaque id that clients echo back in any later `RESUME` attempt. `heartbeat_interval_ms` is the cadence at which the client must send `HEARTBEAT` frames.

## Opcode table

Opcodes are split into ranges so that intent is visible from the byte:

| Range | Class | Direction |
|---|---|---|
| 0x00–0x0F | session control | bi |
| 0x10–0x1F | subscriptions | client → server |
| 0x20–0x2F | actions (mutate state) | client → server |
| 0x30–0x3F | events (state changed) | server → client |
| 0xF0–0xFF | errors and acks | bi |

### Session control

| Op | Name | Direction | Payload | Notes |
|---|---|---|---|---|
| 0x00 | `HELLO` | s→c | `{user_id, server_version, heartbeat_interval_ms, connection_id}` | First frame |
| 0x01 | `HEARTBEAT` | c→s | `{}` | Sent every `heartbeat_interval_ms` |
| 0x02 | `HEARTBEAT_ACK` | s→c | `{}` | Server confirms |
| 0x03 | `RESUME` | c→s | `{connection_id, last_seq}` | Reconnect after disconnect |
| 0x04 | `RESUMED` | s→c | `{from_seq, to_seq}` | Server replayed events |
| 0x05 | `CLOSE` | s→c | `{reason, code}` | Server initiating clean close |

### Subscriptions

| Op | Name | Direction | Payload | Notes |
|---|---|---|---|---|
| 0x10 | `SUBSCRIBE` | c→s | `{channel_ids: [uuid], dm_thread_ids: [uuid]}` | Replace current sub set |
| 0x11 | `SUBSCRIBE_ADD` | c→s | `{channel_ids?: [uuid], dm_thread_ids?: [uuid]}` | Append to sub set |
| 0x12 | `SUBSCRIBE_REMOVE` | c→s | `{channel_ids?: [uuid], dm_thread_ids?: [uuid]}` | Remove from sub set |

The client picks subscriptions; the server validates each against the authenticated user's membership. Unauthorised entries are silently dropped and a single `ServerError(Forbidden)` is sent listing the rejected ids.

### Actions (mutating)

These cause server-side state changes and produce corresponding events on success. Every action carries a `nonce` so the client can correlate the eventual `ACK` or `ERR`.

| Op | Name | Direction | Payload (`d`) |
|---|---|---|---|
| 0x20 | `MSG_SEND` | c→s | `{channel_id?, dm_thread_id?, body, parent_id?, mentions?: [uuid], nonce}` |
| 0x21 | `MSG_EDIT` | c→s | `{message_id, body, nonce}` |
| 0x22 | `MSG_DELETE` | c→s | `{message_id, nonce}` |
| 0x23 | `REACT_ADD` | c→s | `{message_id, emoji, nonce}` |
| 0x24 | `REACT_REMOVE` | c→s | `{message_id, emoji, nonce}` |
| 0x25 | `TYPING` | c→s | `{channel_id?, dm_thread_id?}` (no nonce; fire-and-forget) |
| 0x26 | `PRESENCE_SET` | c→s | `{status: "online"|"away"|"dnd"}` (offline is implicit by disconnect) |

### Events (server-pushed)

| Op | Name | Direction | Payload (`d`) | Notes |
|---|---|---|---|---|
| 0x30 | `MSG_CREATED` | s→c | full message envelope | broadcast to subscribers of the target channel/DM |
| 0x31 | `MSG_EDITED` | s→c | full message envelope (incl. edited_at) | |
| 0x32 | `MSG_DELETED` | s→c | `{message_id, channel_id?, dm_thread_id?, deleted_at}` | |
| 0x33 | `REACT_CREATED` | s→c | `{message_id, user_id, emoji, created_at}` | |
| 0x34 | `REACT_REMOVED` | s→c | `{message_id, user_id, emoji}` | |
| 0x35 | `TYPING` | s→c | `{user_id, channel_id?, dm_thread_id?, at}` | echoes upstream typing pings |
| 0x36 | `PRESENCE_CHANGED` | s→c | `{user_id, status, last_seen_at?}` | only sent to peers who share a channel/DM with the affected user |
| 0x37 | `CHANNEL_CREATED` | s→c | full channel envelope | only to workspace members |
| 0x38 | `CHANNEL_UPDATED` | s→c | partial channel envelope | |
| 0x39 | `CHANNEL_DELETED` | s→c | `{channel_id}` | |

### Errors and acks

| Op | Name | Direction | Payload (`d`) |
|---|---|---|---|
| 0xF0 | `ACK` | s→c | `{nonce, server_id?}` — `server_id` is the persisted id of a newly-created entity, when applicable |
| 0xF1 | `ERR` | s→c | `{nonce?, code, message}` — `code` is one of the domain error variants |

Error codes:
- `NotFound` (target row missing or not visible to caller)
- `Forbidden` (auth/authz failure for this operation)
- `Conflict` (uniqueness / state conflict)
- `Invalid` (input violated validation rules)
- `RateLimited` (per-connection or per-route limiter tripped)
- `Internal` (catch-all; client should not parse the message field)

## Lifecycle and sequencing

### Sequence numbers and at-least-once delivery

Every event frame the server pushes carries a monotonic `seq` value (u64) on the WS connection. The client tracks `last_seq` it has observed.

On reconnect, the client sends `RESUME` with `{connection_id, last_seq}`. The server attempts to replay events newer than `last_seq` from its in-memory ring (default capacity: 1024 events per connection). If the gap is too large to replay (the requested seq has been evicted from the ring), the server falls back to `HELLO` and the client must perform a cold catch-up: refetch the visible channels' latest messages via REST. The frontend's TanStack Query refresh handles this transparently.

This gives **at-least-once delivery for short gaps**. Duplicate-suppression is the client's job: a message-created event whose `id` is already in the local cache is ignored.

### Heartbeats

- The client sends `HEARTBEAT` every `heartbeat_interval_ms` (default 15 s).
- The server replies with `HEARTBEAT_ACK` immediately.
- If three consecutive heartbeats are missed (default 45 s of silence), the server closes the connection.
- The client, symmetrically, closes if no `HEARTBEAT_ACK` is received within 5 s of a `HEARTBEAT` send.

### Presence

- `PRESENCE_SET` updates Redis directly.
- The TTL on `presence:{user_id}` is `heartbeat_interval_ms * 3` so that a missed heartbeat naturally expires presence within ~45 s.
- `PRESENCE_CHANGED` events are fan-out via a Redis pub/sub channel `presence:peers:{user_id}` that other users subscribe to when they share a channel/DM with the affected user. The subscriber set is computed at SUBSCRIBE time and refreshed on every membership change.

### Typing indicators

- `TYPING` (client → server) is throttled at one frame per 3 s per channel/DM.
- The server fans out a `TYPING` event (with `user_id` and `at` timestamp) to peers subscribed to that channel/DM. No persistence, no ack.
- Receivers display the indicator for 5 s after the most recent ping (debounce).

## Fan-out and horizontal scaling

When the server accepts a `MSG_SEND`:

1. Validate input.
2. Insert into Postgres (`messages` row).
3. Build the `MSG_CREATED` envelope.
4. Publish the envelope to the Redis pub/sub channel for the target (`ws:fanout:channel:{id}` or `ws:fanout:dm:{id}`).
5. Send `ACK(nonce, server_id=message.id)` back on the originating connection.

Every WS gateway node subscribes (on Redis) to the pub/sub channels for which it has at least one connected, subscribed client. When a publish event arrives on a node, the node pushes the corresponding `MSG_CREATED` event frame to every local connection that holds the relevant subscription.

This means messages flow correctly regardless of which gateway node the sender is connected to. There is no sticky-session requirement at the load balancer.

The single point of contention is the Redis pub/sub layer. At production scale, pub/sub channels can be sharded by workspace or even by channel-id hash; the topology is opaque to clients.

## Frame size budgets and limits

| Limit | Value | Enforced where |
|---|---|---|
| Max frame size (compressed wire) | 32 KB | WS upgrade options |
| Max message body | 8 KB pre-encoding (~4 KB after msgpack) | server-side validator on `MSG_SEND` |
| Max mentions per message | 50 | server-side validator |
| Max emoji per reaction set | one per (user, message) pair | DB primary key |
| Max frames per connection | 60 frames / 10 s | tower-governor on the WS gateway |
| Server event ring | 1024 events per connection | in-memory, for `RESUME` |

## Reference: end-to-end "send a message"

```
Client                            Server                         Postgres                     Redis
  │                                 │                                │                          │
  │  WS frame: MSG_SEND             │                                │                          │
  │  {channel_id, body, nonce=42}   │                                │                          │
  ├────────────────────────────────▶│                                │                          │
  │                                 │   parse + validate             │                          │
  │                                 │   authorise (member of ch?)    │                          │
  │                                 │   INSERT messages ────────────▶│                          │
  │                                 │◀────── id, created_at ─────────│                          │
  │                                 │   build MSG_CREATED            │                          │
  │                                 │   PUBLISH ws:fanout:channel:X ───────────────────────────▶│
  │                                 │   send ACK(nonce=42, id=...)   │                          │
  │◀────────────────────────────────│                                │                          │
  │                                 │                                │                          │
  │                                 │◀────────── (other gateway node consumes pub/sub) ─────────│
  │                                 │   for each local sub of ch:X:                              │
  │                                 │     send MSG_CREATED(seq=N)    │                          │
  │◀────────────────────────────────│                                │                          │
  │  (originating client; suppress dup by id)                        │                          │
  │                                 │                                │                          │
  │  Other clients on ch:X:                                          │                          │
  │  receive MSG_CREATED(seq=...)   │                                │                          │
  │◀────────────────────────────────│                                │                          │
```

The originating client receives both `ACK` (with the persisted id) and the broadcast `MSG_CREATED`. The frontend treats the `ACK` as the authoritative confirmation; the broadcast is suppressed if the id already exists in the cache.

## Reference: reconnect

```
Client losses connection at last_seq=812, connection_id=C
  │
  │  (network back)
  │  open new WS, send RESUME({connection_id: C, last_seq: 812})
  ├──────────────▶ Server: look up ring for C
  │                if 812 still in ring → replay events 813..N
  │                                    → send RESUMED({from_seq: 813, to_seq: N})
  │                if not → send HELLO (cold path), client refetches via REST
  │
  │◀────── RESUMED({from_seq: 813, to_seq: 850})
  │◀────── 38 event frames in seq order
  │  (resume normal operation)
```

## Implementation notes

- **Rust server** (`server/src/gateway/`): `axum::extract::ws::WebSocket` + `tokio::sync::mpsc` per connection. Each connection task owns: socket halves, a bounded mpsc receiver for fan-out events, a Redis pub/sub subscription handle, the subscription set, and the event ring.
- **Frame parsing**: `rmp-serde` into a tagged enum keyed on `op`. Reject unknown opcodes with `ERR(Invalid)` and one warning; close after three.
- **Per-connection rate limiter**: a `tokio::time::Interval` token-bucket (1 token / ~170 ms, burst 10), kept in the connection task's local state.
- **TypeScript client** (`frontend/src/shared/ws/`): a small class wrapping `WebSocket` + `@msgpack/msgpack`. Exposes a typed `send(op, d)` and an `on(op, handler)` map. Handles reconnect-with-backoff (250 ms → 1 s → 4 s → 16 s → 60 s cap, jittered).
- **TypeScript ↔ Rust type parity**: a hand-maintained `frontend/src/shared/api/types.ts` mirrors the Rust `proto` crate's frame structs. Day 3 includes a one-paragraph "how to keep these in sync" checklist; in production a build step would generate one from the other.

## Versioning

`HELLO.server_version` carries the protocol's semver. Breaking changes bump the major; clients warn the user when major mismatches. Additive changes (new opcodes, new fields in payloads) bump the minor; clients ignore unknown opcodes and unknown payload fields without erroring.
