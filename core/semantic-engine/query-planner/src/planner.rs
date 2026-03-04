//! Query planner: translates parsed intent into storage operations.
//!
//! This is pure deterministic logic — no I/O, no LLM calls. It takes a ParsedIntent
//! and produces a QueryPlan that specifies exactly what data to fetch from each
//! storage layer (knowledge graph, event log, confidence store).

use anidb_intent_parser::types::{DataSource, DecisionClass, ParsedIntent};

use crate::types::*;

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("Invalid entity reference format: {0}")]
    InvalidEntityRef(String),
}

// ============================================================================
// Planner
// ============================================================================

/// Build a query plan from a parsed intent.
///
/// This is the main entry point. It generates queries across all three storage
/// layers based on the decision class and entity references.
pub fn build_query_plan(intent: &ParsedIntent) -> Result<QueryPlan, PlanError> {
    let mut graph_queries = Vec::new();
    let mut event_log_queries = Vec::new();
    let mut confidence_queries = Vec::new();

    let dc_str = intent.decision_class.as_str().to_string();

    // ---- Always: fetch the causal chain for context ----
    graph_queries.push(GraphQueryStep {
        description: format!("Fetch causal beliefs for {}", dc_str),
        query_type: GraphQueryType::CausalChain {
            decision_class: dc_str.clone(),
        },
    });

    // ---- Per entity ref: build entity-specific queries ----
    for entity_ref in &intent.entity_refs {
        match entity_ref.entity_type.as_str() {
            "Customer" => {
                let cid = &entity_ref.identifier;

                // Graph: full customer context
                graph_queries.push(GraphQueryStep {
                    description: format!("Get customer context for {}", cid),
                    query_type: GraphQueryType::CustomerContext {
                        customer_id: cid.clone(),
                    },
                });

                // Event log: customer stream
                event_log_queries.push(EventLogQueryStep {
                    description: format!("Read event history for customer-{}", cid),
                    query_type: EventLogQueryType::ReadStream {
                        stream_name: format!("customer-{}", cid),
                    },
                });

                // Confidence store: entity facts (if UUID)
                if let Ok(uuid) = cid.parse::<uuid::Uuid>() {
                    confidence_queries.push(ConfidenceQueryStep {
                        description: format!("Get confidence-weighted facts for {}", cid),
                        query_type: ConfidenceQueryType::EntityFacts { entity_id: uuid },
                    });
                }
            }
            "Plan" => {
                // Event log: plan stream
                event_log_queries.push(EventLogQueryStep {
                    description: format!(
                        "Read event history for plan-{}",
                        entity_ref.identifier
                    ),
                    query_type: EventLogQueryType::ReadStream {
                        stream_name: format!("plan-{}", entity_ref.identifier),
                    },
                });
            }
            _ => {
                // Other entity types: add to required_data-driven queries below
                tracing::debug!(
                    entity_type = %entity_ref.entity_type,
                    identifier = %entity_ref.identifier,
                    "Unknown entity type in query planner, skipping entity-specific queries"
                );
            }
        }
    }

    // ---- Decision-class-specific queries ----
    add_decision_class_queries(
        intent,
        &dc_str,
        &mut graph_queries,
        &mut event_log_queries,
        &mut confidence_queries,
    );

    // ---- Required data points from LLM suggestion ----
    add_required_data_queries(intent, &mut event_log_queries, &mut confidence_queries);

    Ok(QueryPlan {
        decision_class: dc_str,
        graph_queries,
        event_log_queries,
        confidence_queries,
    })
}

/// Add decision-class-specific queries based on the intent type.
fn add_decision_class_queries(
    intent: &ParsedIntent,
    dc_str: &str,
    graph_queries: &mut Vec<GraphQueryStep>,
    event_log_queries: &mut Vec<EventLogQueryStep>,
    confidence_queries: &mut Vec<ConfidenceQueryStep>,
) {
    let has_customer_refs = intent
        .entity_refs
        .iter()
        .any(|r| r.entity_type == "Customer");

    match intent.decision_class {
        DecisionClass::ChurnIntervention => {
            // If no specific customers, find all active ones
            if !has_customer_refs {
                graph_queries.push(GraphQueryStep {
                    description: "Find active customers for churn analysis".to_string(),
                    query_type: GraphQueryType::CustomersByStatus {
                        status: "active".to_string(),
                    },
                });
            }
            // Always get customer-type facts for churn analysis
            confidence_queries.push(ConfidenceQueryStep {
                description: "Get all customer facts for churn analysis".to_string(),
                query_type: ConfidenceQueryType::TypeFacts {
                    entity_type: "Customer".to_string(),
                },
            });
        }
        DecisionClass::Pricing => {
            // Customer facts for pricing analysis
            confidence_queries.push(ConfidenceQueryStep {
                description: format!("Get customer facts for {} analysis", dc_str),
                query_type: ConfidenceQueryType::TypeFacts {
                    entity_type: "Customer".to_string(),
                },
            });
            // If no specific customers, find active ones
            if !has_customer_refs {
                graph_queries.push(GraphQueryStep {
                    description: "Find active customers for pricing analysis".to_string(),
                    query_type: GraphQueryType::CustomersByStatus {
                        status: "active".to_string(),
                    },
                });
            }
        }
        DecisionClass::CapacityInventory => {
            // Capacity threshold events
            event_log_queries.push(EventLogQueryStep {
                description: "Read capacity threshold events".to_string(),
                query_type: EventLogQueryType::ReadCategory {
                    category: "customer".to_string(),
                },
            });
            // Usage metric facts
            confidence_queries.push(ConfidenceQueryStep {
                description: "Get usage metric facts for capacity planning".to_string(),
                query_type: ConfidenceQueryType::TypeFacts {
                    entity_type: "Customer".to_string(),
                },
            });
        }
    }
}

/// Add queries derived from the LLM's required_data suggestions.
fn add_required_data_queries(
    intent: &ParsedIntent,
    event_log_queries: &mut Vec<EventLogQueryStep>,
    confidence_queries: &mut Vec<ConfidenceQueryStep>,
) {
    for data_point in &intent.required_data {
        match data_point.source {
            DataSource::KnowledgeGraph => {
                // Graph queries are already handled by entity refs and decision class
            }
            DataSource::EventLog => {
                // Add a category read if the description mentions specific event types
                // For the PoC, we rely on the decision-class queries above
            }
            DataSource::ConfidenceStore => {
                // Additional confidence queries are already handled above
            }
        }
    }
    // Suppress unused variable warnings
    let _ = (event_log_queries, confidence_queries);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use anidb_intent_parser::types::{EntityRef, RequiredDataPoint, TimeHorizon};

    fn make_parsed_intent(
        decision_class: DecisionClass,
        entity_refs: Vec<EntityRef>,
    ) -> ParsedIntent {
        ParsedIntent {
            decision_class,
            entity_refs,
            time_horizon: TimeHorizon {
                lookback_days: 30,
                forecast_days: 30,
            },
            min_confidence: 0.5,
            required_data: vec![],
            interpretation: "Test intent".to_string(),
        }
    }

    #[test]
    fn churn_no_entity_refs() {
        let intent = make_parsed_intent(DecisionClass::ChurnIntervention, vec![]);
        let plan = build_query_plan(&intent).unwrap();

        assert_eq!(plan.decision_class, "churn_intervention");
        // Should have: causal chain + find active customers
        assert!(plan.graph_queries.len() >= 2);
        // Should have customer type facts
        assert!(!plan.confidence_queries.is_empty());
        // Verify causal chain is first
        match &plan.graph_queries[0].query_type {
            GraphQueryType::CausalChain { decision_class } => {
                assert_eq!(decision_class, "churn_intervention");
            }
            _ => panic!("First query should be CausalChain"),
        }
        // Verify active customer lookup
        let has_status_query = plan.graph_queries.iter().any(|q| {
            matches!(
                &q.query_type,
                GraphQueryType::CustomersByStatus { status } if status == "active"
            )
        });
        assert!(has_status_query);
    }

    #[test]
    fn churn_with_customer_ref() {
        let intent = make_parsed_intent(
            DecisionClass::ChurnIntervention,
            vec![EntityRef {
                entity_type: "Customer".to_string(),
                identifier: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            }],
        );
        let plan = build_query_plan(&intent).unwrap();

        // Should have: causal chain + customer context (NO customers_by_status since we have refs)
        assert!(plan.graph_queries.len() >= 2);
        let has_customer_context = plan.graph_queries.iter().any(|q| {
            matches!(&q.query_type, GraphQueryType::CustomerContext { .. })
        });
        assert!(has_customer_context);

        // Should have event stream query
        let has_stream = plan.event_log_queries.iter().any(|q| {
            matches!(&q.query_type, EventLogQueryType::ReadStream { .. })
        });
        assert!(has_stream);

        // Should have entity facts (UUID parsed successfully)
        let has_entity_facts = plan.confidence_queries.iter().any(|q| {
            matches!(&q.query_type, ConfidenceQueryType::EntityFacts { .. })
        });
        assert!(has_entity_facts);

        // Should NOT have CustomersByStatus since we have specific refs
        let has_status_query = plan.graph_queries.iter().any(|q| {
            matches!(&q.query_type, GraphQueryType::CustomersByStatus { .. })
        });
        assert!(!has_status_query);
    }

    #[test]
    fn pricing_no_refs() {
        let intent = make_parsed_intent(DecisionClass::Pricing, vec![]);
        let plan = build_query_plan(&intent).unwrap();

        assert_eq!(plan.decision_class, "pricing");
        // Should have causal chain + active customers
        assert!(plan.graph_queries.len() >= 2);
        // Should have customer type facts
        let has_type_facts = plan.confidence_queries.iter().any(|q| {
            matches!(
                &q.query_type,
                ConfidenceQueryType::TypeFacts { entity_type } if entity_type == "Customer"
            )
        });
        assert!(has_type_facts);
    }

    #[test]
    fn capacity_inventory() {
        let intent = make_parsed_intent(DecisionClass::CapacityInventory, vec![]);
        let plan = build_query_plan(&intent).unwrap();

        assert_eq!(plan.decision_class, "capacity_inventory");
        // Should have event log category query for capacity events
        assert!(!plan.event_log_queries.is_empty());
        // Should have customer facts for capacity planning
        assert!(!plan.confidence_queries.is_empty());
    }

    #[test]
    fn plan_entity_ref() {
        let intent = make_parsed_intent(
            DecisionClass::Pricing,
            vec![EntityRef {
                entity_type: "Plan".to_string(),
                identifier: "enterprise".to_string(),
            }],
        );
        let plan = build_query_plan(&intent).unwrap();

        // Should have event log stream for plan
        let has_plan_stream = plan.event_log_queries.iter().any(|q| match &q.query_type {
            EventLogQueryType::ReadStream { stream_name } => {
                stream_name == "plan-enterprise"
            }
            _ => false,
        });
        assert!(has_plan_stream);
    }

    #[test]
    fn plan_serde_roundtrip() {
        let intent = make_parsed_intent(DecisionClass::ChurnIntervention, vec![]);
        let plan = build_query_plan(&intent).unwrap();
        let json = serde_json::to_string(&plan).unwrap();
        let parsed: QueryPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.decision_class, "churn_intervention");
        assert_eq!(parsed.graph_queries.len(), plan.graph_queries.len());
    }

    #[test]
    fn non_uuid_customer_ref_skips_confidence() {
        let intent = make_parsed_intent(
            DecisionClass::ChurnIntervention,
            vec![EntityRef {
                entity_type: "Customer".to_string(),
                identifier: "not-a-uuid".to_string(),
            }],
        );
        let plan = build_query_plan(&intent).unwrap();

        // Should NOT have EntityFacts (can't parse "not-a-uuid" as UUID)
        let has_entity_facts = plan.confidence_queries.iter().any(|q| {
            matches!(&q.query_type, ConfidenceQueryType::EntityFacts { .. })
        });
        assert!(!has_entity_facts);

        // Should still have customer context graph query
        let has_context = plan.graph_queries.iter().any(|q| {
            matches!(&q.query_type, GraphQueryType::CustomerContext { .. })
        });
        assert!(has_context);
    }
}
