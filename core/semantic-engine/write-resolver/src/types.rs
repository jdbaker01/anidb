//! Types for semantic write resolution.

use anidb_shared_types::saas_events::SaasEvent;
use serde::{Deserialize, Serialize};

/// A semantic write declaration from an agent.
///
/// Instead of specifying the exact event type and payload, the agent
/// declares what entity it wants to affect and the properties to set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteDeclaration {
    /// Natural language description of the write intent.
    pub intent: String,
    /// The entity type to write to (e.g., "SupportTicket", "Invoice").
    pub entity_type: String,
    /// Optional entity ID for updates.
    pub entity_id: Option<String>,
    /// Properties to set, as a JSON object matching the event payload.
    pub properties: serde_json::Value,
}

/// A resolved write — the validated, typed event ready to append.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedWrite {
    /// The typed SaaS event produced from the declaration.
    pub event: SaasEvent,
    /// The stream this event should be appended to.
    pub stream_name: String,
    /// The event type string for the event log.
    pub event_type: String,
    /// Any validation notes or warnings.
    pub validation_notes: Vec<String>,
}
