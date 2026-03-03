use serde::{Deserialize, Serialize};

use crate::confidence::ConfidenceScore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentQuery {
    pub intent: String,
    pub context: IntentContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentContext {
    pub decision_class: Option<String>,
    pub entity_refs: Vec<String>,
    pub time_horizon: Option<String>,
    pub min_confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBundle {
    pub decision_class: String,
    pub facts: Vec<Fact>,
    pub causal_context: String,
    pub suggested_queries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub key: String,
    pub value: serde_json::Value,
    pub confidence: ConfidenceScore,
}
