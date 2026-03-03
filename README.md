# ANIDB — Agent-Native Intelligent Database

An intent-semantic data layer for AI agents to operate businesses. Instead of SQL queries, agents declare goals and the system returns contextualized, confidence-weighted data bundles.

## Architecture

```
                          ┌─────────────────┐
                          │   Agent (SDK)    │
                          └────────┬────────┘
                                   │
                          ┌────────▼────────┐
                          │  API Gateway    │  :3000  (TypeScript)
                          └────────┬────────┘
                                   │
                   ┌───────────────▼───────────────┐
                   │       Semantic Engine          │  :8001  (Rust)
                   │  ┌──────────┐ ┌────────────┐  │
                   │  │  Intent  │ │   Query    │  │
                   │  │  Parser  │ │  Planner   │  │
                   │  └──────────┘ └────────────┘  │
                   │  ┌──────────┐ ┌────────────┐  │
                   │  │  Write   │ │  Context   │  │
                   │  │ Resolver │ │  Bundler   │  │
                   │  └──────────┘ └────────────┘  │
                   └──┬──────┬──────┬──────┬───────┘
                      │      │      │      │
         ┌────────────▼┐ ┌──▼───┐ ┌▼────┐ ┌▼──────────┐
         │  Ontology    │ │Event │ │Duck │ │Confidence  │
         │  Service     │ │ Log  │ │ DB  │ │  Store     │
         │  :8002       │ │      │ │     │ │            │
         └──────┬───────┘ └──┬───┘ └─────┘ └────┬──────┘
                │            │                    │
         ┌──────▼───────┐ ┌──▼──────────┐ ┌─────▼──────┐
         │    Neo4j     │ │ EventStore  │ │ PostgreSQL │
         │   :7687      │ │   DB :2113  │ │   :5432    │
         └──────────────┘ └─────────────┘ └────────────┘

         ┌──────────────────────────────────────────────┐
         │        Subscription Engine (Redis :6379)     │
         └──────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Docker & Docker Compose
- Rust (via rustup, stable toolchain)
- Node.js 20+
- Python 3.11+
- An Anthropic API key

### Setup

```bash
# 1. Configure environment
cp .env.example .env
# Add your ANTHROPIC_API_KEY to .env

# 2. Start infrastructure
docker compose up -d

# 3. Build Rust services
cd core && cargo build
cd ..

# 4. Set up API gateway
cd api && npm install
cd ..

# 5. Set up experiments
cd experiments
python3.13 -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
cd ..
```

### Run experiments

```bash
cd experiments
source .venv/bin/activate
python synthetic-data/generate.py --customers 500 --days 90
python anidb-agent/run.py --scenario churn
python baseline-agent/run.py --scenario churn
python evaluation/compare.py --output results/
```

## Services & Ports

| Service | Port | Language |
|---|---|---|
| API Gateway | 3000 | TypeScript |
| Semantic Engine | 8001 | Rust |
| Ontology Service | 8002 | Rust |
| Neo4j | 7474 / 7687 | — |
| EventStoreDB | 2113 / 1113 | — |
| PostgreSQL | 5432 | — |
| Redis | 6379 | — |
| DuckDB | in-process | — |

## Testing

```bash
# Rust
cd core && cargo test

# TypeScript
cd api && npm test

# Python
cd experiments && pytest
```

## License

Apache 2.0 — see [LICENSE](LICENSE).
