use anidb_shared_types::events::EventMetadata;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AppendEventRequest {
    pub stream_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub metadata: EventMetadata,
}

#[derive(Debug, Serialize)]
pub struct AppendEventResponse {
    pub event_id: Uuid,
    pub stream_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AppendBatchRequest {
    pub events: Vec<AppendEventRequest>,
}

#[derive(Debug, Serialize)]
pub struct AppendBatchResponse {
    pub event_ids: Vec<Uuid>,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct ReadStreamParams {
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ReadStreamResponse {
    pub stream_id: String,
    pub events: Vec<anidb_shared_types::Event>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct ReadCategoryResponse {
    pub category: String,
    pub events: Vec<anidb_shared_types::Event>,
    pub count: usize,
}
