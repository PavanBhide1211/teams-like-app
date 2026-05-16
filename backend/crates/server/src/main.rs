//! Cowork Chat backend — boot.

mod error;
mod gateway;
mod middleware;
mod routes;
mod state;

use std::net::SocketAddr;

use axum::{routing::get, Json, Router};
use tower_http::{cors::{Any, CorsLayer}, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use infra::{token::JwtIssuer, PgPool, RedisClient};
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ---- tracing
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,cowork=debug"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // ---- config
    let db_url    = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://cowork:cowork@localhost:5432/cowork".into());
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
    let bind      = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8000".into());

    tracing::info!(%db_url, %redis_url, %bind, "starting cowork chat server");

    // ---- deps
    let pg = PgPool::connect(&db_url).await?;
    pg.migrate().await?;
    let redis = RedisClient::connect(&redis_url).await?;
    let jwt = JwtIssuer::from_env()?;

    let state = AppState::build(pg, redis, jwt);

    // ---- router
    let app = Router::new()
        .route("/health", get(|| async { Json(serde_json::json!({"status":"ok"})) }))
        .merge(routes::auth::router())
        .merge(routes::workspaces::router())
        .merge(routes::channels::router())
        .merge(routes::dms::router())
        .merge(routes::messages::router())
        .merge(routes::messages::reactions_router())
        .merge(gateway::router())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr: SocketAddr = bind.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
