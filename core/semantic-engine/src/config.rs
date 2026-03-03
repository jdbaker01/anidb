//! Configuration for the Semantic Engine service.

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub anthropic_api_key: String,
    pub anthropic_model: String,
    pub event_log_url: String,
    pub confidence_store_url: String,
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: std::env::var("SEMANTIC_ENGINE_PORT")
                .unwrap_or_else(|_| "8001".to_string())
                .parse()
                .context("SEMANTIC_ENGINE_PORT must be a valid u16")?,
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY")
                .context("ANTHROPIC_API_KEY must be set")?,
            anthropic_model: std::env::var("ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string()),
            event_log_url: std::env::var("EVENT_LOG_URL")
                .unwrap_or_else(|_| "http://localhost:8010".to_string()),
            confidence_store_url: std::env::var("CONFIDENCE_STORE_URL")
                .unwrap_or_else(|_| "http://localhost:8003".to_string()),
            neo4j_uri: std::env::var("NEO4J_URI")
                .unwrap_or_else(|_| "bolt://localhost:7687".to_string()),
            neo4j_user: std::env::var("NEO4J_USER")
                .unwrap_or_else(|_| "neo4j".to_string()),
            neo4j_password: std::env::var("NEO4J_PASSWORD")
                .unwrap_or_else(|_| "anidb_dev".to_string()),
        })
    }
}
