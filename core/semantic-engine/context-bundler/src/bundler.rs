//! Context bundler: assembles multi-source query results into confidence-weighted bundles.
//!
//! Takes raw QueryResults from all storage layers and produces a ContextBundle
//! with confidence-scored facts and an LLM-generated causal narrative.

use std::collections::HashMap;
use std::future::Future;

use chrono::Utc;

use anidb_shared_types::confidence::ConfidenceScore;
use anidb_shared_types::intent::{ContextBundle, Fact};

use crate::types::{CausalBeliefResult, QueryResults};

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("Narrative generation failed: {0}")]
    NarrativeError(String),

    #[error("Data assembly error: {0}")]
    AssemblyError(String),
}

// ============================================================================
// Bundler
// ============================================================================

/// Assemble a ContextBundle from raw query results.
///
/// The `narrative_fn` closure calls the LLM for causal narrative generation.
/// This keeps the bundler testable without an API key.
pub async fn assemble_bundle<F, Fut>(
    decision_class: &str,
    results: &QueryResults,
    narrative_fn: F,
) -> Result<ContextBundle, BundleError>
where
    F: FnOnce(String, String) -> Fut,
    Fut: Future<Output = Result<String, String>>,
{
    // Step 1: Convert stored FactRecords into output Facts
    let fact_store_facts = extract_stored_facts(results);

    // Step 2: Create facts from graph data (high confidence — current state)
    let graph_facts = extract_graph_facts(&results.graph_data);

    // Step 3: Create derived facts from event data
    let event_facts = extract_event_facts(&results.events);

    // Step 4: Merge all facts
    let mut all_facts = Vec::new();
    all_facts.extend(fact_store_facts);
    all_facts.extend(graph_facts);
    all_facts.extend(event_facts);

    // Step 5: Generate causal narrative via LLM
    let narrative_prompt =
        build_narrative_prompt(decision_class, &all_facts, &results.causal_beliefs);

    let causal_context = narrative_fn(crate::prompts::narrative_system_prompt(), narrative_prompt)
        .await
        .map_err(BundleError::NarrativeError)?;

    // Step 6: Generate suggested follow-up queries
    let suggested_queries = generate_suggested_queries(decision_class);

    Ok(ContextBundle {
        decision_class: decision_class.to_string(),
        facts: all_facts,
        causal_context,
        suggested_queries,
    })
}

/// Convert FactRecords from the confidence store into output Facts.
fn extract_stored_facts(results: &QueryResults) -> Vec<Fact> {
    results
        .facts
        .iter()
        .map(|fr| Fact {
            key: format!("{}.{}", fr.entity_type, fr.fact_key),
            value: fr.fact_value.clone(),
            confidence: fr.confidence.clone(),
        })
        .collect()
}

/// Convert knowledge graph data into Facts with high confidence.
///
/// Graph data represents the current materialized state, so it gets
/// confidence 0.95 (very high, but not 1.0 since derived from events).
fn extract_graph_facts(graph_data: &[serde_json::Value]) -> Vec<Fact> {
    let mut facts = Vec::new();

    for row in graph_data {
        if let Some(obj) = row.as_object() {
            for (key, value) in obj {
                if !value.is_null() {
                    facts.push(Fact {
                        key: format!("graph.{}", key),
                        value: value.clone(),
                        confidence: ConfidenceScore {
                            value: 0.95,
                            source: "knowledge_graph".to_string(),
                            last_verified: Utc::now(),
                            derivation: vec![],
                        },
                    });
                }
            }
        }
    }

    facts
}

/// Derive aggregate facts from raw events.
///
/// Computes: total event count, counts by event type.
/// These are exact counts (confidence 1.0) since they come from the event log.
fn extract_event_facts(events: &[anidb_shared_types::Event]) -> Vec<Fact> {
    let mut facts = Vec::new();

    if events.is_empty() {
        return facts;
    }

    // Total count
    facts.push(Fact {
        key: "events.total_count".to_string(),
        value: serde_json::json!(events.len()),
        confidence: ConfidenceScore {
            value: 1.0,
            source: "event_log".to_string(),
            last_verified: Utc::now(),
            derivation: vec![],
        },
    });

    // Count by event type
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for event in events {
        *type_counts.entry(event.event_type.clone()).or_insert(0) += 1;
    }
    for (event_type, count) in &type_counts {
        facts.push(Fact {
            key: format!("events.{}_count", to_snake_case(event_type)),
            value: serde_json::json!(count),
            confidence: ConfidenceScore {
                value: 1.0,
                source: "event_log_derived".to_string(),
                last_verified: Utc::now(),
                derivation: vec![],
            },
        });
    }

    facts
}

/// Build the user prompt for narrative generation.
fn build_narrative_prompt(
    decision_class: &str,
    facts: &[Fact],
    causal_beliefs: &[CausalBeliefResult],
) -> String {
    let mut prompt = format!(
        "Decision class: {}\n\nRelevant causal beliefs:\n",
        decision_class
    );

    if causal_beliefs.is_empty() {
        prompt.push_str("(No causal beliefs loaded for this decision class)\n");
    } else {
        for belief in causal_beliefs {
            prompt.push_str(&format!(
                "- {} (strength: {:.2}): {} -> {} — {}\n",
                belief.belief_name,
                belief.strength,
                belief.cause,
                belief.effect,
                belief.description
            ));
        }
    }

    prompt.push_str("\nObserved facts:\n");
    if facts.is_empty() {
        prompt.push_str("(No facts available — data may not be loaded yet)\n");
    } else {
        for fact in facts {
            prompt.push_str(&format!(
                "- {} = {} (confidence: {:.2}, source: {})\n",
                fact.key, fact.value, fact.confidence.value, fact.confidence.source
            ));
        }
    }

    prompt
        .push_str("\nSynthesize a causal narrative connecting these beliefs to the observed data.");
    prompt
}

/// Generate suggested follow-up queries based on the decision class.
fn generate_suggested_queries(decision_class: &str) -> Vec<String> {
    match decision_class {
        "churn_intervention" => vec![
            "What intervention reduced churn for similar customers?".to_string(),
            "Which features do retained customers use most?".to_string(),
            "What is the average time from first warning sign to churn?".to_string(),
        ],
        "pricing" => vec![
            "What is the price elasticity for this customer segment?".to_string(),
            "Which customers are most likely to accept an upgrade offer?".to_string(),
            "What revenue impact would a 10% price increase have?".to_string(),
        ],
        "capacity_inventory" => vec![
            "What is the projected capacity need for the next 30 days?".to_string(),
            "Which customers are approaching their resource limits?".to_string(),
            "What is the cost per additional unit of capacity?".to_string(),
        ],
        _ => vec![],
    }
}

/// Convert CamelCase to snake_case for fact keys.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use anidb_shared_types::events::EventMetadata;
    use uuid::Uuid;

    fn make_event(event_type: &str) -> anidb_shared_types::Event {
        anidb_shared_types::Event {
            id: Uuid::new_v4(),
            stream_id: "customer-123".to_string(),
            event_type: event_type.to_string(),
            payload: serde_json::json!({}),
            metadata: EventMetadata {
                timestamp: Utc::now(),
                actor: "test".to_string(),
                causation_id: None,
                correlation_id: Uuid::new_v4(),
                ontology_version: 1,
            },
        }
    }

    fn make_fact_record(entity_type: &str, fact_key: &str) -> anidb_shared_types::FactRecord {
        anidb_shared_types::FactRecord {
            id: Uuid::new_v4(),
            entity_id: Uuid::new_v4(),
            entity_type: entity_type.to_string(),
            fact_key: fact_key.to_string(),
            fact_value: serde_json::json!(42),
            confidence: ConfidenceScore {
                value: 0.8,
                source: "test".to_string(),
                last_verified: Utc::now(),
                derivation: vec![],
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn assemble_bundle_basic() {
        let results = QueryResults {
            graph_data: vec![serde_json::json!({"status": "active", "mrr_cents": 9900})],
            causal_beliefs: vec![CausalBeliefResult {
                belief_name: "test_belief".to_string(),
                cause: "usage_decline".to_string(),
                effect: "churn".to_string(),
                strength: 0.85,
                description: "Usage decline predicts churn".to_string(),
            }],
            events: vec![
                make_event("LoginEvent"),
                make_event("LoginEvent"),
                make_event("SupportTicketOpened"),
            ],
            facts: vec![make_fact_record("Customer", "churn_risk")],
        };

        let bundle = assemble_bundle("churn_intervention", &results, |_system, _user| async {
            Ok("Test narrative: customer shows declining usage.".to_string())
        })
        .await
        .unwrap();

        assert_eq!(bundle.decision_class, "churn_intervention");
        assert!(!bundle.facts.is_empty());
        assert!(bundle.causal_context.contains("declining usage"));
        assert!(!bundle.suggested_queries.is_empty());
    }

    #[tokio::test]
    async fn assemble_bundle_empty_results() {
        let results = QueryResults::default();

        let bundle = assemble_bundle("pricing", &results, |_system, _user| async {
            Ok("No data available for analysis.".to_string())
        })
        .await
        .unwrap();

        assert_eq!(bundle.decision_class, "pricing");
        assert!(bundle.facts.is_empty());
    }

    #[tokio::test]
    async fn narrative_fn_error_propagates() {
        let results = QueryResults::default();

        let result = assemble_bundle("churn_intervention", &results, |_system, _user| async {
            Err("LLM rate limited".to_string())
        })
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rate limited"));
    }

    #[test]
    fn extract_event_facts_counts() {
        let events = vec![
            make_event("LoginEvent"),
            make_event("LoginEvent"),
            make_event("SupportTicketOpened"),
        ];

        let facts = extract_event_facts(&events);

        // Should have total_count + 2 type counts
        assert!(facts.len() >= 3);
        let total = facts
            .iter()
            .find(|f| f.key == "events.total_count")
            .unwrap();
        assert_eq!(total.value, serde_json::json!(3));
        assert_eq!(total.confidence.value, 1.0);
    }

    #[test]
    fn extract_event_facts_empty() {
        let facts = extract_event_facts(&[]);
        assert!(facts.is_empty());
    }

    #[test]
    fn graph_facts_skip_null() {
        let data = vec![serde_json::json!({"name": "test", "value": null})];
        let facts = extract_graph_facts(&data);

        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].key, "graph.name");
        assert_eq!(facts[0].confidence.value, 0.95);
    }

    #[test]
    fn to_snake_case_conversion() {
        assert_eq!(to_snake_case("LoginEvent"), "login_event");
        assert_eq!(to_snake_case("SupportTicketOpened"), "support_ticket_opened");
        assert_eq!(to_snake_case("simple"), "simple");
    }

    #[test]
    fn suggested_queries_by_decision_class() {
        assert!(!generate_suggested_queries("churn_intervention").is_empty());
        assert!(!generate_suggested_queries("pricing").is_empty());
        assert!(!generate_suggested_queries("capacity_inventory").is_empty());
        assert!(generate_suggested_queries("unknown").is_empty());
    }
}
