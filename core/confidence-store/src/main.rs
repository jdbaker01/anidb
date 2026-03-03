mod config;
mod db;
mod error;
mod handlers;
mod models;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::db::FactRepository;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config = Config::from_env()?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database migrations applied");

    let repo = FactRepository::new(pool);
    let state: handlers::AppState = Arc::new(repo);

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/facts", post(handlers::store_fact))
        .route("/facts/confidence", post(handlers::update_confidence))
        .route("/facts/bulk", get(handlers::get_bulk_facts))
        .route("/facts/type/{entity_type}", get(handlers::get_facts_by_type))
        .route("/facts/{id}", get(handlers::get_fact_by_id))
        .route(
            "/facts/entity/{entity_id}",
            get(handlers::get_facts_for_entity),
        )
        .route(
            "/facts/entity/{entity_id}/{key}",
            get(handlers::get_entity_fact),
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(port = config.port, "Confidence Store service listening");
    axum::serve(listener, app).await?;

    Ok(())
}
