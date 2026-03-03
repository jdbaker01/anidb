use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct StoreFactRequest {
    pub entity_id: Uuid,
    pub entity_type: String,
    pub fact_key: String,
    pub fact_value: serde_json::Value,
    pub confidence_value: f64,
    pub confidence_source: String,
    pub derivation: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfidenceRequest {
    pub fact_id: Uuid,
    pub confidence_value: f64,
    pub confidence_source: String,
    pub derivation: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct BulkFactsQuery {
    pub entity_ids: String,
}

#[derive(Debug, Serialize)]
pub struct FactResponse {
    pub fact: anidb_shared_types::FactRecord,
}

#[derive(Debug, Serialize)]
pub struct FactsResponse {
    pub facts: Vec<anidb_shared_types::FactRecord>,
    pub count: usize,
}
