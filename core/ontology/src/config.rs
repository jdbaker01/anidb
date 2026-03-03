use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let neo4j_uri =
            std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
        let neo4j_user =
            std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
        let neo4j_password =
            std::env::var("NEO4J_PASSWORD").context("NEO4J_PASSWORD must be set")?;
        let port = std::env::var("ONTOLOGY_SERVICE_PORT")
            .unwrap_or_else(|_| "8002".to_string())
            .parse::<u16>()
            .context("ONTOLOGY_SERVICE_PORT must be a valid port")?;
        Ok(Self {
            neo4j_uri,
            neo4j_user,
            neo4j_password,
            port,
        })
    }
}
