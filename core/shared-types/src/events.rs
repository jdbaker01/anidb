use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub stream_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub metadata: EventMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub causation_id: Option<Uuid>,
    pub correlation_id: Uuid,
    pub ontology_version: u32,
}
