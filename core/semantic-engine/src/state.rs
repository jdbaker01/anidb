//! Shared application state for the Semantic Engine service.

use anidb_knowledge_graph::GraphClient;

use crate::anthropic::AnthropicClient;
use crate::clients::{ConfidenceStoreClient, EventLogClient};

/// All service clients bundled together. Wrapped in `Arc` by the Axum router.
#[derive(Clone)]
pub struct AppState {
    pub anthropic: AnthropicClient,
    pub event_log: EventLogClient,
    pub confidence_store: ConfidenceStoreClient,
    pub graph: GraphClient,
}
