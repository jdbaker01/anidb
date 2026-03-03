use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use anidb_knowledge_graph::{queries, schema, GraphClient};

use crate::archetypes::saas::archetype::build_saas_archetype;
use crate::error::OntologyError;
use crate::models::*;

pub type AppState = Arc<GraphClient>;

pub async fn list_entity_types(
    State(_client): State<AppState>,
) -> Result<Json<EntityTypesResponse>, OntologyError> {
    let arch = build_saas_archetype();
    let count = arch.entity_defs.len();
    Ok(Json(EntityTypesResponse {
        entity_types: arch.entity_defs,
        count,
    }))
}

pub async fn get_entity_type(
    State(_client): State<AppState>,
    Path(type_name): Path<String>,
) -> Result<Json<EntityTypeResponse>, OntologyError> {
    let arch = build_saas_archetype();
    let entity_type = arch
        .entity_defs
        .into_iter()
        .find(|e| e.name.eq_ignore_ascii_case(&type_name))
        .ok_or_else(|| OntologyError::NotFound(type_name))?;
    Ok(Json(EntityTypeResponse { entity_type }))
}

pub async fn list_relationships(
    State(_client): State<AppState>,
) -> Result<Json<RelationshipsResponse>, OntologyError> {
    let arch = build_saas_archetype();
    let count = arch.relationship_defs.len();
    Ok(Json(RelationshipsResponse {
        relationships: arch.relationship_defs,
        count,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CausalLinksQuery {
    pub decision_class: Option<String>,
}

pub async fn list_causal_links(
    State(_client): State<AppState>,
    Query(params): Query<CausalLinksQuery>,
) -> Result<Json<CausalLinksResponse>, OntologyError> {
    let arch = build_saas_archetype();
    let beliefs = match params.decision_class {
        Some(dc) => arch
            .causal_beliefs
            .into_iter()
            .filter(|b| b.decision_classes.contains(&dc))
            .collect::<Vec<_>>(),
        None => arch.causal_beliefs,
    };
    let count = beliefs.len();
    Ok(Json(CausalLinksResponse {
        causal_beliefs: beliefs,
        count,
    }))
}

pub async fn get_version(
    State(_client): State<AppState>,
) -> Result<Json<OntologyVersionResponse>, OntologyError> {
    let arch = build_saas_archetype();
    Ok(Json(OntologyVersionResponse {
        version: arch.version,
        archetype: arch.name,
    }))
}

/// Seed the full SaaS archetype into Neo4j.
pub async fn initialize(
    State(client): State<AppState>,
) -> Result<Json<InitializeResponse>, OntologyError> {
    let arch = build_saas_archetype();

    // Step 1: Initialize schema (constraints + indexes)
    schema::initialize_schema(&client).await?;

    // Step 2: Seed entity type definitions
    for entity_def in &arch.entity_defs {
        let properties_json = serde_json::to_string(&entity_def.properties)?;
        let source_events_json = serde_json::to_string(&entity_def.source_events)?;
        let rea_label = entity_def.rea_primitive.neo4j_label();
        client
            .run(queries::merge_entity_type_def(
                &entity_def.name,
                rea_label,
                &entity_def.description,
                &properties_json,
                &source_events_json,
                &arch.name,
                arch.version,
            ))
            .await?;
    }

    // Step 3: Seed relationship type definitions
    for rel_def in &arch.relationship_defs {
        let rea_type = rel_def.rea_relationship.neo4j_type();
        client
            .run(queries::merge_relationship_type_def(
                &rel_def.name,
                &rel_def.from_entity,
                &rel_def.to_entity,
                rea_type,
                &rel_def.description,
            ))
            .await?;
    }

    // Step 4: Seed causal beliefs
    for belief in &arch.causal_beliefs {
        let dc_json = serde_json::to_string(&belief.decision_classes)?;
        client
            .run(queries::merge_causal_belief(
                &belief.name,
                &belief.cause,
                &belief.effect,
                belief.strength,
                &dc_json,
                &belief.description,
            ))
            .await?;
    }

    // Step 5: Set ontology version
    client
        .run(queries::set_ontology_version(arch.version))
        .await?;

    tracing::info!(
        version = arch.version,
        entities = arch.entity_defs.len(),
        relationships = arch.relationship_defs.len(),
        causal_beliefs = arch.causal_beliefs.len(),
        "SaaS archetype initialized in Neo4j"
    );

    Ok(Json(InitializeResponse {
        status: "initialized".to_string(),
        entity_types_created: arch.entity_defs.len(),
        relationships_created: arch.relationship_defs.len(),
        causal_beliefs_created: arch.causal_beliefs.len(),
    }))
}
