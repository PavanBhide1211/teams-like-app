# Cowork Chat — Teams-like Desktop App

A lean, memory-efficient, best-in-class chat client modelled on Microsoft Teams' message + channel surface. Built for desktop with **Tauri 2 + React 18 + TypeScript** on the front and **Rust + Axum + Tokio** on the back. Optimised for low idle RAM (~80 MB target) and steady CPU under sustained message load.

## Status

**Complete.** All three days delivered.

| Day | Scope | Status |
|---|---|---|
| 1 | Architecture, data model, threat model, realtime protocol, repo scaffold | **Delivered** |
| 2 | Rust backend: domain, repos, auth, REST endpoints | **Delivered** |
| 3 | WebSocket gateway, presence, React + Tauri UI, system tray, smoke test, **detailed user guide** | **Delivered** |

## Scope (locked)

**In scope** (lean subset)
- Workspaces and channels (public + private)
- 1:1 and group DMs
- Threaded messages, reactions, mentions
- Presence (online / away / dnd / offline)
- Realtime via WebSocket (binary msgpack frames)
- Local auth (username + password, JWT)
- System-tray icon + native OS notifications
- Memory and CPU optimisation as first-class concerns

**Out of scope** (deliberate for the demo)
- Voice / video / screen-share
- Calendar / meetings scheduling
- File attachments and full-text search (deferred)
- SSO / SAML / OIDC federation
- Mobile clients
- Compliance hold / e-discovery
- App marketplace / extensibility

## Tech stack

| Layer | Choice | Why |
|---|---|---|
| Desktop shell | **Tauri 2** | ~5 MB binary, ~50–100 MB RAM idle vs. Electron's 200–400 MB. Native webview. Rust core. |
| Frontend | **React 18 + TypeScript + Vite + Tailwind** | Fast dev, tree-shakeable, pairs cleanly with Tauri. |
| State | **Zustand + TanStack Query** | Tiny footprint vs. Redux; query cache + WS push composes naturally. |
| Virtualisation | **react-virtuoso** | 10k+ message rows without DOM bloat. |
| Backend | **Rust + Axum + Tokio** | Native concurrency, single binary, ~30 MB resident. Best-in-class for chat fan-out. |
| Realtime transport | **WebSocket + msgpack** | Lower wire size vs. JSON on the hot path. |
| Durable store | **PostgreSQL** | Industry default, excellent SQL surface. |
| Ephemeral store | **Redis** | Presence, pub/sub between gateway nodes. |
| Architecture | **Hexagonal (Rust) + Feature-Sliced (React)** | Domain free of IO leakage; UI organised by user-facing feature, not by tech layer. |
| Tests | **cargo test + vitest + Playwright** | Real test pyramid. |
| Containerisation | **docker-compose** for services; Tauri for the app | One-command spin-up. |

## Repository layout

```
teams-like-app/
├── README.md                        # this file
├── PROGRESS.md                      # rolling daily log
├── manifest.json                    # machine-readable plan + status
├── docs/
│   ├── 01-architecture.md           # ← Day 1
│   ├── 02-data-model.md             # ← Day 1
│   ├── 03-threat-model.md           # ← Day 1
│   ├── 04-realtime-protocol.md      # ← Day 1
│   └── 05-user-guide.md             # ← Day 3 (depends on the UI existing)
├── db/
│   └── schema.sql                   # ← Day 1
├── backend/                         # Rust workspace
│   ├── Cargo.toml                   # ← Day 2
│   ├── crates/
│   │   ├── domain/                  # ← Day 2 (pure types, no IO)
│   │   ├── infra/                   # ← Day 2 (sqlx, redis, smtp adapters)
│   │   ├── proto/                   # ← Day 2 (WS frame schema, msgpack codecs)
│   │   └── server/                  # ← Day 2 (axum app, route handlers, WS gateway)
│   └── migrations/                  # ← Day 2 (sqlx migrations)
├── frontend/                        # Tauri 2 + React app
│   ├── src-tauri/                   # ← Day 3 (Rust shell + tray + notifications)
│   ├── src/
│   │   ├── app/                     # ← Day 3 (routing, providers, global shell)
│   │   ├── features/                # ← Day 3 (auth/, chat/, channels/, presence/)
│   │   ├── shared/                  # ← Day 3 (ui kit, hooks, lib)
│   │   └── entities/                # ← Day 3 (typed domain mirror)
│   └── package.json                 # ← Day 3
├── docker-compose.yml               # ← Day 3
└── tests/
    └── e2e/                         # ← Day 3 (Playwright)
```

## How to read this repo

- **Architect / reviewer**: start with `docs/01-architecture.md`, then `02-data-model.md`, `03-threat-model.md`, `04-realtime-protocol.md` in that order. They are the deliverables of Day 1 and the design contract for the rest.
- **Backend engineer**: read `docs/01-architecture.md` § "Backend" and `docs/02-data-model.md`. Then `backend/README.md` (lands Day 2).
- **Frontend engineer**: read `docs/01-architecture.md` § "Frontend" and `docs/04-realtime-protocol.md`. Then `frontend/README.md` (lands Day 3).
- **End user**: read `docs/05-user-guide.md` (lands Day 3) — installation, first-run, sending a message, creating channels, customising notifications, troubleshooting.

## Quick start (will work end of Day 3)

```bash
# Backend services
docker compose up -d postgres redis

# Backend
cd backend
cargo run -p server

# Frontend (Tauri dev mode)
cd ../frontend
npm install
npm run tauri dev
```

## License and disclaimer

Demo codebase. Not affiliated with Microsoft Teams. Use of the name "Teams" is descriptive only.
