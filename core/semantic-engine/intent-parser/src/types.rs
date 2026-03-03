//! Types for the intent parser's structured output.

use serde::{Deserialize, Serialize};

/// The structured output from LLM intent parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedIntent {
    /// The resolved decision class.
    pub decision_class: DecisionClass,
    /// Entity references extracted from the intent.
    pub entity_refs: Vec<EntityRef>,
    /// Time horizon for the query.
    pub time_horizon: TimeHorizon,
    /// Minimum confidence threshold (0.0 to 1.0).
    pub min_confidence: f64,
    /// What specific data points the intent requires.
    pub required_data: Vec<RequiredDataPoint>,
    /// The LLM's interpretation of the intent in plain language.
    pub interpretation: String,
}

/// The three decision classes supported by the SaaS archetype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionClass {
    ChurnIntervention,
    Pricing,
    CapacityInventory,
}

impl DecisionClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            DecisionClass::ChurnIntervention => "churn_intervention",
            DecisionClass::Pricing => "pricing",
            DecisionClass::CapacityInventory => "capacity_inventory",
        }
    }
}

impl std::fmt::Display for DecisionClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A reference to a specific entity mentioned in the intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRef {
    /// The entity type: "Customer", "Plan", "Feature", etc.
    pub entity_type: String,
    /// The entity identifier (UUID or name).
    pub identifier: String,
}

/// Time horizon for the query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeHorizon {
    /// How many days of historical data to consider.
    pub lookback_days: u32,
    /// How many days forward to forecast.
    pub forecast_days: u32,
}

/// A data point the intent requires from a specific source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredDataPoint {
    /// Which storage layer this data comes from.
    pub source: DataSource,
    /// Human-readable description of what data is needed.
    pub description: String,
}

/// The storage layers available for query plan generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    KnowledgeGraph,
    EventLog,
    ConfidenceStore,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_class_serde_roundtrip() {
        let dc = DecisionClass::ChurnIntervention;
        let json = serde_json::to_string(&dc).unwrap();
        assert_eq!(json, "\"churn_intervention\"");
        let parsed: DecisionClass = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, dc);
    }

    #[test]
    fn parsed_intent_serde_roundtrip() {
        let intent = ParsedIntent {
            decision_class: DecisionClass::Pricing,
            entity_refs: vec![EntityRef {
                entity_type: "Customer".to_string(),
                identifier: "cust-123".to_string(),
            }],
            time_horizon: TimeHorizon {
                lookback_days: 30,
                forecast_days: 90,
            },
            min_confidence: 0.6,
            required_data: vec![RequiredDataPoint {
                source: DataSource::KnowledgeGraph,
                description: "Customer subscription details".to_string(),
            }],
            interpretation: "Find pricing optimization opportunities".to_string(),
        };
        let json = serde_json::to_string(&intent).unwrap();
        let parsed: ParsedIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.decision_class, DecisionClass::Pricing);
        assert_eq!(parsed.entity_refs.len(), 1);
        assert_eq!(parsed.time_horizon.lookback_days, 30);
    }

    #[test]
    fn data_source_serde_roundtrip() {
        let sources = vec![
            DataSource::KnowledgeGraph,
            DataSource::EventLog,
            DataSource::ConfidenceStore,
        ];
        for src in sources {
            let json = serde_json::to_string(&src).unwrap();
            let _: DataSource = serde_json::from_str(&json).unwrap();
        }
    }
}
