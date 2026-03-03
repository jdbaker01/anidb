use anidb_shared_types::saas_events::*;
use neo4rs::query;

use crate::client::{GraphClient, GraphError};
use crate::queries;

/// Project a single SaaS event into the knowledge graph.
/// This is the core event-to-graph synchronization logic.
pub async fn sync_event(client: &GraphClient, event: &SaasEvent) -> Result<(), GraphError> {
    match event {
        SaasEvent::CustomerSubscribed(p) => sync_customer_subscribed(client, p).await,
        SaasEvent::CustomerCancelled(p) => sync_customer_cancelled(client, p).await,
        SaasEvent::PlanChanged(p) => sync_plan_changed(client, p).await,
        SaasEvent::PriceChanged(p) => sync_price_changed(client, p).await,
        SaasEvent::UsageRecorded(p) => sync_usage_recorded(client, p).await,
        SaasEvent::LoginEvent(p) => sync_login(client, p).await,
        SaasEvent::SupportTicketOpened(p) => sync_ticket_opened(client, p).await,
        SaasEvent::SupportTicketClosed(p) => sync_ticket_closed(client, p).await,
        SaasEvent::InvoicePaid(p) => sync_invoice_paid(client, p).await,
        SaasEvent::InvoiceFailed(p) => sync_invoice_failed(client, p).await,
        SaasEvent::TrialStarted(p) => sync_trial_started(client, p).await,
        SaasEvent::TrialConverted(p) => sync_trial_converted(client, p).await,
        SaasEvent::FeatureUsage(p) => sync_feature_usage(client, p).await,
        SaasEvent::SeatCountChanged(p) => sync_seat_count_changed(client, p).await,
        SaasEvent::CapacityThresholdReached(p) => sync_capacity_threshold(client, p).await,
    }
}

async fn sync_customer_subscribed(
    client: &GraphClient,
    p: &CustomerSubscribedPayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    client
        .run(queries::merge_customer(
            &cid,
            "active",
            p.mrr_cents as i64,
            p.seat_count as i64,
        ))
        .await?;
    client.run(queries::merge_plan(&p.plan_id, 0)).await?;
    client.run(queries::create_subscribes_to(&cid, &p.plan_id)).await?;
    tracing::debug!(customer_id = %cid, plan = %p.plan_id, "Synced CustomerSubscribed");
    Ok(())
}

async fn sync_customer_cancelled(
    client: &GraphClient,
    p: &CustomerCancelledPayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    client.run(queries::mark_customer_churned(&cid)).await?;
    tracing::debug!(customer_id = %cid, "Synced CustomerCancelled");
    Ok(())
}

async fn sync_plan_changed(
    client: &GraphClient,
    p: &PlanChangedPayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    client.run(queries::merge_plan(&p.to_plan, 0)).await?;
    client
        .run(queries::change_subscription(&cid, &p.from_plan, &p.to_plan))
        .await?;
    tracing::debug!(customer_id = %cid, from = %p.from_plan, to = %p.to_plan, "Synced PlanChanged");
    Ok(())
}

async fn sync_price_changed(
    client: &GraphClient,
    p: &PriceChangedPayload,
) -> Result<(), GraphError> {
    client
        .run(queries::merge_plan(&p.plan_id, p.new_price_cents as i64))
        .await?;
    tracing::debug!(plan = %p.plan_id, price = p.new_price_cents, "Synced PriceChanged");
    Ok(())
}

async fn sync_usage_recorded(
    client: &GraphClient,
    p: &UsageRecordedPayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    let q = query(
        "MATCH (c:Customer {customer_id: $cid})
         SET c.last_usage_metric = $metric,
             c.last_usage_value = $value,
             c.last_usage_at = datetime($recorded_at)",
    )
    .param("cid", cid.clone())
    .param("metric", p.metric.clone())
    .param("value", p.value)
    .param("recorded_at", p.recorded_at.to_rfc3339());
    client.run(q).await?;
    tracing::debug!(customer_id = %cid, metric = %p.metric, "Synced UsageRecorded");
    Ok(())
}

async fn sync_login(client: &GraphClient, p: &LoginEventPayload) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    client
        .run(queries::record_login(&cid, &p.login_at.to_rfc3339()))
        .await?;
    tracing::debug!(customer_id = %cid, "Synced LoginEvent");
    Ok(())
}

async fn sync_ticket_opened(
    client: &GraphClient,
    p: &SupportTicketOpenedPayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    let tid = p.ticket_id.to_string();
    client
        .run(queries::create_support_ticket(
            &tid,
            &cid,
            &p.priority,
            &p.category,
            "open",
        ))
        .await?;
    tracing::debug!(customer_id = %cid, ticket_id = %tid, "Synced SupportTicketOpened");
    Ok(())
}

async fn sync_ticket_closed(
    client: &GraphClient,
    p: &SupportTicketClosedPayload,
) -> Result<(), GraphError> {
    let tid = p.ticket_id.to_string();
    let score = p.satisfaction_score.map(|s| s as i64);
    client
        .run(queries::close_support_ticket(&tid, &p.resolution, score))
        .await?;
    tracing::debug!(ticket_id = %tid, "Synced SupportTicketClosed");
    Ok(())
}

async fn sync_invoice_paid(
    client: &GraphClient,
    p: &InvoicePaidPayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    let iid = p.invoice_id.to_string();
    client
        .run(queries::create_invoice(
            &iid,
            &cid,
            p.amount_cents as i64,
            "paid",
        ))
        .await?;
    tracing::debug!(customer_id = %cid, invoice_id = %iid, "Synced InvoicePaid");
    Ok(())
}

async fn sync_invoice_failed(
    client: &GraphClient,
    p: &InvoiceFailedPayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    let iid = p.invoice_id.to_string();
    client
        .run(queries::create_invoice(
            &iid,
            &cid,
            p.amount_cents as i64,
            "failed",
        ))
        .await?;
    tracing::debug!(customer_id = %cid, invoice_id = %iid, "Synced InvoiceFailed");
    Ok(())
}

async fn sync_trial_started(
    client: &GraphClient,
    p: &TrialStartedPayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    client
        .run(queries::merge_customer(&cid, "trial", 0, 0))
        .await?;
    client.run(queries::merge_plan(&p.plan_id, 0)).await?;
    client.run(queries::create_subscribes_to(&cid, &p.plan_id)).await?;
    tracing::debug!(customer_id = %cid, "Synced TrialStarted");
    Ok(())
}

async fn sync_trial_converted(
    client: &GraphClient,
    p: &TrialConvertedPayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    let q = query(
        "MATCH (c:Customer {customer_id: $cid})
         SET c.status = 'active',
             c.mrr_cents = $mrr,
             c.converted_at = datetime(),
             c.updated_at = datetime()",
    )
    .param("cid", cid.clone())
    .param("mrr", p.mrr_cents as i64);
    client.run(q).await?;
    tracing::debug!(customer_id = %cid, "Synced TrialConverted");
    Ok(())
}

async fn sync_feature_usage(
    client: &GraphClient,
    p: &FeatureUsagePayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    client.run(queries::merge_feature(&p.feature_name)).await?;
    client
        .run(queries::merge_uses_feature(
            &cid,
            &p.feature_name,
            p.usage_count as i64,
        ))
        .await?;
    tracing::debug!(customer_id = %cid, feature = %p.feature_name, "Synced FeatureUsage");
    Ok(())
}

async fn sync_seat_count_changed(
    client: &GraphClient,
    p: &SeatCountChangedPayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    client
        .run(queries::update_seat_count(&cid, p.new_count as i64))
        .await?;
    tracing::debug!(customer_id = %cid, new_count = p.new_count, "Synced SeatCountChanged");
    Ok(())
}

async fn sync_capacity_threshold(
    client: &GraphClient,
    p: &CapacityThresholdReachedPayload,
) -> Result<(), GraphError> {
    let cid = p.customer_id.to_string();
    let q = query(
        "MATCH (c:Customer {customer_id: $cid})
         SET c.capacity_resource = $resource,
             c.capacity_usage_pct = $usage_pct,
             c.capacity_alert_at = datetime()",
    )
    .param("cid", cid.clone())
    .param("resource", p.resource.clone())
    .param("usage_pct", p.current_usage_pct);
    client.run(q).await?;
    tracing::debug!(customer_id = %cid, resource = %p.resource, "Synced CapacityThresholdReached");
    Ok(())
}
