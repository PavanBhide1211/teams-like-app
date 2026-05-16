FROM rust:1.78-slim-bookworm AS build
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Build dependencies first for cache efficiency.
COPY Cargo.toml ./Cargo.toml
COPY crates/domain/Cargo.toml crates/domain/Cargo.toml
COPY crates/proto/Cargo.toml  crates/proto/Cargo.toml
COPY crates/infra/Cargo.toml  crates/infra/Cargo.toml
COPY crates/server/Cargo.toml crates/server/Cargo.toml
RUN mkdir -p crates/domain/src crates/proto/src crates/infra/src crates/server/src \
    && echo "fn main() {}" > crates/server/src/main.rs \
    && echo "" > crates/domain/src/lib.rs \
    && echo "" > crates/proto/src/lib.rs \
    && echo "" > crates/infra/src/lib.rs
RUN cargo build --release -p server || true

# Real source
COPY crates ./crates
COPY migrations ./migrations
ENV SQLX_OFFLINE=true
RUN cargo build --release -p server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates wget && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/server /usr/local/bin/server
COPY --from=build /app/migrations /app/migrations
WORKDIR /app
EXPOSE 8000
CMD ["server"]
