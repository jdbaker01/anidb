//! Intent parser: translates natural language goal declarations into structured query plans.
//!
//! Uses the Anthropic API with structured output (tool_use) via an injected closure.
//! The closure-based design keeps this crate testable without an API key.

use std::future::Future;

use anidb_shared_types::IntentQuery;

use crate::types::{DecisionClass, ParsedIntent};

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("LLM call failed: {0}")]
    LlmError(String),

    #[error("Failed to parse LLM response: {0}")]
    ResponseParse(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

// ============================================================================
// Parser
// ============================================================================

/// Parse an agent's IntentQuery into a structured ParsedIntent.
///
/// The `llm_call` parameter is a closure that:
/// - Takes (system_prompt, user_message, tool_schema) as String, String, Value
/// - Sends them to the LLM with tool_use
/// - Returns the tool input as a serde_json::Value
///
/// This design allows the semantic-engine binary to inject its AnthropicClient
/// while keeping this crate testable with mock closures.
pub async fn parse_intent<F, Fut>(
    query: &IntentQuery,
    llm_call: F,
) -> Result<ParsedIntent, ParseError>
where
    F: FnOnce(String, String, serde_json::Value) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, String>>,
{
    let system = crate::prompts::system_prompt();
    let user_message = format_user_message(query);
    let tool_schema = crate::prompts::parsed_intent_tool_schema();

    let result = llm_call(system, user_message, tool_schema)
        .await
        .map_err(ParseError::LlmError)?;

    let mut parsed: ParsedIntent = serde_json::from_value(result)
        .map_err(|e| ParseError::ResponseParse(e.to_string()))?;

    // Apply overrides from the original query context
    apply_context_overrides(&mut parsed, query);

    // Validate the parsed intent
    validate_parsed_intent(&parsed)?;

    Ok(parsed)
}

/// Format the user message from an IntentQuery for the LLM.
fn format_user_message(query: &IntentQuery) -> String {
    let mut msg = format!("Agent intent: {}\n", query.intent);

    if let Some(dc) = &query.context.decision_class {
        msg.push_str(&format!("Suggested decision class: {}\n", dc));
    }
    if !query.context.entity_refs.is_empty() {
        msg.push_str(&format!(
            "Entity references: {}\n",
            query.context.entity_refs.join(", ")
        ));
    }
    if let Some(th) = &query.context.time_horizon {
        msg.push_str(&format!("Time horizon: {}\n", th));
    }
    if let Some(mc) = query.context.min_confidence {
        msg.push_str(&format!("Minimum confidence: {}\n", mc));
    }

    msg
}

/// Apply overrides from the original IntentContext.
///
/// If the agent explicitly specified a decision class or min_confidence,
/// those take precedence over the LLM's interpretation.
fn apply_context_overrides(parsed: &mut ParsedIntent, query: &IntentQuery) {
    // Override decision class if explicitly provided
    if let Some(dc_str) = &query.context.decision_class {
        if let Some(dc) = match dc_str.as_str() {
            "churn_intervention" => Some(DecisionClass::ChurnIntervention),
            "pricing" => Some(DecisionClass::Pricing),
            "capacity_inventory" => Some(DecisionClass::CapacityInventory),
            _ => None,
        } {
            parsed.decision_class = dc;
        }
    }

    // Override min_confidence if explicitly provided
    if let Some(mc) = query.context.min_confidence {
        parsed.min_confidence = mc;
    }
}

/// Validate the parsed intent for consistency.
fn validate_parsed_intent(parsed: &ParsedIntent) -> Result<(), ParseError> {
    if parsed.min_confidence < 0.0 || parsed.min_confidence > 1.0 {
        return Err(ParseError::Validation(format!(
            "min_confidence must be between 0.0 and 1.0, got {}",
            parsed.min_confidence
        )));
    }

    if parsed.interpretation.is_empty() {
        return Err(ParseError::Validation(
            "interpretation must not be empty".to_string(),
        ));
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use anidb_shared_types::intent::IntentContext;

    fn mock_intent_query(intent: &str, decision_class: Option<&str>) -> IntentQuery {
        IntentQuery {
            intent: intent.to_string(),
            context: IntentContext {
                decision_class: decision_class.map(|s| s.to_string()),
                entity_refs: vec![],
                time_horizon: None,
                min_confidence: None,
            },
        }
    }

    fn mock_parsed_intent_json(decision_class: &str) -> serde_json::Value {
        serde_json::json!({
            "decision_class": decision_class,
            "entity_refs": [],
            "time_horizon": {"lookback_days": 30, "forecast_days": 30},
            "min_confidence": 0.5,
            "required_data": [
                {"source": "knowledge_graph", "description": "Customer data"}
            ],
            "interpretation": "Test interpretation"
        })
    }

    #[tokio::test]
    async fn parse_churn_intent() {
        let query = mock_intent_query("Which customers are at risk of churning?", None);

        let result = parse_intent(&query, |_system, _user, _schema| async {
            Ok(mock_parsed_intent_json("churn_intervention"))
        })
        .await
        .unwrap();

        assert_eq!(result.decision_class, DecisionClass::ChurnIntervention);
        assert_eq!(result.min_confidence, 0.5);
        assert_eq!(result.time_horizon.lookback_days, 30);
    }

    #[tokio::test]
    async fn parse_pricing_intent() {
        let query = mock_intent_query("Which customers should we upsell?", None);

        let result = parse_intent(&query, |_system, _user, _schema| async {
            Ok(mock_parsed_intent_json("pricing"))
        })
        .await
        .unwrap();

        assert_eq!(result.decision_class, DecisionClass::Pricing);
    }

    #[tokio::test]
    async fn parse_capacity_intent() {
        let query = mock_intent_query("Do we need more infrastructure capacity?", None);

        let result = parse_intent(&query, |_system, _user, _schema| async {
            Ok(mock_parsed_intent_json("capacity_inventory"))
        })
        .await
        .unwrap();

        assert_eq!(result.decision_class, DecisionClass::CapacityInventory);
    }

    #[tokio::test]
    async fn context_override_decision_class() {
        // Agent explicitly says "churn_intervention" but LLM says "pricing"
        let query = mock_intent_query("analyze customer 123", Some("churn_intervention"));

        let result = parse_intent(&query, |_system, _user, _schema| async {
            Ok(mock_parsed_intent_json("pricing")) // LLM returns pricing
        })
        .await
        .unwrap();

        // Context override wins
        assert_eq!(result.decision_class, DecisionClass::ChurnIntervention);
    }

    #[tokio::test]
    async fn context_override_min_confidence() {
        let mut query = mock_intent_query("find churning customers", None);
        query.context.min_confidence = Some(0.8);

        let result = parse_intent(&query, |_system, _user, _schema| async {
            Ok(mock_parsed_intent_json("churn_intervention"))
        })
        .await
        .unwrap();

        assert_eq!(result.min_confidence, 0.8);
    }

    #[tokio::test]
    async fn llm_error_propagates() {
        let query = mock_intent_query("test", None);

        let result = parse_intent(&query, |_system, _user, _schema| async {
            Err("API rate limited".to_string())
        })
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API rate limited"));
    }

    #[tokio::test]
    async fn malformed_llm_response_errors() {
        let query = mock_intent_query("test", None);

        let result = parse_intent(&query, |_system, _user, _schema| async {
            Ok(serde_json::json!({"invalid": "response"}))
        })
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn user_message_includes_context() {
        let mut query = mock_intent_query("find churning customers", Some("churn_intervention"));
        query.context.entity_refs = vec!["customer-123".to_string()];
        query.context.time_horizon = Some("30_days".to_string());
        query.context.min_confidence = Some(0.7);

        // We test the formatting indirectly by verifying the LLM closure receives the data
        let result = parse_intent(&query, |_system, user_msg, _schema| async move {
            assert!(user_msg.contains("find churning customers"));
            assert!(user_msg.contains("churn_intervention"));
            assert!(user_msg.contains("customer-123"));
            assert!(user_msg.contains("30_days"));
            assert!(user_msg.contains("0.7"));
            Ok(mock_parsed_intent_json("churn_intervention"))
        })
        .await
        .unwrap();

        assert_eq!(result.decision_class, DecisionClass::ChurnIntervention);
    }
}
