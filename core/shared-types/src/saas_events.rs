use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// All SaaS event types supported by the prototype.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum SaasEvent {
    CustomerSubscribed(CustomerSubscribedPayload),
    CustomerCancelled(CustomerCancelledPayload),
    PlanChanged(PlanChangedPayload),
    PriceChanged(PriceChangedPayload),
    UsageRecorded(UsageRecordedPayload),
    LoginEvent(LoginEventPayload),
    SupportTicketOpened(SupportTicketOpenedPayload),
    SupportTicketClosed(SupportTicketClosedPayload),
    InvoicePaid(InvoicePaidPayload),
    InvoiceFailed(InvoiceFailedPayload),
    TrialStarted(TrialStartedPayload),
    TrialConverted(TrialConvertedPayload),
    FeatureUsage(FeatureUsagePayload),
    SeatCountChanged(SeatCountChangedPayload),
    CapacityThresholdReached(CapacityThresholdReachedPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerSubscribedPayload {
    pub customer_id: Uuid,
    pub plan_id: String,
    pub mrr_cents: u64,
    pub seat_count: u32,
    pub subscribed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerCancelledPayload {
    pub customer_id: Uuid,
    pub reason: Option<String>,
    pub feedback: Option<String>,
    pub cancelled_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanChangedPayload {
    pub customer_id: Uuid,
    pub from_plan: String,
    pub to_plan: String,
    pub mrr_delta_cents: i64,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceChangedPayload {
    pub plan_id: String,
    pub old_price_cents: u64,
    pub new_price_cents: u64,
    pub effective_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecordedPayload {
    pub customer_id: Uuid,
    pub metric: String,
    pub value: f64,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginEventPayload {
    pub customer_id: Uuid,
    pub user_id: Uuid,
    pub login_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicketOpenedPayload {
    pub customer_id: Uuid,
    pub ticket_id: Uuid,
    pub priority: String,
    pub category: String,
    pub opened_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicketClosedPayload {
    pub customer_id: Uuid,
    pub ticket_id: Uuid,
    pub resolution: String,
    pub satisfaction_score: Option<u8>,
    pub closed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoicePaidPayload {
    pub customer_id: Uuid,
    pub invoice_id: Uuid,
    pub amount_cents: u64,
    pub paid_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceFailedPayload {
    pub customer_id: Uuid,
    pub invoice_id: Uuid,
    pub amount_cents: u64,
    pub failure_reason: String,
    pub attempt_number: u32,
    pub failed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialStartedPayload {
    pub customer_id: Uuid,
    pub plan_id: String,
    pub trial_days: u32,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialConvertedPayload {
    pub customer_id: Uuid,
    pub plan_id: String,
    pub mrr_cents: u64,
    pub converted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureUsagePayload {
    pub customer_id: Uuid,
    pub feature_name: String,
    pub usage_count: u64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatCountChangedPayload {
    pub customer_id: Uuid,
    pub old_count: u32,
    pub new_count: u32,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityThresholdReachedPayload {
    pub customer_id: Uuid,
    pub resource: String,
    pub current_usage_pct: f64,
    pub threshold_pct: f64,
    pub reached_at: DateTime<Utc>,
}

impl SaasEvent {
    pub fn event_type_str(&self) -> &'static str {
        match self {
            SaasEvent::CustomerSubscribed(_) => "CustomerSubscribed",
            SaasEvent::CustomerCancelled(_) => "CustomerCancelled",
            SaasEvent::PlanChanged(_) => "PlanChanged",
            SaasEvent::PriceChanged(_) => "PriceChanged",
            SaasEvent::UsageRecorded(_) => "UsageRecorded",
            SaasEvent::LoginEvent(_) => "LoginEvent",
            SaasEvent::SupportTicketOpened(_) => "SupportTicketOpened",
            SaasEvent::SupportTicketClosed(_) => "SupportTicketClosed",
            SaasEvent::InvoicePaid(_) => "InvoicePaid",
            SaasEvent::InvoiceFailed(_) => "InvoiceFailed",
            SaasEvent::TrialStarted(_) => "TrialStarted",
            SaasEvent::TrialConverted(_) => "TrialConverted",
            SaasEvent::FeatureUsage(_) => "FeatureUsage",
            SaasEvent::SeatCountChanged(_) => "SeatCountChanged",
            SaasEvent::CapacityThresholdReached(_) => "CapacityThresholdReached",
        }
    }

    pub fn customer_id(&self) -> Option<Uuid> {
        match self {
            SaasEvent::CustomerSubscribed(p) => Some(p.customer_id),
            SaasEvent::CustomerCancelled(p) => Some(p.customer_id),
            SaasEvent::PlanChanged(p) => Some(p.customer_id),
            SaasEvent::PriceChanged(_) => None,
            SaasEvent::UsageRecorded(p) => Some(p.customer_id),
            SaasEvent::LoginEvent(p) => Some(p.customer_id),
            SaasEvent::SupportTicketOpened(p) => Some(p.customer_id),
            SaasEvent::SupportTicketClosed(p) => Some(p.customer_id),
            SaasEvent::InvoicePaid(p) => Some(p.customer_id),
            SaasEvent::InvoiceFailed(p) => Some(p.customer_id),
            SaasEvent::TrialStarted(p) => Some(p.customer_id),
            SaasEvent::TrialConverted(p) => Some(p.customer_id),
            SaasEvent::FeatureUsage(p) => Some(p.customer_id),
            SaasEvent::SeatCountChanged(p) => Some(p.customer_id),
            SaasEvent::CapacityThresholdReached(p) => Some(p.customer_id),
        }
    }

    pub fn stream_name(&self) -> String {
        match self {
            SaasEvent::PriceChanged(p) => format!("plan-{}", p.plan_id),
            other => match other.customer_id() {
                Some(cid) => format!("customer-{}", cid),
                None => "system".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip_customer_subscribed() {
        let event = SaasEvent::CustomerSubscribed(CustomerSubscribedPayload {
            customer_id: Uuid::new_v4(),
            plan_id: "pro".to_string(),
            mrr_cents: 9900,
            seat_count: 5,
            subscribed_at: Utc::now(),
        });
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: SaasEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.event_type_str(), deserialized.event_type_str());
    }

    #[test]
    fn event_type_str_values() {
        let event = SaasEvent::InvoiceFailed(InvoiceFailedPayload {
            customer_id: Uuid::new_v4(),
            invoice_id: Uuid::new_v4(),
            amount_cents: 5000,
            failure_reason: "card_declined".to_string(),
            attempt_number: 1,
            failed_at: Utc::now(),
        });
        assert_eq!(event.event_type_str(), "InvoiceFailed");
    }

    #[test]
    fn stream_name_customer_event() {
        let cid = Uuid::new_v4();
        let event = SaasEvent::LoginEvent(LoginEventPayload {
            customer_id: cid,
            user_id: Uuid::new_v4(),
            login_at: Utc::now(),
        });
        assert_eq!(event.stream_name(), format!("customer-{}", cid));
    }

    #[test]
    fn stream_name_plan_event() {
        let event = SaasEvent::PriceChanged(PriceChangedPayload {
            plan_id: "enterprise".to_string(),
            old_price_cents: 19900,
            new_price_cents: 24900,
            effective_at: Utc::now(),
        });
        assert_eq!(event.stream_name(), "plan-enterprise");
    }

    #[test]
    fn customer_id_none_for_plan_events() {
        let event = SaasEvent::PriceChanged(PriceChangedPayload {
            plan_id: "basic".to_string(),
            old_price_cents: 900,
            new_price_cents: 1200,
            effective_at: Utc::now(),
        });
        assert!(event.customer_id().is_none());
    }

    #[test]
    fn all_variants_roundtrip() {
        let cid = Uuid::new_v4();
        let now = Utc::now();
        let events = vec![
            SaasEvent::CustomerSubscribed(CustomerSubscribedPayload {
                customer_id: cid, plan_id: "pro".into(), mrr_cents: 9900,
                seat_count: 5, subscribed_at: now,
            }),
            SaasEvent::CustomerCancelled(CustomerCancelledPayload {
                customer_id: cid, reason: Some("too expensive".into()),
                feedback: None, cancelled_at: now,
            }),
            SaasEvent::PlanChanged(PlanChangedPayload {
                customer_id: cid, from_plan: "basic".into(), to_plan: "pro".into(),
                mrr_delta_cents: 5000, changed_at: now,
            }),
            SaasEvent::PriceChanged(PriceChangedPayload {
                plan_id: "pro".into(), old_price_cents: 9900,
                new_price_cents: 12900, effective_at: now,
            }),
            SaasEvent::UsageRecorded(UsageRecordedPayload {
                customer_id: cid, metric: "api_calls".into(), value: 1500.0,
                recorded_at: now,
            }),
            SaasEvent::LoginEvent(LoginEventPayload {
                customer_id: cid, user_id: Uuid::new_v4(), login_at: now,
            }),
            SaasEvent::SupportTicketOpened(SupportTicketOpenedPayload {
                customer_id: cid, ticket_id: Uuid::new_v4(), priority: "high".into(),
                category: "billing".into(), opened_at: now,
            }),
            SaasEvent::SupportTicketClosed(SupportTicketClosedPayload {
                customer_id: cid, ticket_id: Uuid::new_v4(), resolution: "resolved".into(),
                satisfaction_score: Some(4), closed_at: now,
            }),
            SaasEvent::InvoicePaid(InvoicePaidPayload {
                customer_id: cid, invoice_id: Uuid::new_v4(), amount_cents: 9900,
                paid_at: now,
            }),
            SaasEvent::InvoiceFailed(InvoiceFailedPayload {
                customer_id: cid, invoice_id: Uuid::new_v4(), amount_cents: 9900,
                failure_reason: "insufficient_funds".into(), attempt_number: 1,
                failed_at: now,
            }),
            SaasEvent::TrialStarted(TrialStartedPayload {
                customer_id: cid, plan_id: "pro".into(), trial_days: 14,
                started_at: now,
            }),
            SaasEvent::TrialConverted(TrialConvertedPayload {
                customer_id: cid, plan_id: "pro".into(), mrr_cents: 9900,
                converted_at: now,
            }),
            SaasEvent::FeatureUsage(FeatureUsagePayload {
                customer_id: cid, feature_name: "dashboard".into(), usage_count: 42,
                period_start: now, period_end: now,
            }),
            SaasEvent::SeatCountChanged(SeatCountChangedPayload {
                customer_id: cid, old_count: 5, new_count: 10, changed_at: now,
            }),
            SaasEvent::CapacityThresholdReached(CapacityThresholdReachedPayload {
                customer_id: cid, resource: "storage".into(), current_usage_pct: 85.0,
                threshold_pct: 80.0, reached_at: now,
            }),
        ];

        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let _: SaasEvent = serde_json::from_str(&json).unwrap();
        }
        assert_eq!(events.len(), 15);
    }
}
