# Backend — Cowork Chat

Rust + Axum + Tokio. Hexagonal layout in four crates.

```
crates/
├── domain/   pure types + ports (traits); zero IO; deterministic
├── proto/    wire types (REST DTOs, WS frames) + msgpack codecs
├── infra/    adapters: PgPool, sqlx repos, Redis, argon2, JWT
└── server/   composition root: axum app, routes, WS gateway
```

**Dependency direction**: `server → infra → domain`, `server → proto → domain`. Domain depends on nothing.

## Run

```bash
docker compose up -d postgres redis      # from repo root
cargo run -p server                       # this crate
```

## Configuration (env)

| Variable | Default | Purpose |
|---|---|---|
| `DATABASE_URL` | `postgres://cowork:cowork@localhost:5432/cowork` | Postgres connection |
| `REDIS_URL`    | `redis://localhost:6379`                          | Redis connection |
| `JWT_SIGNING_KEY` | *(required, no default)*                       | 256-bit HS256 key, base64 or raw |
| `BIND_ADDRESS` | `0.0.0.0:8000` | Where Axum listens |
| `LOG_LEVEL`    | `info` | `RUST_LOG`-style filter |
| `ARGON2_M_KB`  | `19456` | argon2id memory cost (KB), OWASP default |
| `ARGON2_T`     | `2`     | argon2id time cost |
| `ARGON2_P`     | `1`     | argon2id parallelism |
| `JWT_ACCESS_TTL_S`  | `86400`   | 24 h access token |
| `JWT_REFRESH_TTL_S` | `2592000` | 30 d refresh token |

## Tests

```bash
cargo test --workspace
```

Domain crate is unit-tested with no IO. Infra crate has integration tests that need a Postgres + Redis up.
