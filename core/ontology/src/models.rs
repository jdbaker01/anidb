use serde::Serialize;

use crate::archetypes::saas::archetype::{
    CausalBelief, DecisionClassDef, SaasEntityDef, SaasRelationshipDef,
};

#[derive(Debug, Serialize)]
pub struct EntityTypesResponse {
    pub entity_types: Vec<SaasEntityDef>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct EntityTypeResponse {
    pub entity_type: SaasEntityDef,
}

#[derive(Debug, Serialize)]
pub struct RelationshipsResponse {
    pub relationships: Vec<SaasRelationshipDef>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct CausalLinksResponse {
    pub causal_beliefs: Vec<CausalBelief>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct DecisionClassesResponse {
    pub decision_classes: Vec<DecisionClassDef>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct OntologyVersionResponse {
    pub version: u32,
    pub archetype: String,
}

#[derive(Debug, Serialize)]
pub struct InitializeResponse {
    pub status: String,
    pub entity_types_created: usize,
    pub relationships_created: usize,
    pub causal_beliefs_created: usize,
}
