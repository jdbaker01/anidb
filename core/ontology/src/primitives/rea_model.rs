use serde::{Deserialize, Serialize};

/// REA Primitive Categories — the five universal business concepts.
/// Every business entity in any archetype maps to exactly one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaPrimitive {
    /// Agents who participate in economic events (people, orgs, systems)
    Party,
    /// Things of economic value (products, services, capacity)
    Resource,
    /// Changes in resource custody or rights (transactions, usage)
    EconomicEvent,
    /// Promised future economic events (contracts, subscriptions)
    Commitment,
    /// Where things happen (regions, endpoints, jurisdictions)
    Location,
}

impl ReaPrimitive {
    /// Returns the Neo4j node label for this primitive.
    pub fn neo4j_label(&self) -> &'static str {
        match self {
            ReaPrimitive::Party => "Party",
            ReaPrimitive::Resource => "Resource",
            ReaPrimitive::EconomicEvent => "EconomicEvent",
            ReaPrimitive::Commitment => "Commitment",
            ReaPrimitive::Location => "Location",
        }
    }

    /// All REA primitives, for iteration.
    pub fn all() -> &'static [ReaPrimitive] {
        &[
            ReaPrimitive::Party,
            ReaPrimitive::Resource,
            ReaPrimitive::EconomicEvent,
            ReaPrimitive::Commitment,
            ReaPrimitive::Location,
        ]
    }
}

impl std::fmt::Display for ReaPrimitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.neo4j_label())
    }
}

/// Standard REA relationship types between primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReaRelationship {
    /// Party participates in an EconomicEvent
    ParticipatesIn,
    /// EconomicEvent affects a Resource
    Affects,
    /// Commitment promises an EconomicEvent
    Promises,
    /// Party is responsible for a Commitment
    ResponsibleFor,
    /// EconomicEvent occurs at a Location
    OccursAt,
    /// Resource is located at a Location
    LocatedAt,
    /// Reciprocal: two EconomicEvents that form a transaction
    Reciprocal,
    /// An EconomicEvent fulfills a Commitment
    Fulfills,
}

impl ReaRelationship {
    /// Returns the Neo4j relationship type string.
    pub fn neo4j_type(&self) -> &'static str {
        match self {
            ReaRelationship::ParticipatesIn => "PARTICIPATES_IN",
            ReaRelationship::Affects => "AFFECTS",
            ReaRelationship::Promises => "PROMISES",
            ReaRelationship::ResponsibleFor => "RESPONSIBLE_FOR",
            ReaRelationship::OccursAt => "OCCURS_AT",
            ReaRelationship::LocatedAt => "LOCATED_AT",
            ReaRelationship::Reciprocal => "RECIPROCAL",
            ReaRelationship::Fulfills => "FULFILLS",
        }
    }
}

impl std::fmt::Display for ReaRelationship {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.neo4j_type())
    }
}

/// Describes a valid REA connection (from_primitive → relationship → to_primitive).
/// Used to validate that archetype relationships conform to REA rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaConnectionRule {
    pub from: ReaPrimitive,
    pub relationship: ReaRelationship,
    pub to: ReaPrimitive,
}

/// Returns the valid REA connection rules.
pub fn rea_connection_rules() -> Vec<ReaConnectionRule> {
    vec![
        ReaConnectionRule { from: ReaPrimitive::Party, relationship: ReaRelationship::ParticipatesIn, to: ReaPrimitive::EconomicEvent },
        ReaConnectionRule { from: ReaPrimitive::EconomicEvent, relationship: ReaRelationship::Affects, to: ReaPrimitive::Resource },
        ReaConnectionRule { from: ReaPrimitive::Commitment, relationship: ReaRelationship::Promises, to: ReaPrimitive::EconomicEvent },
        ReaConnectionRule { from: ReaPrimitive::Party, relationship: ReaRelationship::ResponsibleFor, to: ReaPrimitive::Commitment },
        ReaConnectionRule { from: ReaPrimitive::EconomicEvent, relationship: ReaRelationship::OccursAt, to: ReaPrimitive::Location },
        ReaConnectionRule { from: ReaPrimitive::Resource, relationship: ReaRelationship::LocatedAt, to: ReaPrimitive::Location },
        ReaConnectionRule { from: ReaPrimitive::EconomicEvent, relationship: ReaRelationship::Reciprocal, to: ReaPrimitive::EconomicEvent },
        ReaConnectionRule { from: ReaPrimitive::EconomicEvent, relationship: ReaRelationship::Fulfills, to: ReaPrimitive::Commitment },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_primitives_have_labels() {
        for p in ReaPrimitive::all() {
            assert!(!p.neo4j_label().is_empty());
        }
    }

    #[test]
    fn serde_roundtrip_primitive() {
        let p = ReaPrimitive::Party;
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"party\"");
        let back: ReaPrimitive = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn connection_rules_cover_all_relationship_types() {
        let rules = rea_connection_rules();
        assert_eq!(rules.len(), 8);
    }
}
