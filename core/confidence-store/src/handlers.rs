use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::FactRepository;
use crate::error::ConfidenceStoreError;
use crate::models::*;

pub type AppState = Arc<FactRepository>;

pub async fn store_fact(
    State(repo): State<AppState>,
    Json(req): Json<StoreFactRequest>,
) -> Result<Json<FactResponse>, ConfidenceStoreError> {
    if req.confidence_value < 0.0 || req.confidence_value > 1.0 {
        return Err(ConfidenceStoreError::Validation(
            "confidence_value must be between 0.0 and 1.0".to_string(),
        ));
    }

    let derivation = req.derivation.unwrap_or_default();
    let fact = repo
        .upsert(
            req.entity_id,
            &req.entity_type,
            &req.fact_key,
            &req.fact_value,
            req.confidence_value,
            &req.confidence_source,
            &derivation,
        )
        .await?;

    tracing::info!(
        fact_id = %fact.id,
        entity_id = %fact.entity_id,
        fact_key = %fact.fact_key,
        confidence = fact.confidence.value,
        "Fact stored"
    );

    Ok(Json(FactResponse { fact }))
}

pub async fn update_confidence(
    State(repo): State<AppState>,
    Json(req): Json<UpdateConfidenceRequest>,
) -> Result<Json<FactResponse>, ConfidenceStoreError> {
    if req.confidence_value < 0.0 || req.confidence_value > 1.0 {
        return Err(ConfidenceStoreError::Validation(
            "confidence_value must be between 0.0 and 1.0".to_string(),
        ));
    }

    let derivation = req.derivation.unwrap_or_default();
    let fact = repo
        .update_confidence(
            req.fact_id,
            req.confidence_value,
            &req.confidence_source,
            &derivation,
        )
        .await?;

    tracing::info!(
        fact_id = %fact.id,
        new_confidence = fact.confidence.value,
        "Confidence updated"
    );

    Ok(Json(FactResponse { fact }))
}

pub async fn get_fact_by_id(
    State(repo): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<FactResponse>, ConfidenceStoreError> {
    let fact = repo.get_by_id(id).await?;
    Ok(Json(FactResponse { fact }))
}

pub async fn get_facts_for_entity(
    State(repo): State<AppState>,
    Path(entity_id): Path<Uuid>,
) -> Result<Json<FactsResponse>, ConfidenceStoreError> {
    let facts = repo.get_for_entity(entity_id).await?;
    let count = facts.len();
    Ok(Json(FactsResponse { facts, count }))
}

pub async fn get_entity_fact(
    State(repo): State<AppState>,
    Path((entity_id, key)): Path<(Uuid, String)>,
) -> Result<Json<FactResponse>, ConfidenceStoreError> {
    let fact = repo.get_fact(entity_id, &key).await?;
    Ok(Json(FactResponse { fact }))
}

pub async fn get_bulk_facts(
    State(repo): State<AppState>,
    Query(query): Query<BulkFactsQuery>,
) -> Result<Json<FactsResponse>, ConfidenceStoreError> {
    let entity_ids: Vec<Uuid> = query
        .entity_ids
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.trim()
                .parse::<Uuid>()
                .map_err(|_| ConfidenceStoreError::Validation(format!("Invalid UUID: {}", s)))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let facts = repo.get_bulk(&entity_ids).await?;
    let count = facts.len();
    Ok(Json(FactsResponse { facts, count }))
}

pub async fn get_facts_by_type(
    State(repo): State<AppState>,
    Path(entity_type): Path<String>,
) -> Result<Json<FactsResponse>, ConfidenceStoreError> {
    let facts = repo.get_by_type(&entity_type).await?;
    let count = facts.len();
    Ok(Json(FactsResponse { facts, count }))
}
