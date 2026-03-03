use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum ConfidenceStoreError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Fact not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl IntoResponse for ConfidenceStoreError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ConfidenceStoreError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ConfidenceStoreError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ConfidenceStoreError::Database(e) => {
                tracing::error!(error = %e, "Database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            ConfidenceStoreError::Serialization(e) => {
                tracing::error!(error = %e, "Serialization error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        (status, message).into_response()
    }
}
