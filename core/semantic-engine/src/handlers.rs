//! HTTP route handlers for the Semantic Engine.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use anidb_shared_types::intent::{ContextBundle, IntentQuery};
use anidb_write_resolver::WriteDeclaration;

use crate::error::SemanticEngineError;
use crate::pipeline;
use crate::state::AppState;

pub type SharedState = Arc<AppState>;

/// POST /intent-read
///
/// Receives an IntentQuery from the API gateway and returns a ContextBundle
/// with confidence-weighted facts and a causal narrative.
pub async fn intent_read(
    State(state): State<SharedState>,
    Json(query): Json<IntentQuery>,
) -> Result<Json<ContextBundle>, SemanticEngineError> {
    if query.intent.trim().is_empty() {
        return Err(SemanticEngineError::Validation(
            "intent must not be empty".to_string(),
        ));
    }

    let bundle = pipeline::process_intent_read(&state, query).await?;
    Ok(Json(bundle))
}

/// POST /intent-write
///
/// Receives a WriteDeclaration and resolves it into a typed SaaS event.
/// Returns the resolved write with the event, stream name, and validation notes.
pub async fn intent_write(
    State(_state): State<SharedState>,
    Json(decl): Json<WriteDeclaration>,
) -> Result<Json<anidb_write_resolver::types::ResolvedWrite>, SemanticEngineError> {
    if decl.entity_type.trim().is_empty() {
        return Err(SemanticEngineError::Validation(
            "entity_type must not be empty".to_string(),
        ));
    }

    let resolved = anidb_write_resolver::resolve_write(&decl)?;
    Ok(Json(resolved))
}
