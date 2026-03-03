//! Unified error type for the Semantic Engine service.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum SemanticEngineError {
    #[error("Intent parsing failed: {0}")]
    IntentParse(#[from] anidb_intent_parser::ParseError),

    #[error("Query planning failed: {0}")]
    QueryPlan(#[from] anidb_query_planner::PlanError),

    #[error("Context bundling failed: {0}")]
    ContextBundle(#[from] anidb_context_bundler::BundleError),

    #[error("Write resolution failed: {0}")]
    WriteResolve(#[from] anidb_write_resolver::ResolveError),

    #[error("Anthropic API error: {0}")]
    Anthropic(#[from] crate::anthropic::AnthropicError),

    #[error("Service client error: {0}")]
    Client(#[from] crate::clients::ClientError),

    #[error("Knowledge graph error: {0}")]
    Graph(#[from] anidb_knowledge_graph::GraphError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for SemanticEngineError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            SemanticEngineError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            SemanticEngineError::IntentParse(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
            }
            SemanticEngineError::WriteResolve(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            _ => {
                tracing::error!(error = %self, "Semantic engine error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
