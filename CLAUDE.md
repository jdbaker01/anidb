# ANIDB — Agent-Native Intelligent Database
## Claude Code Instructions

This file defines how to build, run, and extend the ANIDB prototype. Read it fully before making any changes.

---

## Project Overview

ANIDB is an intent-semantic data layer designed for AI agents to operate businesses. Instead of SQL queries, agents declare goals and the system returns contextualized, confidence-weighted data bundles. The system is event-sourced, ontology-driven, and probabilistic by design.

The prototype validates the core hypothesis: **agents using ANIDB make better business decisions than agents using SQL.**

---

## Repository Structure

```
anidb/
├── CLAUDE.md                        # This file
├── README.md
├── docker-compose.yml               # Full local stack
├── .env.example
│
├── core/                            # Rust — all services except API and experiments
│   ├── event-log/                   # Append-only event store
│   ├── confidence-store/            # Provenance & confidence metadata
│   ├── subscription-engine/         # Reactive condition engine
│   ├── semantic-engine/             # LLM-backed intent resolver & context bundler
│   │   ├── intent-parser/           # Parses agent goal declarations
│   │   ├── query-planner/           # Translates intent to storage ops
│   │   ├── write-resolver/          # Resolves semantic write declarations
│   │   └── context-bundler/         # Assembles decision-context bundles
│   ├── ontology/                    # Ontology lifecycle & validation
│   │   ├── primitives/              # Universal business primitives (REA model)
│   │   ├── archetypes/saas/         # SaaS archetype (first to build)
│   │   ├── compiler/                # Prose-to-ontology compiler
│   │   └── inference/               # Ontology inference from historical data
│   └── knowledge-graph/             # Neo4j interface & ontology sync
│       ├── schema/                  # Graph schema definitions
│       ├── queries/                 # Cypher query templates
│       └── sync/                    # Event log → graph sync
│
├── api/                             # TypeScript — Agent-facing API gateway
│   ├── routes/
│   │   ├── intent-read/             # Goal-based read endpoint
│   │   ├── intent-write/            # Semantic write endpoint
│   │   └── subscriptions/           # Subscription registration & delivery
│   ├── sdk/                         # Agent SDK (TypeScript)
│   └── middleware/
│
├── experiments/                     # Python — Research validation only
│   ├── synthetic-data/              # SaaS business simulation generator
│   ├── baseline-agent/              # SQL-based comparison agent
│   ├── anidb-agent/                 # ANIDB-powered test agent
│   ├── evaluation/                  # Scoring rubrics & metrics
│   └── notebooks/                   # Jupyter analysis notebooks
│
└── infra/                           # Infrastructure config
    ├── neo4j/                       # Knowledge graph config
    ├── eventstore/                  # EventStoreDB config
    ├── duckdb/                      # Columnar analytics config
    └── redis/                       # Subscription engine backing store
```

---

## Language Decisions & Rationale

| Component | Language | Why |
|---|---|---|
| Event Log & Confidence Store | **Rust** | Append-only writes at high throughput require zero-GC performance. This is the core source of truth — correctness and speed are non-negotiable. |
| Subscription Engine | **Rust** | Condition evaluation runs on every event write; latency must be sub-millisecond. |
| Semantic Engine | **Rust** | Intent parsing, query planning, and context bundling are async I/O orchestration — HTTP calls to the Anthropic API, parallel queries across storage layers, JSON assembly. Rust's async runtime (Tokio) handles this excellently. The Anthropic SDK has a first-class Rust client. Shared types with the event log eliminate integration bugs. |
| Ontology Service | **Rust** | Ontology operations are JSON manipulation and LLM API calls — both well-supported in Rust. Shared type definitions with the event log and a single toolchain across all services. |
| Knowledge Graph Interface | **Rust** | Neo4j has a solid Rust driver. Graph queries are async HTTP. Keeping this in Rust means one language across the entire service layer with no GIL constraints. |
| API Gateway | **TypeScript** | The agent-facing surface needs strong typing for the SDK. TypeScript gives clean interfaces, good async handling, and easy npm distribution of the SDK. |
| Experiments & Evaluation | **Python** | The only Python in the stack. Jupyter notebooks, pandas, matplotlib, and scipy for statistical analysis have no credible Rust equivalent for research work. Data generation and experiment scoring belong here. |

---

## Services & Ports (Local Development)

| Service | Port | Notes |
|---|---|---|
| API Gateway | 3000 | Main agent entry point |
| Semantic Engine | 8001 | Internal; called by API gateway |
| Ontology Service | 8002 | Internal; manages ontology lifecycle |
| Knowledge Graph (Neo4j) | 7474 / 7687 | Browser UI at 7474 |
| EventStoreDB | 2113 / 1113 | Admin UI at 2113 |
| DuckDB | — | In-process; no separate port |
| Redis | 6379 | Subscription engine backing store |

---

## Getting Started

### Prerequisites
- Docker & Docker Compose
- Rust (via rustup, stable toolchain)
- Python 3.11+
- Node.js 20+
- An Anthropic API key (for the semantic engine)

### First-time setup
```bash
cp .env.example .env
# Add your ANTHROPIC_API_KEY to .env

docker-compose up -d          # Start infrastructure (Neo4j, EventStoreDB, Redis)
cd core && cargo build        # Build all Rust services
cd ../api && npm install      # Install API gateway dependencies
cd ../experiments && pip install -e ".[dev]"  # Install experiment dependencies
```

### Run the full stack
```bash
docker-compose up             # All services + infrastructure
```

### Run experiments
```bash
cd experiments
pip install -e ".[dev]"
python synthetic-data/generate.py --customers 500 --days 90
python anidb-agent/run.py --scenario churn
python baseline-agent/run.py --scenario churn
python evaluation/compare.py --output results/
```

---

## Core Concepts to Understand Before Coding

**Event Sourcing:** The database never mutates. Every state change is a new event appended to the log. Current state is always derived by replaying events. Do not add UPDATE or DELETE operations anywhere in the event log service.

**Intent Queries:** Agents never specify data shape. They describe a decision they need to make. The semantic engine is responsible for translating that intent into the right combination of graph queries, event log queries, and columnar analytics — and bundling the result with confidence scores.

**Ontology as Code:** The ontology is not a static schema. It is a living model of the business that evolves. Every entity type, relationship, and causal belief is stored in the knowledge graph and versioned. Do not hardcode business logic anywhere outside the ontology layer.

**Confidence First:** Every fact returned to an agent must carry a confidence score and provenance metadata. There is no such thing as a bare value in ANIDB's output. The confidence store is not optional.

---

## Prototype Scope (Phase 1 PoC)

The prototype is scoped to prove the core hypothesis. Build only what is needed for the experiments. Scope limits:

- **One archetype only:** SaaS
- **Three decision classes:** churn intervention, pricing, capacity/inventory
- **One business simulation:** 500 customers, 90 days, ~10,000 events
- **No multi-tenancy:** single business ontology
- **No production hardening:** this is a research prototype

Do not scope-creep into multi-tenancy, auth, or production infrastructure. The goal is a working comparison, not a shippable product.

---

## Development Rules

- **Never mutate the event log.** Append only. Any code that issues an UPDATE or DELETE against EventStoreDB is wrong.
- **All agent-facing responses must include confidence scores.** If a value doesn't have a confidence score, it is not ready to return.
- **Ontology changes go through the ontology service.** Do not hardcode entity types or relationships anywhere else.
- **The SQL baseline agent must be implemented fairly.** It gets full schema access, optimized prompting, and no artificial limitations. A weak baseline invalidates the research.
- **Experiments are immutable once started.** Do not change the evaluation rubric or synthetic dataset after experiments begin. Define both fully before running.
- **All LLM calls go through the semantic engine.** Do not add direct LLM calls in other services.

---

## Environment Variables

```
ANTHROPIC_API_KEY=            # Required — used by semantic engine
NEO4J_URI=bolt://localhost:7687
NEO4J_USER=neo4j
NEO4J_PASSWORD=
EVENTSTORE_URI=esdb://localhost:2113?tls=false
REDIS_URI=redis://localhost:6379
DUCKDB_PATH=./data/analytics.duckdb
LOG_LEVEL=info                # debug | info | warn | error
ONTOLOGY_VERSION=1            # Increment on breaking ontology changes
```

---

## Key Files to Read First

Before working on any component, read these files in order:

1. `core/ontology/primitives/rea_model.rs` — the universal business ontology foundation
2. `core/ontology/archetypes/saas/archetype.rs` — the SaaS domain model
3. `core/event-log/src/schema.rs` — the event schema
4. `core/semantic-engine/intent-parser/parser.rs` — how intent queries are parsed
5. `api/routes/intent-read/handler.ts` — the main agent-facing read endpoint

---

## Testing

```bash
# Rust (all services)
cd core && cargo test

# TypeScript (API + SDK)
cd api && npm test

# Python (experiments only)
cd experiments && pytest
```

Integration tests require the full Docker stack running. Unit tests can run without it.

---

## Questions & Open Problems

See `OPEN_QUESTIONS.md` for the list of unresolved design decisions. If you encounter a situation not covered by this file, add it there rather than making an assumption.
