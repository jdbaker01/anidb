//! Core orchestration pipeline for the Semantic Engine.
//!
//! This module ties the four subcrates together:
//! 1. intent-parser (LLM call #1) → ParsedIntent
//! 2. query-planner (pure logic)  → QueryPlan
//! 3. execute plan (I/O)          → QueryResults
//! 4. context-bundler (LLM call #2) → ContextBundle

use anidb_context_bundler::{assemble_bundle, CausalBeliefResult, QueryResults};
use anidb_intent_parser::parse_intent;
use anidb_knowledge_graph::queries as graph_queries;
use anidb_query_planner::{build_query_plan, ConfidenceQueryType, EventLogQueryType, GraphQueryType};
use anidb_shared_types::intent::{ContextBundle, IntentQuery};

use crate::anthropic::{Message, MessageContent, MessageRequest, ToolChoice, ToolDef};
use crate::error::SemanticEngineError;
use crate::state::AppState;

/// Execute the full intent-read pipeline.
///
/// Flow: IntentQuery → parse → plan → execute → bundle → ContextBundle
pub async fn process_intent_read(
    state: &AppState,
    query: IntentQuery,
) -> Result<ContextBundle, SemanticEngineError> {
    // ---- Phase 1: Parse intent via LLM ----
    tracing::info!(intent = %query.intent, "Phase 1: Parsing intent");

    let parsed = parse_intent(&query, |system, user_msg, tool_schema| {
        let client = state.anthropic.clone();
        async move {
            let request = MessageRequest {
                model: client.model().to_string(),
                max_tokens: 1024,
                system: Some(system),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: MessageContent::Text(user_msg),
                }],
                tools: Some(vec![ToolDef {
                    name: "parse_intent".to_string(),
                    description: "Parse the agent's intent into structured query parameters"
                        .to_string(),
                    input_schema: tool_schema,
                }]),
                tool_choice: Some(ToolChoice {
                    choice_type: "tool".to_string(),
                    name: "parse_intent".to_string(),
                }),
            };
            client
                .send_structured::<serde_json::Value>(request)
                .await
                .map_err(|e| e.to_string())
        }
    })
    .await?;

    tracing::info!(
        decision_class = parsed.decision_class.as_str(),
        entity_refs = parsed.entity_refs.len(),
        interpretation = %parsed.interpretation,
        "Phase 1 complete: Intent parsed"
    );

    // ---- Phase 2: Build query plan (pure logic) ----
    let plan = build_query_plan(&parsed)?;

    tracing::info!(
        graph_queries = plan.graph_queries.len(),
        event_queries = plan.event_log_queries.len(),
        confidence_queries = plan.confidence_queries.len(),
        "Phase 2 complete: Query plan built"
    );

    // ---- Phase 3: Execute query plan across storage layers ----
    let results = execute_query_plan(state, &plan).await?;

    tracing::info!(
        graph_data = results.graph_data.len(),
        causal_beliefs = results.causal_beliefs.len(),
        events = results.events.len(),
        facts = results.facts.len(),
        "Phase 3 complete: Query plan executed"
    );

    // ---- Phase 4: Assemble context bundle with LLM narrative ----
    let bundle = assemble_bundle(
        &plan.decision_class,
        &results,
        |system, user_msg| {
            let client = state.anthropic.clone();
            async move {
                let request = MessageRequest {
                    model: client.model().to_string(),
                    max_tokens: 2048,
                    system: Some(system),
                    messages: vec![Message {
                        role: "user".to_string(),
                        content: MessageContent::Text(user_msg),
                    }],
                    tools: None,
                    tool_choice: None,
                };
                client.send_text(request).await.map_err(|e| e.to_string())
            }
        },
    )
    .await?;

    tracing::info!(
        decision_class = %bundle.decision_class,
        facts_count = bundle.facts.len(),
        narrative_len = bundle.causal_context.len(),
        "Phase 4 complete: Context bundle assembled"
    );

    Ok(bundle)
}

/// Execute a QueryPlan by dispatching queries to all storage backends.
async fn execute_query_plan(
    state: &AppState,
    plan: &anidb_query_planner::QueryPlan,
) -> Result<QueryResults, SemanticEngineError> {
    let mut results = QueryResults::default();

    // Execute graph queries
    for gq in &plan.graph_queries {
        match &gq.query_type {
            GraphQueryType::CustomerContext { customer_id } => {
                tracing::debug!(customer_id = %customer_id, "Querying customer context from graph");
                match state
                    .graph
                    .execute_collect(graph_queries::get_customer_context(customer_id))
                    .await
                {
                    Ok(rows) => {
                        for row in rows {
                            results.graph_data.push(extract_customer_context_json(&row));
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, customer_id = %customer_id, "Failed to query customer context");
                    }
                }
            }
            GraphQueryType::CausalChain { decision_class } => {
                tracing::debug!(decision_class = %decision_class, "Querying causal chain from graph");
                match state
                    .graph
                    .execute_collect(graph_queries::get_causal_chain(decision_class))
                    .await
                {
                    Ok(rows) => {
                        for row in rows {
                            if let Some(belief) = extract_causal_belief(&row) {
                                results.causal_beliefs.push(belief);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, decision_class = %decision_class, "Failed to query causal chain");
                    }
                }
            }
            GraphQueryType::CustomersByStatus { status } => {
                tracing::debug!(status = %status, "Querying customers by status from graph");
                match state
                    .graph
                    .execute_collect(graph_queries::find_customers_by_status(status))
                    .await
                {
                    Ok(rows) => {
                        for row in rows {
                            results.graph_data.push(extract_customer_node_json(&row));
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, status = %status, "Failed to query customers by status");
                    }
                }
            }
            GraphQueryType::CausalBeliefs { decision_class } => {
                tracing::debug!(decision_class = %decision_class, "Querying causal beliefs from graph");
                match state
                    .graph
                    .execute_collect(graph_queries::list_causal_beliefs(Some(decision_class)))
                    .await
                {
                    Ok(rows) => {
                        for row in rows {
                            if let Some(belief) = extract_causal_belief(&row) {
                                results.causal_beliefs.push(belief);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to query causal beliefs");
                    }
                }
            }
        }
    }

    // Execute event log queries
    for eq in &plan.event_log_queries {
        match &eq.query_type {
            EventLogQueryType::ReadStream { stream_name } => {
                tracing::debug!(stream = %stream_name, "Reading event stream");
                match state.event_log.read_stream(stream_name).await {
                    Ok(events) => results.events.extend(events),
                    Err(e) => {
                        tracing::warn!(error = %e, stream = %stream_name, "Failed to read event stream");
                    }
                }
            }
            EventLogQueryType::ReadCategory { category } => {
                tracing::debug!(category = %category, "Reading event category");
                match state.event_log.read_category(category).await {
                    Ok(events) => results.events.extend(events),
                    Err(e) => {
                        tracing::warn!(error = %e, category = %category, "Failed to read event category");
                    }
                }
            }
        }
    }

    // Execute confidence queries
    for cq in &plan.confidence_queries {
        match &cq.query_type {
            ConfidenceQueryType::EntityFacts { entity_id } => {
                tracing::debug!(entity_id = %entity_id, "Querying entity facts");
                match state.confidence_store.get_entity_facts(*entity_id).await {
                    Ok(facts) => results.facts.extend(facts),
                    Err(e) => {
                        tracing::warn!(error = %e, entity_id = %entity_id, "Failed to get entity facts");
                    }
                }
            }
            ConfidenceQueryType::BulkFacts { entity_ids } => {
                tracing::debug!(count = entity_ids.len(), "Querying bulk facts");
                match state.confidence_store.get_bulk_facts(entity_ids).await {
                    Ok(facts) => results.facts.extend(facts),
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to get bulk facts");
                    }
                }
            }
            ConfidenceQueryType::TypeFacts { entity_type } => {
                tracing::debug!(entity_type = %entity_type, "Querying facts by type");
                match state.confidence_store.get_facts_by_type(entity_type).await {
                    Ok(facts) => results.facts.extend(facts),
                    Err(e) => {
                        tracing::warn!(error = %e, entity_type = %entity_type, "Failed to get type facts");
                    }
                }
            }
        }
    }

    Ok(results)
}

// ============================================================================
// Neo4j row extraction helpers
// ============================================================================

/// Extract customer context from a get_customer_context() query result.
///
/// The query returns: c (Customer node), p (Plan node), features, tickets, invoices
/// as collected lists. We extract what we can into a flat JSON object.
fn extract_customer_context_json(row: &neo4rs::Row) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    // Try to extract the customer node as a neo4rs::Node
    if let Ok(node) = row.get::<neo4rs::Node>("c") {
        extract_node_properties(&node, &mut map, "customer");
    }

    // Try to extract the plan node
    if let Ok(node) = row.get::<neo4rs::Node>("p") {
        extract_node_properties(&node, &mut map, "plan");
    }

    // Features, tickets, invoices come as collected lists — extract as JSON
    for list_key in &["features", "tickets", "invoices"] {
        // These are collected as list of maps in Cypher
        // neo4rs may return them as BoltList; try to get as a generic value
        if let Ok(val) = row.get::<Vec<serde_json::Value>>(list_key) {
            map.insert(list_key.to_string(), serde_json::json!(val));
        }
    }

    serde_json::Value::Object(map)
}

/// Extract a single customer node from find_customers_by_status() result.
fn extract_customer_node_json(row: &neo4rs::Row) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    if let Ok(node) = row.get::<neo4rs::Node>("c") {
        extract_node_properties(&node, &mut map, "");
    }

    serde_json::Value::Object(map)
}

/// Extract known properties from a Neo4j Node into a JSON map.
fn extract_node_properties(
    node: &neo4rs::Node,
    map: &mut serde_json::Map<String, serde_json::Value>,
    prefix: &str,
) {
    let prefix_str = if prefix.is_empty() {
        String::new()
    } else {
        format!("{}_", prefix)
    };

    // String properties
    for key in &[
        "customer_id",
        "status",
        "plan_id",
        "feature_name",
        "priority",
        "category",
    ] {
        if let Ok(v) = node.get::<String>(key) {
            map.insert(format!("{}{}", prefix_str, key), serde_json::json!(v));
        }
    }

    // Integer properties
    for key in &["mrr_cents", "seat_count", "price_cents", "amount_cents", "login_count"] {
        if let Ok(v) = node.get::<i64>(key) {
            map.insert(format!("{}{}", prefix_str, key), serde_json::json!(v));
        }
    }
}

/// Extract a causal belief from a get_causal_chain() query result.
fn extract_causal_belief(row: &neo4rs::Row) -> Option<CausalBeliefResult> {
    let belief_name = row.get::<String>("belief_name").ok()?;
    let cause = row.get::<String>("cause").ok()?;
    let effect = row.get::<String>("effect").ok()?;
    let strength = row.get::<f64>("strength").ok()?;
    let description = row.get::<String>("description").ok()?;

    Some(CausalBeliefResult {
        belief_name,
        cause,
        effect,
        strength,
        description,
    })
}
