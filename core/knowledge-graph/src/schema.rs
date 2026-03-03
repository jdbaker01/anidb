use neo4rs::query;

use crate::client::{GraphClient, GraphError};

/// Initialize the Neo4j schema with constraints and indexes.
/// Safe to run multiple times (CREATE ... IF NOT EXISTS).
pub async fn initialize_schema(client: &GraphClient) -> Result<(), GraphError> {
    let statements = schema_statements();
    for cypher in &statements {
        client.run(query(cypher)).await?;
    }
    tracing::info!(
        constraint_and_index_count = statements.len(),
        "Neo4j schema initialized"
    );
    Ok(())
}

/// Returns all schema DDL statements.
pub fn schema_statements() -> Vec<String> {
    vec![
        // === Uniqueness constraints ===
        "CREATE CONSTRAINT customer_id_unique IF NOT EXISTS FOR (c:Customer) REQUIRE c.customer_id IS UNIQUE".into(),
        "CREATE CONSTRAINT plan_id_unique IF NOT EXISTS FOR (p:Plan) REQUIRE p.plan_id IS UNIQUE".into(),
        "CREATE CONSTRAINT feature_name_unique IF NOT EXISTS FOR (f:Feature) REQUIRE f.feature_name IS UNIQUE".into(),
        "CREATE CONSTRAINT invoice_id_unique IF NOT EXISTS FOR (i:Invoice) REQUIRE i.invoice_id IS UNIQUE".into(),
        "CREATE CONSTRAINT ticket_id_unique IF NOT EXISTS FOR (t:SupportTicket) REQUIRE t.ticket_id IS UNIQUE".into(),
        "CREATE CONSTRAINT entity_type_name_unique IF NOT EXISTS FOR (e:OntologyEntityType) REQUIRE e.name IS UNIQUE".into(),
        "CREATE CONSTRAINT rel_type_name_unique IF NOT EXISTS FOR (r:OntologyRelationshipType) REQUIRE r.name IS UNIQUE".into(),
        "CREATE CONSTRAINT causal_belief_name_unique IF NOT EXISTS FOR (cb:CausalBelief) REQUIRE cb.name IS UNIQUE".into(),
        // === Indexes for query patterns ===
        "CREATE INDEX customer_status_idx IF NOT EXISTS FOR (c:Customer) ON (c.status)".into(),
        "CREATE INDEX customer_mrr_idx IF NOT EXISTS FOR (c:Customer) ON (c.mrr_cents)".into(),
        "CREATE INDEX ticket_status_idx IF NOT EXISTS FOR (t:SupportTicket) ON (t.status)".into(),
        "CREATE INDEX invoice_status_idx IF NOT EXISTS FOR (i:Invoice) ON (i.status)".into(),
        "CREATE INDEX usage_metric_name_idx IF NOT EXISTS FOR (u:UsageMetric) ON (u.metric)".into(),
        "CREATE INDEX entity_type_rea_idx IF NOT EXISTS FOR (e:OntologyEntityType) ON (e.rea_primitive)".into(),
    ]
}

/// Drop all ANIDB-created nodes. Used in tests.
pub async fn drop_all(client: &GraphClient) -> Result<(), GraphError> {
    client.run(query("MATCH (n) DETACH DELETE n")).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_statements_are_non_empty() {
        let stmts = schema_statements();
        assert!(!stmts.is_empty());
        for s in &stmts {
            assert!(
                s.contains("CREATE"),
                "Statement does not contain CREATE: {}",
                s
            );
        }
    }
}
