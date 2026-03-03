use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;

use crate::client::EventStoreClient;
use crate::error::EventLogError;
use crate::models::*;
use crate::schema;

pub type AppState = Arc<EventStoreClient>;

pub async fn append_event(
    State(client): State<AppState>,
    Json(req): Json<AppendEventRequest>,
) -> Result<Json<AppendEventResponse>, EventLogError> {
    if !schema::is_valid_event_type(&req.event_type) {
        return Err(EventLogError::Validation(format!(
            "Unknown event type: {}",
            req.event_type
        )));
    }

    let event_id = client
        .append(&req.stream_id, &req.event_type, &req.payload, &req.metadata)
        .await?;

    tracing::info!(
        event_id = %event_id,
        stream = %req.stream_id,
        event_type = %req.event_type,
        "Event appended"
    );

    Ok(Json(AppendEventResponse {
        event_id,
        stream_id: req.stream_id,
    }))
}

pub async fn append_batch(
    State(client): State<AppState>,
    Json(req): Json<AppendBatchRequest>,
) -> Result<Json<AppendBatchResponse>, EventLogError> {
    let mut event_ids = Vec::with_capacity(req.events.len());

    for event in &req.events {
        if !schema::is_valid_event_type(&event.event_type) {
            return Err(EventLogError::Validation(format!(
                "Unknown event type: {}",
                event.event_type
            )));
        }
        let id = client
            .append(
                &event.stream_id,
                &event.event_type,
                &event.payload,
                &event.metadata,
            )
            .await?;
        event_ids.push(id);
    }

    let count = event_ids.len();
    tracing::info!(count = count, "Batch appended");
    Ok(Json(AppendBatchResponse { event_ids, count }))
}

pub async fn read_stream(
    State(client): State<AppState>,
    Path(stream_name): Path<String>,
    Query(_params): Query<ReadStreamParams>,
) -> Result<Json<ReadStreamResponse>, EventLogError> {
    let events = client.read_stream(&stream_name).await?;
    let count = events.len();
    Ok(Json(ReadStreamResponse {
        stream_id: stream_name,
        events,
        count,
    }))
}

pub async fn read_category(
    State(client): State<AppState>,
    Path(category_name): Path<String>,
) -> Result<Json<ReadCategoryResponse>, EventLogError> {
    let events = client.read_by_category(&category_name).await?;
    let count = events.len();
    Ok(Json(ReadCategoryResponse {
        category: category_name,
        events,
        count,
    }))
}
