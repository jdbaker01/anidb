pub const VALID_EVENT_TYPES: &[&str] = &[
    "CustomerSubscribed",
    "CustomerCancelled",
    "PlanChanged",
    "PriceChanged",
    "UsageRecorded",
    "LoginEvent",
    "SupportTicketOpened",
    "SupportTicketClosed",
    "InvoicePaid",
    "InvoiceFailed",
    "TrialStarted",
    "TrialConverted",
    "FeatureUsage",
    "SeatCountChanged",
    "CapacityThresholdReached",
];

pub fn is_valid_event_type(event_type: &str) -> bool {
    VALID_EVENT_TYPES.contains(&event_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_event_types() {
        assert!(is_valid_event_type("CustomerSubscribed"));
        assert!(is_valid_event_type("CapacityThresholdReached"));
        assert!(!is_valid_event_type("FakeEvent"));
        assert!(!is_valid_event_type(""));
    }
}
