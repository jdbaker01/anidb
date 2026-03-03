use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum EventLogError {
    #[error("Event validation failed: {0}")]
    Validation(String),

    #[error("Stream not found: {0}")]
    StreamNotFound(String),

    #[error("EventStoreDB error: {0}")]
    Store(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl IntoResponse for EventLogError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            EventLogError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            EventLogError::StreamNotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            EventLogError::Store(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            EventLogError::Serialization(e) => (StatusCode::BAD_REQUEST, e.to_string()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
