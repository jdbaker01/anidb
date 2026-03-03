mod client;
mod config;
mod error;
mod handlers;
mod models;
mod schema;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::client::EventStoreClient;
use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config = Config::from_env()?;

    let es_client = EventStoreClient::new(&config.eventstore_uri)?;
    let state: handlers::AppState = Arc::new(es_client);

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/events", post(handlers::append_event))
        .route("/events/batch", post(handlers::append_batch))
        .route("/streams/{stream_name}", get(handlers::read_stream))
        .route(
            "/categories/{category_name}",
            get(handlers::read_category),
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(port = config.port, "Event Log service listening");
    axum::serve(listener, app).await?;

    Ok(())
}
