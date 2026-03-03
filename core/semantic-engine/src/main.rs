//! Semantic Engine service — the LLM-backed intelligence core of ANIDB.
//!
//! Orchestrates intent parsing, query planning, multi-source data retrieval,
//! and context bundling with causal narrative generation.

mod anthropic;
mod clients;
mod config;
mod error;
mod handlers;
mod pipeline;
mod state;

use std::sync::Arc;

use anyhow::Result;
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use anidb_knowledge_graph::GraphClient;

use crate::anthropic::AnthropicClient;
use crate::clients::{ConfidenceStoreClient, EventLogClient};
use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config = Config::from_env()?;

    // Initialize the Anthropic API client
    let anthropic_client = AnthropicClient::new(
        config.anthropic_api_key.clone(),
        config.anthropic_model.clone(),
    );
    tracing::info!(model = %config.anthropic_model, "Anthropic client initialized");

    // Initialize service clients
    let event_log = EventLogClient::new(config.event_log_url.clone());
    tracing::info!(url = %config.event_log_url, "Event Log client initialized");

    let confidence_store = ConfidenceStoreClient::new(config.confidence_store_url.clone());
    tracing::info!(url = %config.confidence_store_url, "Confidence Store client initialized");

    // Connect to Neo4j
    let graph = GraphClient::new(&config.neo4j_uri, &config.neo4j_user, &config.neo4j_password)
        .await?;
    tracing::info!(uri = %config.neo4j_uri, "Knowledge Graph client connected");

    // Build shared state
    let state = Arc::new(AppState {
        anthropic: anthropic_client,
        event_log,
        confidence_store,
        graph,
    });

    // Build router
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/intent-read", post(handlers::intent_read))
        .route("/intent-write", post(handlers::intent_write))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(port = config.port, "Semantic Engine listening");
    axum::serve(listener, app).await?;

    Ok(())
}
