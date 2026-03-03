# ANIDB Open Questions

Track unresolved design decisions here. Do not make assumptions —
document the question and resolve it before building on it.

## Active Questions

### OQ-001: Anthropic Rust SDK Selection
- **Context:** Multiple unofficial Rust crates exist for the Anthropic API. None are officially maintained by Anthropic.
- **Options:** (a) Use an existing community crate, (b) Use reqwest directly with typed request/response structs, (c) Write a thin wrapper crate
- **Decision:** TBD
- **Impact:** Semantic engine, ontology service

### OQ-002: DuckDB Integration Point
- **Context:** DuckDB is in-process. Which service(s) should embed it?
- **Options:** (a) Only the semantic engine embeds DuckDB for analytics during context bundling, (b) A separate columnar-store crate that other services call via a trait
- **Decision:** TBD
- **Impact:** Workspace structure, query planner

### OQ-003: Subscription Delivery Mechanism
- **Context:** How do subscriptions reach agents? The design doc mentions webhooks or SSE.
- **Options:** (a) Webhook callbacks, (b) Server-Sent Events, (c) Both, configurable per subscription
- **Decision:** TBD
- **Impact:** API gateway, subscription engine

## Resolved Questions
(none yet)
