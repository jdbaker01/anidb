//! Types for context bundling — the intermediate data collected from all storage layers.

use anidb_shared_types::{Event, FactRecord};
use serde::{Deserialize, Serialize};

/// Raw results from executing a QueryPlan, before narrative generation.
#[derive(Debug, Clone, Default)]
pub struct QueryResults {
    /// Customer/entity context from the knowledge graph (serialized to JSON).
    pub graph_data: Vec<serde_json::Value>,
    /// Causal beliefs retrieved from the knowledge graph.
    pub causal_beliefs: Vec<CausalBeliefResult>,
    /// Raw events from the event log.
    pub events: Vec<Event>,
    /// Confidence-weighted facts from the confidence store.
    pub facts: Vec<FactRecord>,
}

/// A causal belief retrieved from Neo4j.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalBeliefResult {
    pub belief_name: String,
    pub cause: String,
    pub effect: String,
    pub strength: f64,
    pub description: String,
}
