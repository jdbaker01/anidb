use neo4rs::{query, Query};

// ============================================================================
// Ontology metadata operations
// ============================================================================

/// Seed an entity type definition into Neo4j as an OntologyEntityType node.
pub fn merge_entity_type_def(
    name: &str,
    rea_primitive: &str,
    description: &str,
    properties_json: &str,
    source_events_json: &str,
    archetype: &str,
    version: u32,
) -> Query {
    query(
        "MERGE (e:OntologyEntityType {name: $name})
         SET e.rea_primitive = $rea_primitive,
             e.description = $description,
             e.properties = $properties,
             e.source_events = $source_events,
             e.archetype = $archetype,
             e.version = $version",
    )
    .param("name", name.to_string())
    .param("rea_primitive", rea_primitive.to_string())
    .param("description", description.to_string())
    .param("properties", properties_json.to_string())
    .param("source_events", source_events_json.to_string())
    .param("archetype", archetype.to_string())
    .param("version", version as i64)
}

/// Seed a relationship type definition.
pub fn merge_relationship_type_def(
    name: &str,
    from_entity: &str,
    to_entity: &str,
    rea_relationship: &str,
    description: &str,
) -> Query {
    query(
        "MERGE (r:OntologyRelationshipType {name: $name})
         SET r.from_entity = $from_entity,
             r.to_entity = $to_entity,
             r.rea_relationship = $rea_relationship,
             r.description = $description",
    )
    .param("name", name.to_string())
    .param("from_entity", from_entity.to_string())
    .param("to_entity", to_entity.to_string())
    .param("rea_relationship", rea_relationship.to_string())
    .param("description", description.to_string())
}

/// Seed a causal belief.
pub fn merge_causal_belief(
    name: &str,
    cause: &str,
    effect: &str,
    strength: f64,
    decision_classes_json: &str,
    description: &str,
) -> Query {
    query(
        "MERGE (cb:CausalBelief {name: $name})
         SET cb.cause = $cause,
             cb.effect = $effect,
             cb.strength = $strength,
             cb.decision_classes = $decision_classes,
             cb.description = $description",
    )
    .param("name", name.to_string())
    .param("cause", cause.to_string())
    .param("effect", effect.to_string())
    .param("strength", strength)
    .param("decision_classes", decision_classes_json.to_string())
    .param("description", description.to_string())
}

/// Set the ontology version (singleton node).
pub fn set_ontology_version(version: u32) -> Query {
    query(
        "MERGE (v:OntologyVersion {singleton: true})
         SET v.version = $version",
    )
    .param("version", version as i64)
}

/// Get the current ontology version.
pub fn get_ontology_version() -> Query {
    query("MATCH (v:OntologyVersion) RETURN v.version AS version")
}

// ============================================================================
// Instance entity operations (used by sync.rs when events arrive)
// ============================================================================

/// Create or update a Customer node (dual-labeled Customer:Party).
pub fn merge_customer(customer_id: &str, status: &str, mrr_cents: i64, seat_count: i64) -> Query {
    query(
        "MERGE (c:Customer:Party {customer_id: $customer_id})
         SET c.status = $status,
             c.mrr_cents = $mrr_cents,
             c.seat_count = $seat_count,
             c.updated_at = datetime()",
    )
    .param("customer_id", customer_id.to_string())
    .param("status", status.to_string())
    .param("mrr_cents", mrr_cents)
    .param("seat_count", seat_count)
}

/// Create or update a Plan node (dual-labeled Plan:Resource).
pub fn merge_plan(plan_id: &str, price_cents: i64) -> Query {
    query(
        "MERGE (p:Plan:Resource {plan_id: $plan_id})
         SET p.price_cents = $price_cents,
             p.updated_at = datetime()",
    )
    .param("plan_id", plan_id.to_string())
    .param("price_cents", price_cents)
}

/// Create or update a Feature node (dual-labeled Feature:Resource).
pub fn merge_feature(feature_name: &str) -> Query {
    query(
        "MERGE (f:Feature:Resource {feature_name: $feature_name})
         SET f.updated_at = datetime()",
    )
    .param("feature_name", feature_name.to_string())
}

/// Create SUBSCRIBES_TO relationship between Customer and Plan.
pub fn create_subscribes_to(customer_id: &str, plan_id: &str) -> Query {
    query(
        "MATCH (c:Customer {customer_id: $customer_id}),
               (p:Plan {plan_id: $plan_id})
         MERGE (c)-[:SUBSCRIBES_TO]->(p)",
    )
    .param("customer_id", customer_id.to_string())
    .param("plan_id", plan_id.to_string())
}

/// Switch subscription from old plan to new plan.
pub fn change_subscription(customer_id: &str, old_plan_id: &str, new_plan_id: &str) -> Query {
    query(
        "MATCH (c:Customer {customer_id: $customer_id})-[r:SUBSCRIBES_TO]->(old:Plan {plan_id: $old_plan_id})
         DELETE r
         WITH c
         MATCH (new:Plan {plan_id: $new_plan_id})
         MERGE (c)-[:SUBSCRIBES_TO]->(new)",
    )
    .param("customer_id", customer_id.to_string())
    .param("old_plan_id", old_plan_id.to_string())
    .param("new_plan_id", new_plan_id.to_string())
}

/// Record feature usage relationship.
pub fn merge_uses_feature(customer_id: &str, feature_name: &str, usage_count: i64) -> Query {
    query(
        "MATCH (c:Customer {customer_id: $customer_id}),
               (f:Feature {feature_name: $feature_name})
         MERGE (c)-[r:USES_FEATURE]->(f)
         SET r.usage_count = $usage_count,
             r.updated_at = datetime()",
    )
    .param("customer_id", customer_id.to_string())
    .param("feature_name", feature_name.to_string())
    .param("usage_count", usage_count)
}

/// Create an Invoice node and link to Customer.
pub fn create_invoice(
    invoice_id: &str,
    customer_id: &str,
    amount_cents: i64,
    status: &str,
) -> Query {
    query(
        "MERGE (i:Invoice:EconomicEvent {invoice_id: $invoice_id})
         SET i.amount_cents = $amount_cents,
             i.status = $status,
             i.updated_at = datetime()
         WITH i
         MATCH (c:Customer {customer_id: $customer_id})
         MERGE (i)-[:BILLED_TO]->(c)",
    )
    .param("invoice_id", invoice_id.to_string())
    .param("customer_id", customer_id.to_string())
    .param("amount_cents", amount_cents)
    .param("status", status.to_string())
}

/// Create a SupportTicket node and link to Customer.
pub fn create_support_ticket(
    ticket_id: &str,
    customer_id: &str,
    priority: &str,
    category: &str,
    status: &str,
) -> Query {
    query(
        "MERGE (t:SupportTicket:EconomicEvent {ticket_id: $ticket_id})
         SET t.priority = $priority,
             t.category = $category,
             t.status = $status,
             t.updated_at = datetime()
         WITH t
         MATCH (c:Customer {customer_id: $customer_id})
         MERGE (t)-[:OPENED_BY]->(c)",
    )
    .param("ticket_id", ticket_id.to_string())
    .param("customer_id", customer_id.to_string())
    .param("priority", priority.to_string())
    .param("category", category.to_string())
    .param("status", status.to_string())
}

/// Close a support ticket.
pub fn close_support_ticket(
    ticket_id: &str,
    resolution: &str,
    satisfaction_score: Option<i64>,
) -> Query {
    query(
        "MATCH (t:SupportTicket {ticket_id: $ticket_id})
         SET t.status = 'closed',
             t.resolution = $resolution,
             t.satisfaction_score = $satisfaction_score,
             t.updated_at = datetime()",
    )
    .param("ticket_id", ticket_id.to_string())
    .param("resolution", resolution.to_string())
    .param("satisfaction_score", satisfaction_score.unwrap_or(-1))
}

/// Update Customer's last_login_at and increment login_count.
pub fn record_login(customer_id: &str, login_at: &str) -> Query {
    query(
        "MATCH (c:Customer {customer_id: $customer_id})
         SET c.last_login_at = datetime($login_at),
             c.login_count = COALESCE(c.login_count, 0) + 1",
    )
    .param("customer_id", customer_id.to_string())
    .param("login_at", login_at.to_string())
}

/// Set customer status to churned.
pub fn mark_customer_churned(customer_id: &str) -> Query {
    query(
        "MATCH (c:Customer {customer_id: $customer_id})
         SET c.status = 'churned',
             c.churned_at = datetime(),
             c.updated_at = datetime()",
    )
    .param("customer_id", customer_id.to_string())
}

/// Update seat count on customer.
pub fn update_seat_count(customer_id: &str, new_count: i64) -> Query {
    query(
        "MATCH (c:Customer {customer_id: $customer_id})
         SET c.seat_count = $new_count,
             c.updated_at = datetime()",
    )
    .param("customer_id", customer_id.to_string())
    .param("new_count", new_count)
}

// ============================================================================
// Read queries (ontology service + future semantic engine)
// ============================================================================

/// List all ontology entity type definitions.
pub fn list_entity_types() -> Query {
    query("MATCH (e:OntologyEntityType) RETURN e ORDER BY e.name")
}

/// Get a single entity type definition by name.
pub fn get_entity_type(name: &str) -> Query {
    query("MATCH (e:OntologyEntityType {name: $name}) RETURN e").param("name", name.to_string())
}

/// List all ontology relationship type definitions.
pub fn list_relationship_types() -> Query {
    query("MATCH (r:OntologyRelationshipType) RETURN r ORDER BY r.name")
}

/// List all causal beliefs, optionally filtered by decision class.
pub fn list_causal_beliefs(decision_class: Option<&str>) -> Query {
    match decision_class {
        Some(dc) => query(
            "MATCH (cb:CausalBelief)
             WHERE cb.decision_classes CONTAINS $dc
             RETURN cb ORDER BY cb.strength DESC",
        )
        .param("dc", dc.to_string()),
        None => query("MATCH (cb:CausalBelief) RETURN cb ORDER BY cb.strength DESC"),
    }
}

/// Get a customer with all their relationships (for context bundling).
pub fn get_customer_context(customer_id: &str) -> Query {
    query(
        "MATCH (c:Customer {customer_id: $customer_id})
         OPTIONAL MATCH (c)-[sub:SUBSCRIBES_TO]->(p:Plan)
         OPTIONAL MATCH (c)-[uf:USES_FEATURE]->(f:Feature)
         OPTIONAL MATCH (t:SupportTicket)-[:OPENED_BY]->(c)
         OPTIONAL MATCH (i:Invoice)-[:BILLED_TO]->(c)
         RETURN c, p,
                collect(DISTINCT {feature: f.feature_name, usage_count: uf.usage_count}) AS features,
                collect(DISTINCT {ticket_id: t.ticket_id, status: t.status, priority: t.priority}) AS tickets,
                collect(DISTINCT {invoice_id: i.invoice_id, status: i.status, amount: i.amount_cents}) AS invoices",
    )
    .param("customer_id", customer_id.to_string())
}

/// Traverse causal chain for a decision class.
pub fn get_causal_chain(decision_class: &str) -> Query {
    query(
        "MATCH (cb:CausalBelief)
         WHERE cb.decision_classes CONTAINS $dc
         RETURN cb.name AS belief_name,
                cb.cause AS cause,
                cb.effect AS effect,
                cb.strength AS strength,
                cb.description AS description
         ORDER BY abs(cb.strength) DESC",
    )
    .param("dc", decision_class.to_string())
}

/// Find all customers matching a status.
pub fn find_customers_by_status(status: &str) -> Query {
    query(
        "MATCH (c:Customer {status: $status})
         RETURN c ORDER BY c.mrr_cents DESC",
    )
    .param("status", status.to_string())
}
