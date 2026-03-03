mod archetypes;
mod config;
mod error;
mod handlers;
mod models;
mod primitives;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use anidb_knowledge_graph::GraphClient;

use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config = Config::from_env()?;

    let graph_client = GraphClient::new(
        &config.neo4j_uri,
        &config.neo4j_user,
        &config.neo4j_password,
    )
    .await?;

    let state: handlers::AppState = Arc::new(graph_client);

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ontology/entities", get(handlers::list_entity_types))
        .route(
            "/ontology/entities/{type_name}",
            get(handlers::get_entity_type),
        )
        .route(
            "/ontology/relationships",
            get(handlers::list_relationships),
        )
        .route("/ontology/causal-links", get(handlers::list_causal_links))
        .route("/ontology/version", get(handlers::get_version))
        .route("/ontology/initialize", post(handlers::initialize))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(port = config.port, "Ontology Service listening");
    axum::serve(listener, app).await?;

    Ok(())
}
