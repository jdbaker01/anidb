//! Types for multi-source query plans.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A complete plan for fetching data from all storage layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub decision_class: String,
    pub graph_queries: Vec<GraphQueryStep>,
    pub event_log_queries: Vec<EventLogQueryStep>,
    pub confidence_queries: Vec<ConfidenceQueryStep>,
}

/// A single graph query to execute against Neo4j.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryStep {
    /// Human-readable description of this query.
    pub description: String,
    /// The specific graph query to run.
    pub query_type: GraphQueryType,
}

/// Knowledge graph query variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphQueryType {
    /// Get full customer context (customer + plan + features + tickets + invoices).
    CustomerContext { customer_id: String },
    /// Get causal chain for a decision class, ordered by strength.
    CausalChain { decision_class: String },
    /// Find customers by status (e.g., "active", "churned").
    CustomersByStatus { status: String },
    /// List causal beliefs for a decision class.
    CausalBeliefs { decision_class: String },
}

/// A single event log query to execute against EventStoreDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLogQueryStep {
    /// Human-readable description of this query.
    pub description: String,
    /// The specific event log query to run.
    pub query_type: EventLogQueryType,
}

/// Event log query variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventLogQueryType {
    /// Read a specific stream (e.g., "customer-{uuid}").
    ReadStream { stream_name: String },
    /// Read events by category (e.g., "customer").
    ReadCategory { category: String },
}

/// A single confidence store query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceQueryStep {
    /// Human-readable description of this query.
    pub description: String,
    /// The specific confidence query to run.
    pub query_type: ConfidenceQueryType,
}

/// Confidence store query variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceQueryType {
    /// Get all facts for a specific entity.
    EntityFacts { entity_id: Uuid },
    /// Get facts for multiple entities.
    BulkFacts { entity_ids: Vec<Uuid> },
    /// Get all facts of a certain entity type.
    TypeFacts { entity_type: String },
}
