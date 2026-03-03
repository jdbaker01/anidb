use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyEntity {
    pub id: Uuid,
    pub entity_type: String,
    pub archetype: String,
    pub properties: serde_json::Value,
    pub causal_links: Vec<CausalLink>,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalLink {
    pub target_type: String,
    pub relationship: String,
    pub strength: f64,
}
