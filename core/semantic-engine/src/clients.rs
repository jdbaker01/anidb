//! HTTP clients for the Event Log and Confidence Store services.
//!
//! The semantic engine calls these services over HTTP to fetch events and
//! confidence-weighted facts during query plan execution.

use anidb_shared_types::{Event, FactRecord};
use serde::Deserialize;
use uuid::Uuid;

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Service returned {status}: {body}")]
    Service { status: u16, body: String },

    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

// ============================================================================
// Event Log Client (port 8010)
// ============================================================================

#[derive(Debug, Clone)]
pub struct EventLogClient {
    http: reqwest::Client,
    base_url: String,
}

/// Matches the Event Log's ReadStreamResponse.
#[derive(Debug, Deserialize)]
struct ReadStreamResponse {
    #[allow(dead_code)]
    stream_id: String,
    events: Vec<Event>,
    #[allow(dead_code)]
    count: usize,
}

/// Matches the Event Log's ReadCategoryResponse.
#[derive(Debug, Deserialize)]
struct ReadCategoryResponse {
    #[allow(dead_code)]
    category: String,
    events: Vec<Event>,
    #[allow(dead_code)]
    count: usize,
}

impl EventLogClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
        }
    }

    /// Read all events from a specific stream (e.g., "customer-{uuid}").
    pub async fn read_stream(&self, stream_name: &str) -> Result<Vec<Event>, ClientError> {
        let url = format!("{}/streams/{}", self.base_url, stream_name);
        let resp = self.http.get(&url).send().await?;

        let status = resp.status().as_u16();
        if status >= 400 {
            let body = resp.text().await.unwrap_or_default();
            // 404 means no events — return empty, not error
            if status == 404 {
                return Ok(vec![]);
            }
            return Err(ClientError::Service { status, body });
        }

        let response: ReadStreamResponse = resp
            .json()
            .await
            .map_err(|e| ClientError::Deserialization(e.to_string()))?;
        Ok(response.events)
    }

    /// Read events by category (e.g., "customer" reads all customer-* streams).
    pub async fn read_category(&self, category: &str) -> Result<Vec<Event>, ClientError> {
        let url = format!("{}/categories/{}", self.base_url, category);
        let resp = self.http.get(&url).send().await?;

        let status = resp.status().as_u16();
        if status >= 400 {
            let body = resp.text().await.unwrap_or_default();
            if status == 404 {
                return Ok(vec![]);
            }
            return Err(ClientError::Service { status, body });
        }

        let response: ReadCategoryResponse = resp
            .json()
            .await
            .map_err(|e| ClientError::Deserialization(e.to_string()))?;
        Ok(response.events)
    }
}

// ============================================================================
// Confidence Store Client (port 8003)
// ============================================================================

#[derive(Debug, Clone)]
pub struct ConfidenceStoreClient {
    http: reqwest::Client,
    base_url: String,
}

/// Matches the Confidence Store's FactsResponse.
#[derive(Debug, Deserialize)]
struct FactsResponse {
    facts: Vec<FactRecord>,
    #[allow(dead_code)]
    count: usize,
}

impl ConfidenceStoreClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
        }
    }

    /// Get all confidence-weighted facts for a specific entity.
    pub async fn get_entity_facts(&self, entity_id: Uuid) -> Result<Vec<FactRecord>, ClientError> {
        let url = format!("{}/entities/{}/facts", self.base_url, entity_id);
        let resp = self.http.get(&url).send().await?;

        let status = resp.status().as_u16();
        if status >= 400 {
            let body = resp.text().await.unwrap_or_default();
            if status == 404 {
                return Ok(vec![]);
            }
            return Err(ClientError::Service { status, body });
        }

        let response: FactsResponse = resp
            .json()
            .await
            .map_err(|e| ClientError::Deserialization(e.to_string()))?;
        Ok(response.facts)
    }

    /// Get facts for multiple entities in a single request.
    pub async fn get_bulk_facts(
        &self,
        entity_ids: &[Uuid],
    ) -> Result<Vec<FactRecord>, ClientError> {
        let ids_str = entity_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let url = format!("{}/facts/bulk?entity_ids={}", self.base_url, ids_str);
        let resp = self.http.get(&url).send().await?;

        let status = resp.status().as_u16();
        if status >= 400 {
            let body = resp.text().await.unwrap_or_default();
            return Err(ClientError::Service { status, body });
        }

        let response: FactsResponse = resp
            .json()
            .await
            .map_err(|e| ClientError::Deserialization(e.to_string()))?;
        Ok(response.facts)
    }

    /// Get all facts for a given entity type (e.g., "Customer").
    pub async fn get_facts_by_type(
        &self,
        entity_type: &str,
    ) -> Result<Vec<FactRecord>, ClientError> {
        let url = format!("{}/entity-type/{}/facts", self.base_url, entity_type);
        let resp = self.http.get(&url).send().await?;

        let status = resp.status().as_u16();
        if status >= 400 {
            let body = resp.text().await.unwrap_or_default();
            if status == 404 {
                return Ok(vec![]);
            }
            return Err(ClientError::Service { status, body });
        }

        let response: FactsResponse = resp
            .json()
            .await
            .map_err(|e| ClientError::Deserialization(e.to_string()))?;
        Ok(response.facts)
    }
}
