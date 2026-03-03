use anyhow::{Context, Result};

pub struct Config {
    pub database_url: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL must be set")?;
        let port = std::env::var("CONFIDENCE_STORE_PORT")
            .unwrap_or_else(|_| "8003".to_string())
            .parse()
            .context("CONFIDENCE_STORE_PORT must be a valid port number")?;
        Ok(Self { database_url, port })
    }
}
