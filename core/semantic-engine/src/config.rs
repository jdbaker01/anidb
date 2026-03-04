//! Configuration for the Semantic Engine service.

use anyhow::{bail, Context, Result};

/// Which LLM provider to use at runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmProvider {
    Anthropic,
    OpenAI,
}

impl LlmProvider {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(LlmProvider::Anthropic),
            "openai" => Ok(LlmProvider::OpenAI),
            other => bail!("Unknown LLM_PROVIDER '{}'. Valid values: anthropic, openai", other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,

    // Provider selection
    pub llm_provider: LlmProvider,

    // Anthropic config (required when provider = anthropic)
    pub anthropic_api_key: Option<String>,
    pub anthropic_model: String,

    // OpenAI config (required when provider = openai)
    pub openai_api_key: Option<String>,
    pub openai_model: String,

    // Service URLs
    pub event_log_url: String,
    pub confidence_store_url: String,

    // Neo4j
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let llm_provider = LlmProvider::from_str(
            &std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string()),
        )?;

        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let openai_api_key = std::env::var("OPENAI_API_KEY").ok();

        // Validate that the required key for the chosen provider is present
        match &llm_provider {
            LlmProvider::Anthropic => {
                if anthropic_api_key.is_none() {
                    bail!("ANTHROPIC_API_KEY must be set when LLM_PROVIDER=anthropic");
                }
            }
            LlmProvider::OpenAI => {
                if openai_api_key.is_none() {
                    bail!("OPENAI_API_KEY must be set when LLM_PROVIDER=openai");
                }
            }
        }

        Ok(Self {
            port: std::env::var("SEMANTIC_ENGINE_PORT")
                .unwrap_or_else(|_| "8001".to_string())
                .parse()
                .context("SEMANTIC_ENGINE_PORT must be a valid u16")?,
            llm_provider,
            anthropic_api_key,
            anthropic_model: std::env::var("ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-6".to_string()),
            openai_api_key,
            openai_model: std::env::var("OPENAI_MODEL")
                .unwrap_or_else(|_| "gpt-5.1".to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_provider_parses_case_insensitive() {
        assert_eq!(LlmProvider::from_str("anthropic").unwrap(), LlmProvider::Anthropic);
        assert_eq!(LlmProvider::from_str("ANTHROPIC").unwrap(), LlmProvider::Anthropic);
        assert_eq!(LlmProvider::from_str("openai").unwrap(), LlmProvider::OpenAI);
        assert_eq!(LlmProvider::from_str("OpenAI").unwrap(), LlmProvider::OpenAI);
        assert!(LlmProvider::from_str("gemini").is_err());
    }
}
