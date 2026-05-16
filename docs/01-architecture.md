# Architecture — Cowork Chat

> *Audience: engineers and architects who will build, review, or extend this system. Reads top-down: system context first, then frontend, then backend, then cross-cutting concerns, then the Architecture Decision Records (ADRs) that pinned the major choices.*

## System context

The product is a desktop chat client and its supporting realtime backend. A single deployment serves a single tenant (one organisation per backend instance); horizontal scale is taken seriously in the gateway layer but is not multi-tenant by design.

```
                     ┌─────────────────────────────────────────────┐
                     │                Cowork Chat Desktop           │
                     │  (Tauri 2 shell + React 18 SPA in webview)   │
                     │                                              │
                     │  ┌──────────────┐    ┌────────────────────┐  │
                     │  │  Tauri core  │    │  React UI (Vite)   │  │
                     │  │  (Rust)      │◀──▶│  feature-sliced    │  │
                     │  │  tray/notif  │    │  Zustand + TQ      │  │
                     │  └─────┬────────┘    └──────────┬─────────┘  │
                     └────────┼─────────────────────────┼───────────┘
                              │                         │
                       OS APIs│                         │ HTTPS + WSS
                              ▼                         ▼
                                       ┌─────────────────────────┐
                                       │       Axum server       │
                                       │   (REST + WS gateway)   │
                                       │                         │
                                       │  ┌──────┐  ┌─────────┐  │
                                       │  │ REST │  │   WS    │  │
                                       │  │ API  │  │ gateway │  │
                                       │  └──┬───┘  └────┬────┘  │
                                       │     │           │       │
                                       │  ┌──▼───────────▼────┐  │
                                       │  │   Application     │  │
                                       │  │      Layer        │  │
                                       │  └──┬─────────────┬──┘  │
                                       │     │             │     │
                                       │  ┌──▼──┐    ┌─────▼──┐  │
                                       │  │ sqlx│    │ redis  │  │
                                       │  │     │    │ client │  │
                                       │  └──┬──┘    └────┬───┘  │
                                       └─────┼────────────┼──────┘
                                             │            │
                                             ▼            ▼
                                       ┌─────────┐  ┌──────────┐
                                       │Postgres │  │  Redis   │
                                       │(messages│  │(presence,│
                                       │ users,  │  │ pub/sub) │
                                       │ chans)  │  │          │
                                       └─────────┘  └──────────┘
```

Two stores by design. Postgres owns durable truth: users, workspaces, channels, messages, threads, reactions. Redis owns ephemeral state and the pub/sub bus that lets the WS gateway scale horizontally without a sticky session: presence heartbeats and TTL, and message fan-out between gateway nodes when a message lands on one node but a recipient is connected to another. We use Redis for what it is good at (millisecond key/value + pub/sub) and Postgres for what it is good at (relational integrity, durable history, search-adjacency).

## Frontend architecture — Feature-Sliced Design

The React UI is organised by user-facing feature, not by tech layer. This is the **Feature-Sliced Design (FSD)** convention, applied conservatively.

```
frontend/src/
├── app/                 layers global concerns: providers, routing, error boundary, theme
├── pages/               (light) route components; mostly compose features
├── features/            user-facing capabilities — each is self-contained
│   ├── auth/                login, register, session refresh
│   ├── chat/                composer, message list, reactions, threads
│   ├── channels/            channel list, create/edit, switcher
│   ├── presence/            presence dot, status menu
│   └── notifications/       toasts + native notification bridge
├── entities/            typed domain mirror (User, Channel, Message…) and small CRUD pieces
├── shared/              ui kit, hooks, lib, api client, ws client
```

Dependency rule: a feature can depend on entities and shared, but **a feature cannot import another feature directly**. Cross-feature work is composed in `app/` or `pages/`. This prevents the kind of import spaghetti that turns a 20-feature codebase into mud over time.

Key client-side libraries and why:

- **Zustand** owns local + transient state (current channel, draft text, modal visibility). Tiny (~1 KB gz), no provider boilerplate.
- **TanStack Query** owns server state (cache, revalidation, optimistic updates). Pairs naturally with WS push: WS messages mutate the query cache via `queryClient.setQueryData`, the UI re-renders, no separate state copy.
- **react-virtuoso** is mandatory for the message list. Without virtualisation, a busy channel with 10k+ messages turns into a 1 GB DOM tree. With it, only the visible window is mounted; memory stays flat.
- **Tailwind** keeps the CSS surface narrow and JIT-purged. We do not ship a CSS-in-JS runtime.
- **Lucide-react** for icons. Pure SVG, tree-shakeable.

### Memory and CPU tactics on the frontend

1. **Virtualised lists** for messages (`react-virtuoso`) and channel lists (`react-virtuoso` for > 200 channels; plain map below that).
2. **Window-bounded queries**. We never fetch "all messages in channel X." A query is parameterised by `(channel_id, cursor, limit=50)`; older messages page in on scroll.
3. **WS reconciliation, not re-fetch**. When a WS frame for message-created arrives, we mutate the relevant query cache page in place. No HTTP round-trip.
4. **Suspended off-screen routes**. React Router lazy-loads route bundles. The DM area is not loaded until the user navigates to a DM for the first time.
5. **No persistent images in memory**. Avatars are HTTP-cached by the OS; we render `<img>` not data URIs.
6. **CSS containment** (`contain: layout paint`) on each message row, which lets the browser skip recomputing the entire channel layout on a single-row update.
7. **Throttled typing indicators**. Composer sends "typing" pings at most once per 3s; receive-side debounce keeps the indicator stable.
8. **`requestIdleCallback`** for non-critical work (e.g., updating relative-time labels) so it never blocks the main thread.
9. **Web Workers** are deliberately not used. The hot paths are small; the cost of postMessage marshalling is not worth it at this scale. If the message list ever exceeds 100k visible candidates, revisit.

## Backend architecture — Hexagonal (ports + adapters)

The Rust backend is organised in four crates inside one Cargo workspace. The crate boundary is the architectural boundary.

```
backend/crates/
├── domain/      pure types, business rules, errors. No tokio, no sqlx, no axum, no time-of-day.
├── proto/       wire types: REST DTOs, WS frame layout, msgpack codecs.
├── infra/       adapters: sqlx repos, redis client, argon2 + JWT helpers, OS clock.
└── server/      composition root: builds the dependency graph, runs axum, mounts the WS gateway.
```

**Domain** contains entities (`User`, `Workspace`, `Channel`, `Message`, `Thread`, `Reaction`, `Presence`) and the *ports* (Rust traits) the application layer depends on: `UserRepo`, `MessageRepo`, `PresenceStore`, `EventBus`, `Clock`, `TokenIssuer`. Domain code is deterministic and unit-testable in pure Rust with no IO.

**Infra** implements those traits against concrete IO: `PgUserRepo`, `PgMessageRepo`, `RedisPresenceStore`, `RedisEventBus`, `Argon2Hasher`, `JwtIssuer`. Each adapter is replaceable; tests can swap in in-memory variants.

**Proto** is the wire schema. Keeping it in its own crate (instead of in `server/`) means we can later add a CLI or a benchmark binary that speaks the same protocol without dragging the whole server in.

**Server** is the composition root. It wires `Infra` impls into the application service, mounts Axum routes, and owns the WS gateway loop. This is the only crate that knows about all the others.

Dependency direction is unidirectional: `server → infra → domain` and `server → proto → domain`. **Domain depends on nothing.** That is the entire point.

### Backend module map (target end-state)

```
server/src/
├── main.rs                 // boot: load config, init tracing, build deps, run axum
├── app.rs                  // application service: orchestrates domain + infra
├── routes/
│   ├── auth.rs             // POST /auth/register, /auth/login, /auth/refresh
│   ├── workspaces.rs
│   ├── channels.rs
│   ├── dms.rs
│   ├── messages.rs
│   └── ws.rs               // GET /ws upgrade handler
├── gateway/
│   ├── mod.rs              // connection lifecycle, fan-out
│   ├── frame.rs            // opcode dispatch
│   └── presence_loop.rs    // heartbeat timer, TTL refresh
└── middleware/
    ├── auth.rs             // bearer JWT → UserId in request extensions
    ├── rate_limit.rs       // tower-governor with per-route policy
    └── tracing.rs          // request_id + span injection
```

### Memory and CPU tactics on the backend

1. **Tokio's multi-threaded runtime** with the work-stealing scheduler. No blocking work in tasks.
2. **sqlx** with a small pool (default 8 connections); per-route handler limits prevent connection exhaustion.
3. **Compile-time SQL verification** via `sqlx::query!` macros — no runtime SQL parsing.
4. **msgpack for WS frames** instead of JSON. Roughly 30–50% smaller on the wire and ~3× faster to parse for our typical payloads.
5. **Per-connection bounded mpsc channels** in the gateway. If a slow client falls behind, we drop them rather than buffering unboundedly.
6. **Presence is Redis-only**. No Postgres write per heartbeat. Heartbeats expire after 30s via Redis TTL.
7. **Bulk message inserts** are batched in a single transaction when multiple messages land within the same micro-window (rare on the demo but cheap to support).
8. **Tracing** is structured (JSON to stderr) and sampling-aware — we do not pay log-formatting cost on the hot path beyond what we need.
9. **No GraphQL**. REST + WS is leaner for a chat surface. The cognitive cost of a query language is not paid for by the variety of reads we have.

## Realtime architecture

The full protocol is in `docs/04-realtime-protocol.md`. At the architectural level: a single Axum route (`GET /ws`) handles upgrade. After auth, the gateway subscribes the connection to the relevant Redis pub/sub channels (one per workspace channel the user is a member of, plus one for DMs). Inbound WS frames are parsed, validated, and forwarded to the application service. Outbound events from the application service are published to Redis; every gateway node consuming that channel pushes the event to its connected sockets.

This means we can run *N* gateway nodes behind a load balancer with no sticky sessions, and messages still fan-out correctly. The trade-off is a single point of contention on Redis pub/sub; at the demo scale this is irrelevant, and at production scale we'd shard pub/sub channels by workspace.

## Cross-cutting concerns

### Authentication
Local accounts (email + password). Argon2id for password hashing (OWASP defaults: m=19 MB, t=2, p=1). JWT for session tokens: 24-hour access token, 30-day refresh token, refresh rotation on use. The JWT's `sub` is the user ID; everything else (workspace memberships, roles) is fetched on demand and cached for the duration of the request.

### Authorisation
RBAC at the workspace and channel level. Workspace roles: `owner | admin | member`. Channel roles: `member` (default) + `private` channels have an explicit member list. Authorisation checks live in the application service (`app.rs`), not in HTTP handlers; this lets the same checks apply to WS-frame-driven operations.

### Configuration
12-factor. All config via environment variables: `DATABASE_URL`, `REDIS_URL`, `JWT_SIGNING_KEY`, `BIND_ADDRESS`, `LOG_LEVEL`. A `.env.example` ships in `backend/`; secrets never appear in code.

### Observability
- **Logs**: `tracing` crate, JSON formatter, level `info` by default.
- **Traces**: OpenTelemetry hooks at every HTTP route and WS frame; exporter is no-op by default; turn on via `OTEL_EXPORTER_OTLP_ENDPOINT`.
- **Metrics**: not implemented in the lean demo; an explicit non-goal. Counter scaffolding documented for production extension.

### Error handling
The domain returns `Result<T, DomainError>` where `DomainError` is a small enum (`NotFound`, `Conflict`, `Forbidden`, `Invalid`, `Internal`). Each variant maps to a stable HTTP status code in the routes layer. WS frames map errors to a `ServerError` opcode with the same enum codes. **The frontend never parses error messages — it switches on the error code.**

### Tests
- **Unit tests** in `domain/` for any rule worth pinning (e.g., "you cannot react with the same emoji twice on the same message").
- **Integration tests** in `infra/` against a docker-compose Postgres + Redis.
- **HTTP/WS smoke tests** in `server/tests/`.
- **E2E** in `tests/e2e/` (Playwright) for the happy paths.

## Architecture Decision Records (ADRs) — pinned for this build

### ADR-001 — Tauri 2, not Electron
**Decision**: Tauri 2 for the desktop shell.
**Reasoning**: Idle RAM is the single biggest perceived-quality metric for chat clients. Tauri uses the OS-native webview (~50–100 MB) versus Electron's bundled Chromium (~200–400 MB). The binary is 5–10 MB vs. Electron's 80–150 MB. Rust core means we can extend the shell with native features (system tray, notifications) without a Node sidecar.
**Trade-off**: webview parity is OS-dependent (WebView2 on Windows, WKWebView on macOS, WebKitGTK on Linux). We pin a minimum version per OS in the Tauri config and verify in CI.
**Reconsider when**: we need features that the OS webviews don't support uniformly (e.g., WebCodecs in 2024 was uneven). For the lean scope this is not a concern.

### ADR-002 — Rust + Axum + Tokio, not Node, Go, or Python
**Decision**: Rust on the backend.
**Reasoning**: chat workloads are realtime fan-out, not heavy compute. The metric that matters is "tail latency per concurrent connection." Tokio + Axum gives us that with a small memory footprint (~30 MB resident at idle) and zero GC pauses. The same code base ships as a single static binary.
**Trade-off**: development velocity in Rust is slower than in TypeScript. We mitigate by keeping the surface small (REST + WS) and writing domain logic in pure types where the compiler does the heavy lifting.
**Reconsider when**: we add a feature with rich orchestration (e.g., bots, integrations) where a higher-level runtime would help. At that point an isolated Node side-service is fine; the Rust core stays.

### ADR-003 — WebSocket + msgpack, not WebSocket + JSON, not SSE, not gRPC-web
**Decision**: WebSocket with msgpack-encoded frames.
**Reasoning**: bidirectional realtime is the chat pattern. SSE is one-way, so it doubles the round-trips. gRPC-web has weak browser support for bidirectional streams. JSON over WS works but pays a 1.4–2× wire and parse cost vs. msgpack on chat-sized payloads. msgpack is small, fast, and has good Rust + TS libraries.
**Trade-off**: harder to debug with `curl`. We mitigate with a "debug" build flag on the server that logs decoded frames in human-readable form to stderr.
**Reconsider when**: bandwidth is no longer a concern (probably never).

### ADR-004 — Hexagonal in Rust, Feature-Sliced in React
**Decision**: Domain crate is IO-free; UI is organised by feature.
**Reasoning**: both conventions make change cheap. Hexagonal means we can swap Postgres for SQLite for tests in 50 lines. FSD means a new feature doesn't reach into the guts of every other feature.
**Trade-off**: more boilerplate up front than a flat layout. Worth it on day 1; pays back from day 30.
**Reconsider when**: never, for a codebase that anyone will maintain over a year.

### ADR-005 — Two stores: Postgres + Redis
**Decision**: durable truth in Postgres; ephemeral state and pub/sub in Redis.
**Reasoning**: presence heartbeats every 15 s × thousands of connections would crush Postgres if stored there. Redis is built for this. Pub/sub lets the WS gateway scale horizontally without sticky sessions.
**Trade-off**: two stores to operate. For a demo, both run as docker containers and add maybe 100 MB of resident memory between them. For production, the additional operational cost is justified by the order-of-magnitude improvement in tail latency.
**Reconsider when**: the deployment is constrained to a single VM with very little RAM. Then a single Postgres with materialised presence views and `LISTEN/NOTIFY` for pub/sub is workable; the operational tradeoff goes the other way.

### ADR-006 — Local auth, not SSO, for the lean demo
**Decision**: email + password + JWT.
**Reasoning**: SSO (SAML/OIDC) is the right answer in production but it adds an identity-provider integration that is out of scope for a 3-day demo. Local auth with strong hashing is functionally sufficient for demonstration and is the same shape (login → token → bearer) that an OIDC implementation would use, so we are not painting ourselves into a corner.
**Trade-off**: not production-ready. Documented.
**Reconsider when**: shipping for real. Then add `openidconnect` crate behind a feature flag and a second login button on the frontend; the application layer doesn't change.

### ADR-007 — No GraphQL, no tRPC, no gRPC
**Decision**: REST for the request/response surface; typed WS frames for realtime.
**Reasoning**: chat's request shape is narrow (CRUD + WS push). The variety of GraphQL exists to serve heterogeneous read patterns; we don't have those. A schema generator would cost cognitive load without paying back.
**Trade-off**: less type-safety between client and server out of the box. We mitigate with a small `proto` crate on the Rust side and a `shared/api/types.ts` mirror generated by hand on the TS side. Day 3 includes a checklist to keep them in sync.
**Reconsider when**: the surface widens past ~20 query shapes (then GraphQL becomes a win) or when we add many external integrations (then OpenAPI codegen).

### ADR-008 — `react-virtuoso`, not custom virtualisation
**Decision**: virtuoso for any list that can plausibly grow past 200 items.
**Reasoning**: writing correct virtualisation from scratch is fiddly (height measurement, sticky headers, scroll-to-bottom). Virtuoso is small, well-maintained, and handles message-list quirks (reverse pagination, scroll anchoring).
**Trade-off**: a ~25 KB gzipped library on the critical path. Cheaper than the bugs we would otherwise ship.
**Reconsider when**: never.

### ADR-009 — Zustand + TanStack Query, not Redux Toolkit
**Decision**: split server-state (TQ) from local-state (Zustand).
**Reasoning**: 90% of "state" in a chat app is server state with a cache. TQ does that one job well. Zustand for the remaining local/UI state is ~1 KB and has no boilerplate. RTK would couple both into the same machinery and pay a complexity tax.
**Trade-off**: developers used to Redux need a short orientation. Day 3 includes a one-paragraph "where does this state go?" decision tree.
**Reconsider when**: never, unless we add time-travel debugging as a hard requirement.

---

Day 1's other docs build on this one. `02-data-model.md` translates the entities sketched here into a Postgres DDL. `03-threat-model.md` examines this architecture for security weaknesses. `04-realtime-protocol.md` specifies the WS frame layout that ADR-003 sketches.
