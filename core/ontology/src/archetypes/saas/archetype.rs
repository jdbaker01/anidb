use serde::{Deserialize, Serialize};

use crate::primitives::rea_model::{ReaPrimitive, ReaRelationship};

/// A SaaS entity type definition. Metadata about entity types, not instances.
/// Instances live in Neo4j as nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaasEntityDef {
    /// The entity type name, used as a secondary Neo4j label (e.g., "Customer")
    pub name: String,
    /// Which REA primitive this maps to
    pub rea_primitive: ReaPrimitive,
    /// Human-readable description for the semantic engine
    pub description: String,
    /// Property schema definitions
    pub properties: Vec<PropertyDef>,
    /// Which SaaS event types can create/modify this entity
    pub source_events: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDef {
    pub name: String,
    pub property_type: PropertyType,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    Uuid,
    String,
    Integer,
    Float,
    Boolean,
    Timestamp,
    /// Stored as cents (i64)
    Currency,
}

/// A relationship definition between SaaS entity types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaasRelationshipDef {
    pub name: String,
    pub from_entity: String,
    pub to_entity: String,
    pub rea_relationship: ReaRelationship,
    pub description: String,
}

/// A causal belief: entity A's property/behavior influences entity B's property/behavior.
/// These are the core of ANIDB's intelligence — they tell the semantic engine
/// what factors to consider when assembling decision context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalBelief {
    pub name: String,
    /// The cause (entity_type.property or entity_type.behavior)
    pub cause: String,
    /// The effect (entity_type.property or entity_type.behavior)
    pub effect: String,
    /// Strength: -1.0 (strong negative) to 1.0 (strong positive)
    pub strength: f64,
    /// Which decision classes this belief is relevant to
    pub decision_classes: Vec<String>,
    /// Human-readable description for the semantic engine to use as context
    pub description: String,
}

/// A decision class that agents can query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionClassDef {
    pub name: String,
    pub description: String,
    /// Which entity types are relevant for this decision
    pub relevant_entities: Vec<String>,
    /// Which causal beliefs should be consulted
    pub relevant_causal_beliefs: Vec<String>,
}

/// The complete SaaS archetype definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaasArchetype {
    pub name: String,
    pub version: u32,
    pub entity_defs: Vec<SaasEntityDef>,
    pub relationship_defs: Vec<SaasRelationshipDef>,
    pub causal_beliefs: Vec<CausalBelief>,
    pub decision_classes: Vec<DecisionClassDef>,
}

/// Build the complete SaaS archetype definition.
/// This is the single source of truth for the SaaS domain model.
pub fn build_saas_archetype() -> SaasArchetype {
    SaasArchetype {
        name: "saas".to_string(),
        version: 1,
        entity_defs: build_entity_defs(),
        relationship_defs: build_relationship_defs(),
        causal_beliefs: build_causal_beliefs(),
        decision_classes: build_decision_classes(),
    }
}

fn build_entity_defs() -> Vec<SaasEntityDef> {
    vec![
        SaasEntityDef {
            name: "Customer".into(),
            rea_primitive: ReaPrimitive::Party,
            description: "A business or individual who subscribes to the SaaS product".into(),
            properties: vec![
                PropertyDef { name: "customer_id".into(), property_type: PropertyType::Uuid, description: "Unique customer identifier".into(), required: true },
                PropertyDef { name: "status".into(), property_type: PropertyType::String, description: "active | churned | trial".into(), required: true },
                PropertyDef { name: "mrr_cents".into(), property_type: PropertyType::Currency, description: "Current monthly recurring revenue in cents".into(), required: true },
                PropertyDef { name: "seat_count".into(), property_type: PropertyType::Integer, description: "Number of active seats".into(), required: true },
                PropertyDef { name: "subscribed_at".into(), property_type: PropertyType::Timestamp, description: "When the customer first subscribed".into(), required: false },
                PropertyDef { name: "last_login_at".into(), property_type: PropertyType::Timestamp, description: "Most recent login timestamp".into(), required: false },
                PropertyDef { name: "health_score".into(), property_type: PropertyType::Float, description: "Derived engagement/health score 0.0-1.0".into(), required: false },
            ],
            source_events: vec![
                "CustomerSubscribed".into(), "CustomerCancelled".into(),
                "LoginEvent".into(), "SeatCountChanged".into(),
                "TrialStarted".into(), "TrialConverted".into(),
            ],
        },
        SaasEntityDef {
            name: "Plan".into(),
            rea_primitive: ReaPrimitive::Resource,
            description: "A subscription plan offered by the SaaS product".into(),
            properties: vec![
                PropertyDef { name: "plan_id".into(), property_type: PropertyType::String, description: "Unique plan identifier (e.g., basic, pro, enterprise)".into(), required: true },
                PropertyDef { name: "price_cents".into(), property_type: PropertyType::Currency, description: "Monthly price in cents".into(), required: true },
            ],
            source_events: vec!["PriceChanged".into()],
        },
        SaasEntityDef {
            name: "Subscription".into(),
            rea_primitive: ReaPrimitive::Commitment,
            description: "The active commitment between a Customer and a Plan".into(),
            properties: vec![
                PropertyDef { name: "plan_id".into(), property_type: PropertyType::String, description: "Current plan".into(), required: true },
                PropertyDef { name: "mrr_cents".into(), property_type: PropertyType::Currency, description: "MRR for this subscription".into(), required: true },
                PropertyDef { name: "status".into(), property_type: PropertyType::String, description: "active | cancelled | trial".into(), required: true },
                PropertyDef { name: "started_at".into(), property_type: PropertyType::Timestamp, description: "Subscription start date".into(), required: true },
            ],
            source_events: vec![
                "CustomerSubscribed".into(), "CustomerCancelled".into(),
                "PlanChanged".into(), "TrialStarted".into(), "TrialConverted".into(),
            ],
        },
        SaasEntityDef {
            name: "Feature".into(),
            rea_primitive: ReaPrimitive::Resource,
            description: "A product feature that can be used and measured".into(),
            properties: vec![
                PropertyDef { name: "feature_name".into(), property_type: PropertyType::String, description: "Feature identifier".into(), required: true },
            ],
            source_events: vec!["FeatureUsage".into()],
        },
        SaasEntityDef {
            name: "Invoice".into(),
            rea_primitive: ReaPrimitive::EconomicEvent,
            description: "A billing event — payment attempt".into(),
            properties: vec![
                PropertyDef { name: "invoice_id".into(), property_type: PropertyType::Uuid, description: "Unique invoice identifier".into(), required: true },
                PropertyDef { name: "amount_cents".into(), property_type: PropertyType::Currency, description: "Invoice amount in cents".into(), required: true },
                PropertyDef { name: "status".into(), property_type: PropertyType::String, description: "paid | failed".into(), required: true },
            ],
            source_events: vec!["InvoicePaid".into(), "InvoiceFailed".into()],
        },
        SaasEntityDef {
            name: "SupportTicket".into(),
            rea_primitive: ReaPrimitive::EconomicEvent,
            description: "A customer support interaction".into(),
            properties: vec![
                PropertyDef { name: "ticket_id".into(), property_type: PropertyType::Uuid, description: "Unique ticket identifier".into(), required: true },
                PropertyDef { name: "priority".into(), property_type: PropertyType::String, description: "high | medium | low".into(), required: true },
                PropertyDef { name: "category".into(), property_type: PropertyType::String, description: "Ticket category".into(), required: true },
                PropertyDef { name: "status".into(), property_type: PropertyType::String, description: "open | closed".into(), required: true },
                PropertyDef { name: "satisfaction_score".into(), property_type: PropertyType::Integer, description: "CSAT score 1-5 (if closed)".into(), required: false },
            ],
            source_events: vec!["SupportTicketOpened".into(), "SupportTicketClosed".into()],
        },
        SaasEntityDef {
            name: "UsageMetric".into(),
            rea_primitive: ReaPrimitive::EconomicEvent,
            description: "A recorded usage measurement for a customer".into(),
            properties: vec![
                PropertyDef { name: "metric".into(), property_type: PropertyType::String, description: "Name of the metric (e.g., api_calls, storage_gb)".into(), required: true },
                PropertyDef { name: "value".into(), property_type: PropertyType::Float, description: "Measured value".into(), required: true },
                PropertyDef { name: "recorded_at".into(), property_type: PropertyType::Timestamp, description: "When the measurement was taken".into(), required: true },
            ],
            source_events: vec!["UsageRecorded".into(), "CapacityThresholdReached".into()],
        },
    ]
}

fn build_relationship_defs() -> Vec<SaasRelationshipDef> {
    vec![
        SaasRelationshipDef {
            name: "SUBSCRIBES_TO".into(),
            from_entity: "Customer".into(),
            to_entity: "Plan".into(),
            rea_relationship: ReaRelationship::ResponsibleFor,
            description: "Customer has an active subscription to a Plan".into(),
        },
        SaasRelationshipDef {
            name: "HAS_SUBSCRIPTION".into(),
            from_entity: "Customer".into(),
            to_entity: "Subscription".into(),
            rea_relationship: ReaRelationship::ResponsibleFor,
            description: "Customer owns a Subscription commitment".into(),
        },
        SaasRelationshipDef {
            name: "FOR_PLAN".into(),
            from_entity: "Subscription".into(),
            to_entity: "Plan".into(),
            rea_relationship: ReaRelationship::Promises,
            description: "Subscription is for a specific Plan".into(),
        },
        SaasRelationshipDef {
            name: "USES_FEATURE".into(),
            from_entity: "Customer".into(),
            to_entity: "Feature".into(),
            rea_relationship: ReaRelationship::ParticipatesIn,
            description: "Customer uses a Feature (with usage counts)".into(),
        },
        SaasRelationshipDef {
            name: "BILLED_TO".into(),
            from_entity: "Invoice".into(),
            to_entity: "Customer".into(),
            rea_relationship: ReaRelationship::Affects,
            description: "Invoice is billed to a Customer".into(),
        },
        SaasRelationshipDef {
            name: "OPENED_BY".into(),
            from_entity: "SupportTicket".into(),
            to_entity: "Customer".into(),
            rea_relationship: ReaRelationship::ParticipatesIn,
            description: "Support ticket was opened by a Customer".into(),
        },
        SaasRelationshipDef {
            name: "MEASURED_FOR".into(),
            from_entity: "UsageMetric".into(),
            to_entity: "Customer".into(),
            rea_relationship: ReaRelationship::Affects,
            description: "Usage metric was measured for a Customer".into(),
        },
    ]
}

fn build_causal_beliefs() -> Vec<CausalBelief> {
    vec![
        // --- Churn intervention ---
        CausalBelief {
            name: "declining_usage_causes_churn".into(),
            cause: "Customer.usage_trend".into(),
            effect: "Customer.churn_risk".into(),
            strength: 0.85,
            decision_classes: vec!["churn_intervention".into()],
            description: "Declining product usage over 30 days is a strong predictor of churn. Customers who reduce usage by >40% have 3x churn rate.".into(),
        },
        CausalBelief {
            name: "support_tickets_influence_satisfaction".into(),
            cause: "SupportTicket.frequency".into(),
            effect: "Customer.satisfaction".into(),
            strength: -0.65,
            decision_classes: vec!["churn_intervention".into()],
            description: "Frequent support tickets indicate friction. More than 3 tickets in 30 days correlates with low satisfaction.".into(),
        },
        CausalBelief {
            name: "failed_invoices_increase_churn".into(),
            cause: "Invoice.failure_count".into(),
            effect: "Customer.churn_risk".into(),
            strength: 0.70,
            decision_classes: vec!["churn_intervention".into()],
            description: "Repeated payment failures cause involuntary churn. 2+ failures in 60 days is a critical risk signal.".into(),
        },
        CausalBelief {
            name: "low_seat_utilization_predicts_downgrade".into(),
            cause: "Customer.seat_utilization".into(),
            effect: "Customer.churn_risk".into(),
            strength: 0.55,
            decision_classes: vec!["churn_intervention".into(), "pricing".into()],
            description: "Customers using <50% of their purchased seats are likely to downgrade or churn.".into(),
        },
        CausalBelief {
            name: "feature_breadth_reduces_churn".into(),
            cause: "Customer.feature_adoption_breadth".into(),
            effect: "Customer.churn_risk".into(),
            strength: -0.75,
            decision_classes: vec!["churn_intervention".into()],
            description: "Customers using 3+ features have significantly lower churn rates. Feature lock-in creates switching costs.".into(),
        },
        // --- Pricing ---
        CausalBelief {
            name: "usage_growth_signals_upgrade".into(),
            cause: "Customer.usage_trend".into(),
            effect: "Customer.upgrade_propensity".into(),
            strength: 0.80,
            decision_classes: vec!["pricing".into()],
            description: "Customers with >20% usage growth over 30 days are prime upgrade candidates.".into(),
        },
        CausalBelief {
            name: "capacity_threshold_drives_expansion".into(),
            cause: "UsageMetric.capacity_pct".into(),
            effect: "Customer.expansion_revenue_potential".into(),
            strength: 0.90,
            decision_classes: vec!["pricing".into(), "capacity_inventory".into()],
            description: "Customers at >80% capacity utilization have high expansion revenue potential and may need upsell.".into(),
        },
        CausalBelief {
            name: "price_sensitivity_from_plan_tier".into(),
            cause: "Plan.price_cents".into(),
            effect: "Customer.price_sensitivity".into(),
            strength: -0.50,
            decision_classes: vec!["pricing".into()],
            description: "Customers on lower-tier plans are more price-sensitive to increases. Enterprise customers tolerate price changes better.".into(),
        },
        // --- Capacity/inventory ---
        CausalBelief {
            name: "seat_growth_predicts_capacity_need".into(),
            cause: "Customer.seat_count_trend".into(),
            effect: "Customer.capacity_demand".into(),
            strength: 0.85,
            decision_classes: vec!["capacity_inventory".into()],
            description: "Increasing seat counts predict future capacity requirements. Plan ahead for 30-day seat growth trajectory.".into(),
        },
        CausalBelief {
            name: "usage_spikes_signal_scaling".into(),
            cause: "UsageMetric.value_variance".into(),
            effect: "Customer.scaling_urgency".into(),
            strength: 0.70,
            decision_classes: vec!["capacity_inventory".into()],
            description: "High variance in usage metrics indicates bursty workloads that may require capacity headroom.".into(),
        },
        CausalBelief {
            name: "trial_conversion_increases_load".into(),
            cause: "Customer.trial_conversion_rate".into(),
            effect: "Plan.capacity_pressure".into(),
            strength: 0.60,
            decision_classes: vec!["capacity_inventory".into()],
            description: "High trial conversion rates increase near-term capacity pressure on popular plans.".into(),
        },
    ]
}

fn build_decision_classes() -> Vec<DecisionClassDef> {
    vec![
        DecisionClassDef {
            name: "churn_intervention".into(),
            description: "Determine which customers are at risk of churning and what interventions to recommend.".into(),
            relevant_entities: vec![
                "Customer".into(), "Subscription".into(), "SupportTicket".into(),
                "Invoice".into(), "UsageMetric".into(), "Feature".into(),
            ],
            relevant_causal_beliefs: vec![
                "declining_usage_causes_churn".into(),
                "support_tickets_influence_satisfaction".into(),
                "failed_invoices_increase_churn".into(),
                "low_seat_utilization_predicts_downgrade".into(),
                "feature_breadth_reduces_churn".into(),
            ],
        },
        DecisionClassDef {
            name: "pricing".into(),
            description: "Determine optimal pricing actions: upgrades, downgrades, price changes, or custom offers.".into(),
            relevant_entities: vec![
                "Customer".into(), "Subscription".into(), "Plan".into(),
                "UsageMetric".into(), "Feature".into(),
            ],
            relevant_causal_beliefs: vec![
                "usage_growth_signals_upgrade".into(),
                "capacity_threshold_drives_expansion".into(),
                "price_sensitivity_from_plan_tier".into(),
                "low_seat_utilization_predicts_downgrade".into(),
            ],
        },
        DecisionClassDef {
            name: "capacity_inventory".into(),
            description: "Forecast and manage infrastructure capacity and resource allocation.".into(),
            relevant_entities: vec![
                "Customer".into(), "Plan".into(), "UsageMetric".into(),
            ],
            relevant_causal_beliefs: vec![
                "seat_growth_predicts_capacity_need".into(),
                "usage_spikes_signal_scaling".into(),
                "trial_conversion_increases_load".into(),
                "capacity_threshold_drives_expansion".into(),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archetype_builds_successfully() {
        let arch = build_saas_archetype();
        assert_eq!(arch.name, "saas");
        assert_eq!(arch.version, 1);
        assert_eq!(arch.entity_defs.len(), 7);
        assert_eq!(arch.decision_classes.len(), 3);
    }

    #[test]
    fn all_entity_defs_have_source_events() {
        let arch = build_saas_archetype();
        for entity in &arch.entity_defs {
            assert!(
                !entity.source_events.is_empty(),
                "Entity {} has no source events",
                entity.name
            );
        }
    }

    #[test]
    fn all_causal_beliefs_reference_valid_decision_classes() {
        let arch = build_saas_archetype();
        let valid_classes: Vec<&str> = arch
            .decision_classes
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        for belief in &arch.causal_beliefs {
            for dc in &belief.decision_classes {
                assert!(
                    valid_classes.contains(&dc.as_str()),
                    "Causal belief {} references unknown decision class {}",
                    belief.name,
                    dc
                );
            }
        }
    }

    #[test]
    fn causal_strength_within_range() {
        let arch = build_saas_archetype();
        for belief in &arch.causal_beliefs {
            assert!(
                belief.strength >= -1.0 && belief.strength <= 1.0,
                "Causal belief {} has out-of-range strength {}",
                belief.name,
                belief.strength
            );
        }
    }

    #[test]
    fn serde_roundtrip_archetype() {
        let arch = build_saas_archetype();
        let json = serde_json::to_string(&arch).unwrap();
        let back: SaasArchetype = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, arch.name);
        assert_eq!(back.entity_defs.len(), arch.entity_defs.len());
    }

    #[test]
    fn relationship_endpoints_reference_valid_entities() {
        let arch = build_saas_archetype();
        let entity_names: Vec<&str> = arch.entity_defs.iter().map(|e| e.name.as_str()).collect();
        for rel in &arch.relationship_defs {
            assert!(
                entity_names.contains(&rel.from_entity.as_str()),
                "Relationship {} references unknown from_entity {}",
                rel.name,
                rel.from_entity
            );
            assert!(
                entity_names.contains(&rel.to_entity.as_str()),
                "Relationship {} references unknown to_entity {}",
                rel.name,
                rel.to_entity
            );
        }
    }
}
