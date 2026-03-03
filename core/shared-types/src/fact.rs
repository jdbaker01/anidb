use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::confidence::ConfidenceScore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactRecord {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub entity_type: String,
    pub fact_key: String,
    pub fact_value: serde_json::Value,
    pub confidence: ConfidenceScore,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
