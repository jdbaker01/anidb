use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub eventstore_uri: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let eventstore_uri = std::env::var("EVENTSTORE_URI")
            .unwrap_or_else(|_| "esdb://localhost:2113?tls=false".to_string());
        let port = std::env::var("EVENT_LOG_PORT")
            .unwrap_or_else(|_| "8010".to_string())
            .parse::<u16>()
            .context("EVENT_LOG_PORT must be a valid u16")?;
        Ok(Self { eventstore_uri, port })
    }
}
