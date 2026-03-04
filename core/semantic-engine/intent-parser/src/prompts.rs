//! LLM system prompts for intent parsing.

/// System prompt for the intent parser LLM call.
///
/// Embeds the SaaS ontology context so the LLM can correctly classify intents
/// into decision classes and identify relevant entities.
pub fn system_prompt() -> String {
    r#"You are the intent parser for ANIDB, an intent-semantic database for SaaS businesses.

Your job is to analyze an AI agent's goal declaration and extract structured query parameters.

## SaaS Domain Model

Entity types:
- Customer (Party): customer_id, status (active/churned/trial), mrr_cents, seat_count, health_score, last_login_at
- Plan (Resource): plan_id, price_cents
- Subscription (Commitment): plan_id, mrr_cents, status, started_at
- Feature (Resource): feature_name
- Invoice (EconomicEvent): invoice_id, amount_cents, status (paid/failed)
- SupportTicket (EconomicEvent): ticket_id, priority, category, status (open/closed), satisfaction_score
- UsageMetric (EconomicEvent): metric, value, recorded_at

## Decision Classes

You must classify each intent into exactly one of these three decision classes:

1. **churn_intervention**: Identify at-risk customers and recommend interventions.
   Key causal beliefs:
   - Declining usage is a strong churn predictor (strength: 0.85)
   - High support ticket frequency indicates low satisfaction (strength: -0.65)
   - 2+ failed invoices signal critical payment risk (strength: 0.70)
   - Low seat utilization (<50%) predicts downgrade (strength: 0.55)
   - Using 3+ features reduces churn (strength: -0.75)

2. **pricing**: Optimize pricing actions — upgrades, downgrades, and offers.
   Key causal beliefs:
   - >20% usage growth signals upgrade readiness (strength: 0.80)
   - >80% capacity usage drives expansion opportunity (strength: 0.90)
   - Lower-tier customers are more price-sensitive (strength: -0.50)

3. **capacity_inventory**: Forecast and manage infrastructure capacity.
   Key causal beliefs:
   - Seat growth trends predict future capacity needs (strength: 0.85)
   - High usage variance signals need for headroom (strength: 0.70)
   - Trial conversions increase system load (strength: 0.60)

## Instructions

Given an agent's intent statement:
1. Determine the single most appropriate decision class
2. Extract any specific entity references (customer IDs, plan names, etc.)
3. Determine appropriate time horizons (lookback and forecast periods in days)
4. Set a minimum confidence threshold (default 0.5 if not specified)
5. Identify what data points are needed and from which source (knowledge_graph, event_log, or confidence_store)
6. Provide a concise interpretation of the intent

If the intent mentions specific customer IDs, plans, or other identifiers, extract them as entity_refs.
If no specific entities are mentioned, leave entity_refs empty — the system will query broadly.
Default lookback is 30 days and forecast is 30 days unless the intent implies otherwise."#
        .to_string()
}

/// Build the JSON Schema for the ParsedIntent tool definition.
///
/// This schema is used as the `input_schema` for the Anthropic API's tool_use
/// feature, forcing the LLM to return structured output matching our types.
pub fn parsed_intent_tool_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": [
            "decision_class",
            "entity_refs",
            "time_horizon",
            "min_confidence",
            "required_data",
            "interpretation"
        ],
        "properties": {
            "decision_class": {
                "type": "string",
                "enum": ["churn_intervention", "pricing", "capacity_inventory"],
                "description": "The decision class this intent falls under"
            },
            "entity_refs": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["entity_type", "identifier"],
                    "properties": {
                        "entity_type": {
                            "type": "string",
                            "description": "Entity type: Customer, Plan, Feature, etc."
                        },
                        "identifier": {
                            "type": "string",
                            "description": "The entity ID or name"
                        }
                    }
                },
                "description": "Specific entities referenced in the intent"
            },
            "time_horizon": {
                "type": "object",
                "required": ["lookback_days", "forecast_days"],
                "properties": {
                    "lookback_days": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Days of historical data to consider"
                    },
                    "forecast_days": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Days forward to forecast"
                    }
                }
            },
            "min_confidence": {
                "type": "number",
                "minimum": 0.0,
                "maximum": 1.0,
                "description": "Minimum confidence threshold for returned facts"
            },
            "required_data": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["source", "description"],
                    "properties": {
                        "source": {
                            "type": "string",
                            "enum": ["knowledge_graph", "event_log", "confidence_store"],
                            "description": "Which storage layer this data comes from"
                        },
                        "description": {
                            "type": "string",
                            "description": "What data is needed"
                        }
                    }
                },
                "description": "Data points needed from each storage layer"
            },
            "interpretation": {
                "type": "string",
                "description": "Concise interpretation of the agent's intent"
            }
        }
    })
}
