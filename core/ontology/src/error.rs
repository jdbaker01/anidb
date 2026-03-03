use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum OntologyError {
    #[error("Entity type not found: {0}")]
    NotFound(String),

    #[error("Graph error: {0}")]
    Graph(#[from] anidb_knowledge_graph::GraphError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Already initialized")]
    AlreadyInitialized,
}

impl IntoResponse for OntologyError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            OntologyError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            OntologyError::Graph(e) => {
                tracing::error!(error = %e, "Graph error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            OntologyError::Serialization(e) => {
                tracing::error!(error = %e, "Serialization error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            OntologyError::AlreadyInitialized => (
                StatusCode::CONFLICT,
                "Ontology already initialized".to_string(),
            ),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
