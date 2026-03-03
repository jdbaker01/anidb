use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    pub value: f64,
    pub source: String,
    pub last_verified: DateTime<Utc>,
    pub derivation: Vec<Uuid>,
}
