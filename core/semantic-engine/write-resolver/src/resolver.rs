//! Write resolver: resolves semantic write declarations into typed events.
//!
//! For the PoC, this does straightforward property-to-payload mapping via
//! serde deserialization. No LLM involvement — just validation against the
//! SaaS event schema.

use anidb_shared_types::saas_events::*;

use crate::types::*;

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("Unknown entity type: {0}")]
    UnknownEntity(String),

    #[error("Missing required property: {0}")]
    MissingProperty(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),
}

// ============================================================================
// Resolver
// ============================================================================

/// Resolve a write declaration into a typed SaaS event.
///
/// Maps the entity_type to the appropriate event variant and deserializes
/// the properties into the typed payload struct.
pub fn resolve_write(decl: &WriteDeclaration) -> Result<ResolvedWrite, ResolveError> {
    match decl.entity_type.as_str() {
        "Customer" => resolve_customer(decl),
        "SupportTicket" => resolve_support_ticket(decl),
        "Invoice" => resolve_invoice(decl),
        "Subscription" | "Plan" => resolve_subscription(decl),
        "Usage" | "UsageMetric" => resolve_usage(decl),
        "Feature" => resolve_feature(decl),
        other => Err(ResolveError::UnknownEntity(other.to_string())),
    }
}

fn resolve_customer(decl: &WriteDeclaration) -> Result<ResolvedWrite, ResolveError> {
    // Determine the specific event based on properties
    if decl.properties.get("plan_id").is_some()
        && decl.properties.get("mrr_cents").is_some()
    {
        let payload: CustomerSubscribedPayload =
            serde_json::from_value(decl.properties.clone())?;
        let event = SaasEvent::CustomerSubscribed(payload);
        let stream_name = event.stream_name();
        let event_type = event.event_type_str().to_string();
        Ok(ResolvedWrite {
            event,
            stream_name,
            event_type,
            validation_notes: vec![],
        })
    } else if decl.properties.get("reason").is_some()
        || decl.properties.get("cancelled_at").is_some()
    {
        let payload: CustomerCancelledPayload =
            serde_json::from_value(decl.properties.clone())?;
        let event = SaasEvent::CustomerCancelled(payload);
        let stream_name = event.stream_name();
        let event_type = event.event_type_str().to_string();
        Ok(ResolvedWrite {
            event,
            stream_name,
            event_type,
            validation_notes: vec![],
        })
    } else {
        Err(ResolveError::Validation(
            "Cannot determine Customer event type from properties. Include plan_id+mrr_cents for subscribe, or reason for cancel.".to_string(),
        ))
    }
}

fn resolve_support_ticket(decl: &WriteDeclaration) -> Result<ResolvedWrite, ResolveError> {
    // If resolution field present, this is a close event
    if decl.properties.get("resolution").is_some() {
        let payload: SupportTicketClosedPayload =
            serde_json::from_value(decl.properties.clone())?;
        let event = SaasEvent::SupportTicketClosed(payload);
        let stream_name = event.stream_name();
        let event_type = event.event_type_str().to_string();
        Ok(ResolvedWrite {
            event,
            stream_name,
            event_type,
            validation_notes: vec![],
        })
    } else {
        let payload: SupportTicketOpenedPayload =
            serde_json::from_value(decl.properties.clone())?;
        let event = SaasEvent::SupportTicketOpened(payload);
        let stream_name = event.stream_name();
        let event_type = event.event_type_str().to_string();
        Ok(ResolvedWrite {
            event,
            stream_name,
            event_type,
            validation_notes: vec![],
        })
    }
}

fn resolve_invoice(decl: &WriteDeclaration) -> Result<ResolvedWrite, ResolveError> {
    // If failure_reason present, this is a failed invoice
    if decl.properties.get("failure_reason").is_some() {
        let payload: InvoiceFailedPayload =
            serde_json::from_value(decl.properties.clone())?;
        let event = SaasEvent::InvoiceFailed(payload);
        let stream_name = event.stream_name();
        let event_type = event.event_type_str().to_string();
        Ok(ResolvedWrite {
            event,
            stream_name,
            event_type,
            validation_notes: vec![],
        })
    } else {
        let payload: InvoicePaidPayload =
            serde_json::from_value(decl.properties.clone())?;
        let event = SaasEvent::InvoicePaid(payload);
        let stream_name = event.stream_name();
        let event_type = event.event_type_str().to_string();
        Ok(ResolvedWrite {
            event,
            stream_name,
            event_type,
            validation_notes: vec![],
        })
    }
}

fn resolve_subscription(decl: &WriteDeclaration) -> Result<ResolvedWrite, ResolveError> {
    if decl.properties.get("from_plan").is_some() {
        let payload: PlanChangedPayload =
            serde_json::from_value(decl.properties.clone())?;
        let event = SaasEvent::PlanChanged(payload);
        let stream_name = event.stream_name();
        let event_type = event.event_type_str().to_string();
        Ok(ResolvedWrite {
            event,
            stream_name,
            event_type,
            validation_notes: vec![],
        })
    } else {
        Err(ResolveError::Validation(
            "Subscription writes require from_plan and to_plan properties.".to_string(),
        ))
    }
}

fn resolve_usage(decl: &WriteDeclaration) -> Result<ResolvedWrite, ResolveError> {
    let payload: UsageRecordedPayload =
        serde_json::from_value(decl.properties.clone())?;
    let event = SaasEvent::UsageRecorded(payload);
    let stream_name = event.stream_name();
    let event_type = event.event_type_str().to_string();
    Ok(ResolvedWrite {
        event,
        stream_name,
        event_type,
        validation_notes: vec![],
    })
}

fn resolve_feature(decl: &WriteDeclaration) -> Result<ResolvedWrite, ResolveError> {
    let payload: FeatureUsagePayload =
        serde_json::from_value(decl.properties.clone())?;
    let event = SaasEvent::FeatureUsage(payload);
    let stream_name = event.stream_name();
    let event_type = event.event_type_str().to_string();
    Ok(ResolvedWrite {
        event,
        stream_name,
        event_type,
        validation_notes: vec![],
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn resolve_support_ticket_opened() {
        let cid = Uuid::new_v4();
        let tid = Uuid::new_v4();
        let decl = WriteDeclaration {
            intent: "Create a support ticket".to_string(),
            entity_type: "SupportTicket".to_string(),
            entity_id: None,
            properties: serde_json::json!({
                "customer_id": cid,
                "ticket_id": tid,
                "priority": "high",
                "category": "billing",
                "opened_at": Utc::now()
            }),
        };

        let resolved = resolve_write(&decl).unwrap();
        assert_eq!(resolved.event_type, "SupportTicketOpened");
        assert!(resolved.stream_name.starts_with("customer-"));
    }

    #[test]
    fn resolve_support_ticket_closed() {
        let cid = Uuid::new_v4();
        let tid = Uuid::new_v4();
        let decl = WriteDeclaration {
            intent: "Close a support ticket".to_string(),
            entity_type: "SupportTicket".to_string(),
            entity_id: Some(tid.to_string()),
            properties: serde_json::json!({
                "customer_id": cid,
                "ticket_id": tid,
                "resolution": "resolved",
                "satisfaction_score": 5,
                "closed_at": Utc::now()
            }),
        };

        let resolved = resolve_write(&decl).unwrap();
        assert_eq!(resolved.event_type, "SupportTicketClosed");
    }

    #[test]
    fn resolve_invoice_paid() {
        let cid = Uuid::new_v4();
        let iid = Uuid::new_v4();
        let decl = WriteDeclaration {
            intent: "Record paid invoice".to_string(),
            entity_type: "Invoice".to_string(),
            entity_id: None,
            properties: serde_json::json!({
                "customer_id": cid,
                "invoice_id": iid,
                "amount_cents": 9900,
                "paid_at": Utc::now()
            }),
        };

        let resolved = resolve_write(&decl).unwrap();
        assert_eq!(resolved.event_type, "InvoicePaid");
    }

    #[test]
    fn resolve_invoice_failed() {
        let cid = Uuid::new_v4();
        let iid = Uuid::new_v4();
        let decl = WriteDeclaration {
            intent: "Record failed invoice".to_string(),
            entity_type: "Invoice".to_string(),
            entity_id: None,
            properties: serde_json::json!({
                "customer_id": cid,
                "invoice_id": iid,
                "amount_cents": 9900,
                "failure_reason": "card_declined",
                "attempt_number": 1,
                "failed_at": Utc::now()
            }),
        };

        let resolved = resolve_write(&decl).unwrap();
        assert_eq!(resolved.event_type, "InvoiceFailed");
    }

    #[test]
    fn resolve_usage_metric() {
        let cid = Uuid::new_v4();
        let decl = WriteDeclaration {
            intent: "Record usage".to_string(),
            entity_type: "Usage".to_string(),
            entity_id: None,
            properties: serde_json::json!({
                "customer_id": cid,
                "metric": "api_calls",
                "value": 1500.0,
                "recorded_at": Utc::now()
            }),
        };

        let resolved = resolve_write(&decl).unwrap();
        assert_eq!(resolved.event_type, "UsageRecorded");
    }

    #[test]
    fn resolve_unknown_entity_errors() {
        let decl = WriteDeclaration {
            intent: "test".to_string(),
            entity_type: "Unknown".to_string(),
            entity_id: None,
            properties: serde_json::json!({}),
        };

        let result = resolve_write(&decl);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown"));
    }

    #[test]
    fn resolve_bad_properties_errors() {
        let decl = WriteDeclaration {
            intent: "test".to_string(),
            entity_type: "SupportTicket".to_string(),
            entity_id: None,
            properties: serde_json::json!({"invalid": "data"}),
        };

        let result = resolve_write(&decl);
        assert!(result.is_err());
    }
}
