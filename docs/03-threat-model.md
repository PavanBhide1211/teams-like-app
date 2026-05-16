# Threat Model — Cowork Chat

> *Audience: anyone reviewing the security posture of this system before it ships. Organised by STRIDE (Spoofing, Tampering, Repudiation, Information disclosure, Denial of service, Elevation of privilege) with each threat mapped to the architectural element it touches and the mitigation already designed in.*

## Trust boundaries

```
                         ┌──────── Trust Zone A ─────────┐
                         │   User's device (laptop)       │
                         │                                │
                         │   Tauri shell (Rust) ───┐      │
                         │      ▲                  │      │
                         │      │ Tauri IPC        │      │
                         │      ▼                  │      │
                         │   React app (webview) ◀─┘      │
                         └─────────────┬──────────────────┘
                                       │ TLS 1.3
                                       ▼
                         ┌──────── Trust Zone B ─────────┐
                         │   Axum server                 │
                         │   (auth + app layer + WS)     │
                         │                                │
                         │   sqlx ──▶ Postgres            │
                         │   redis ─▶ Redis               │
                         └────────────────────────────────┘
```

**A → B trust**: untrusted. Every request is authenticated (JWT) and authorised; every input is validated server-side. Nothing the client says is trusted on the server side, ever.

**B → A trust**: partial. The client may render server-controlled content (message bodies), so we mind cross-site scripting via the webview's CSP and through controlled rendering (no `dangerouslySetInnerHTML`, sanitised markdown).

**Within B**: Postgres and Redis are accessed only by the server. Production deployments place them on a private network with no public ingress.

## STRIDE — applied

### Spoofing — pretending to be someone else

**S-1. Login brute-force / credential stuffing.**
*Surface*: `POST /auth/login`.
*Mitigation*: argon2id hashing keeps offline cracking expensive. Server-side rate limiting on `/auth/login`: 10 attempts per IP per 5 min, 20 per email per hour. Generic error message ("invalid credentials") prevents user-enumeration through differential errors. Account lockout intentionally not implemented (it itself is a DoS vector); rate limiting is sufficient at this scale.

**S-2. JWT theft and replay.**
*Surface*: Authorization header on REST + WS handshake.
*Mitigation*: short access-token lifetime (24 h). Refresh tokens are rotated on every use and bound to a stored refresh-token id (revocable). Tokens are HS256-signed with a strong 256-bit secret loaded from env. **No token data is read from the client; the server re-fetches workspace memberships per request.**
*Residual risk*: a stolen access token is usable for up to 24 h. For higher-stakes deployments, switch to 15-min access + token-binding to a device fingerprint, at the cost of UX (re-login friction).

**S-3. Impersonation of the WS sender.**
*Surface*: every WS frame.
*Mitigation*: the WS handshake authenticates the user once; subsequent frames are attributed to the connection's user-id server-side. Client-supplied `author_id` fields in frames are ignored and overwritten.

**S-4. Workspace-slug squatting on registration.**
*Surface*: workspace creation.
*Mitigation*: slug uniqueness enforced at the DB. Reserved-slug allow-list prevents creating `admin`, `api`, `auth`, etc.

### Tampering — modifying data in flight or at rest

**T-1. Message-content tampering in transit.**
*Surface*: REST and WS over the network.
*Mitigation*: TLS 1.3 mandatory. The Tauri webview is configured to refuse non-HTTPS upgrades for the WS endpoint in release builds; dev builds allow `ws://localhost:*` only.

**T-2. Tampering with the `mentions[]` array to mention everyone.**
*Surface*: message create.
*Mitigation*: server re-parses `mentions` from the body (`@username` syntax), intersects with the channel's member set, and uses the server-derived list, not the client's. The client's array is treated as a hint only.

**T-3. Soft-delete bypass.**
*Surface*: the messages table.
*Mitigation*: hard-delete endpoints do not exist in the public API. The only way to remove rows is the sweeper job (out of scope for the lean demo), which runs with elevated DB credentials and audits every deletion.

**T-4. Audit-log tampering.**
*Out of scope for the lean demo*. Production deployments would either pipe app logs to an append-only sink (Cloudwatch Logs, GCP Logging, an S3 bucket with object-lock) or maintain a chained-hash log in Postgres like the EU AI Act project we did earlier.

### Repudiation — "I didn't send that"

**R-1. Sender denies authorship of a message.**
*Mitigation*: `messages.author_id` is server-stamped from the authenticated user-id. Edits leave `edited_at` set; the original body is not preserved in the lean demo (production would add a `message_revisions` table). Deletions are soft, with `deleted_at` and the author preserved.

**R-2. Admin denies executing a privileged action.**
*Mitigation*: server-side action log (out of scope for the lean demo; documented as a production extension).

### Information disclosure

**I-1. Cross-workspace data leakage.**
*Surface*: any read endpoint.
*Mitigation*: every read passes through an authorisation check that asserts the requesting user has membership in the target workspace. **The authorisation check lives in the application layer, not the route handler** — same check applies whether the request arrived via REST, WS, or a hypothetical future RPC.

**I-2. Private-channel content leakage to non-members.**
*Surface*: channel reads.
*Mitigation*: `channels.kind = 'private'` is checked; non-members are denied at the application service. Membership cannot be inferred by error response (consistent 403 whether the user lacks membership or the channel doesn't exist).

**I-3. Mention-driven user enumeration.**
*Surface*: composer autocomplete.
*Mitigation*: mention autocomplete only suggests members of the current channel/DM, never the global user directory.

**I-4. Password-hash leakage via API.**
*Surface*: `users` row reads.
*Mitigation*: `password_hash` column is excluded from every SELECT used by API handlers. Type-level enforcement: the `User` domain type does not carry a hash field; only the `Credentials` repo internal type does.

**I-5. Avatar URL pointing to attacker-controlled host.**
*Surface*: avatar_url.
*Mitigation*: the lean demo only renders avatars from the same origin as the API; user-supplied avatar URLs are rejected unless they pass a same-origin or allow-listed check. (Production would proxy through an image fetcher with size limits.)

**I-6. WebView content injection (XSS).**
*Surface*: message body rendering.
*Mitigation*: messages are stored as plain text; we render a *strict markdown subset* (bold, italic, code, code blocks, links, mentions). No raw HTML is allowed. The renderer never uses `dangerouslySetInnerHTML`; it walks a parsed AST. Links open in the user's browser via Tauri's `shell.open` API with the host pre-validated; `javascript:` and `data:` URLs are stripped.

**I-7. CSP weakening via the webview.**
*Surface*: the Tauri webview's CSP.
*Mitigation*: strict CSP shipped with the app — `default-src 'self'; img-src 'self' data: https:; connect-src 'self' wss: https:; style-src 'self' 'unsafe-inline'; script-src 'self'`. `'unsafe-inline'` is acceptable on `style-src` for Tailwind's runtime-generated styles; it is not extended to scripts. Day 3 will lock this down in `frontend/src-tauri/tauri.conf.json`.

### Denial of service

**D-1. Per-endpoint flood.**
*Surface*: any HTTP endpoint.
*Mitigation*: tower-governor middleware with per-route policies:
- `/auth/login`: 10 / 5 min / IP, plus 20 / hour / email.
- `/auth/register`: 5 / hour / IP.
- POST `/messages`: 30 / minute / user.
- All others: 60 / minute / user, 300 / minute / IP.

**D-2. WS-frame flood from a single connection.**
*Surface*: WS gateway.
*Mitigation*: per-connection token-bucket — 60 frames / 10 s. Over-budget connections receive a `ServerError(RateLimited)` frame, then the connection is closed after one warning.

**D-3. Backpressure starvation in fan-out.**
*Surface*: WS publisher → many subscribers.
*Mitigation*: per-connection bounded mpsc (1024 frames). A slow consumer that fills its buffer is disconnected; the rest of the fan-out is not blocked.

**D-4. Postgres connection exhaustion.**
*Surface*: REST handlers.
*Mitigation*: sqlx pool size capped (default 8, configurable). Connection acquisition has a 5 s timeout; on exhaustion the handler returns 503 with a `Retry-After: 1` header.

**D-5. Storage exhaustion via giant messages.**
*Surface*: POST `/messages`.
*Mitigation*: body length cap (8 KB before encoding, ~4 KB after, per message) enforced server-side. Mentions array capped at 50 ids. Reactions per message capped at 100 distinct emoji rows (per-user emoji uniqueness enforced by PK).

**D-6. Tauri shell DoS via malicious deep-links.**
*Surface*: Tauri custom URL scheme.
*Mitigation*: deep-link handler validates the URL against a small allow-list before opening anything. Out of scope for the lean demo; documented for future extension.

### Elevation of privilege

**E-1. Member becomes admin via parameter tampering.**
*Surface*: PATCH `/memberships`.
*Mitigation*: role-change endpoint accepts only `(target_user_id, new_role)` and checks that the requester has `owner` (and only an owner) for the workspace. Demotion of the last owner is refused.

**E-2. Becoming a channel member without permission.**
*Surface*: POST `/channels/{id}/members`.
*Mitigation*: for `kind='private'` channels, only existing members or workspace admins may add. For `kind='public'`, no add endpoint exists — public membership is implicit by workspace membership.

**E-3. Cross-user message deletion.**
*Surface*: DELETE `/messages/{id}`.
*Mitigation*: only the author or a workspace admin may delete. Authorisation in the application service.

**E-4. JWT replay against a different workspace.**
*Surface*: workspace-scoped endpoints.
*Mitigation*: the JWT does not embed workspace membership. Each request re-checks the user's membership; revoking a user from a workspace takes effect on the very next request.

## Tauri-specific hardening

The Tauri shell expands the attack surface in characteristic ways. Specific controls:

- **`tauri.conf.json` `app.security.csp`** is set as above (strict). It is enforced by the webview process.
- **`allowlist.fs`** is set to `false`. The frontend has no direct disk access; it must go through Tauri commands that are explicitly enumerated.
- **`allowlist.shell.open`** is set to `true` (so users can click external links) but with `scope`: only `http*://*` URLs that pass a same-host check. `file://` and `javascript:` are forbidden.
- **`allowlist.http.request`** is `false`. The frontend has no `fetch` of arbitrary URLs from the privileged context; all server communication goes through our typed API client which hits the configured backend origin only.
- **`updater`** is `false` in the lean demo. Production would enable signed updates with a pinned public key.
- **`devTools`** is `false` in release builds.

## Data-at-rest

- **Database encryption**: Postgres encryption-at-rest is the responsibility of the deployment (cloud-managed disk encryption, or `pgcrypto` columns for specific fields). The schema does not encrypt at the column level by default.
- **Password hashes** are argon2id-encoded strings. The encoded string contains its own parameters and salt; rotating parameters is a per-user lazy migration.
- **Tokens** are stored only in client-side memory (Tauri's secure storage on disk for refresh tokens). They are never logged on the server.
- **Logs** do not include passwords, tokens, or message bodies. They include request ids, user ids, route names, status codes, and timings.

## What is explicitly *not* in this threat model

- **Federation / multi-tenant security** (single-tenant deployment by design).
- **Compliance with specific regimes** (GDPR-grade DPIA, HIPAA, SOC 2). The lean demo's posture is "consistent with reasonable security practice"; specific compliance work is project-by-project.
- **Anti-abuse / spam / content moderation**. A real deployment would need this; designing it is out of scope for a 3-day demo.

## Pre-launch review checklist (Day 3)

Before any real demo:

- [ ] Strict CSP confirmed in `tauri.conf.json`.
- [ ] `allowlist` minimised (only commands actually used are exposed).
- [ ] `devTools` off in release.
- [ ] All authenticated endpoints require a valid JWT (test by hitting with a stripped header).
- [ ] All workspace-scoped endpoints check membership (cross-workspace test).
- [ ] Rate limits applied on `/auth/*` and on `POST /messages`.
- [ ] No `password_hash` in any API response (test by inspecting a `/users/me` response).
- [ ] WS frames from unauthenticated connections are rejected.
- [ ] Demo deployment has TLS terminated in front of the Axum server.
