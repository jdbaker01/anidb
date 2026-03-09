# Contributing to ANIDB

Thank you for your interest in ANIDB! This document explains how to contribute.

## Getting Started

1. Fork the repository
2. Clone your fork and set up the development environment:

```bash
cp .env.example .env
# Add your ANTHROPIC_API_KEY to .env

docker compose up -d
cd core && cargo build && cd ..
cd api && npm install && cd ..
cd experiments && python3 -m venv .venv && source .venv/bin/activate && pip install -e ".[dev]" && cd ..
```

3. Create a branch for your change:

```bash
git checkout -b your-branch-name
```

## Development

ANIDB uses three languages — see [CLAUDE.md](CLAUDE.md) for full architecture details.

| Component | Language | Build | Test |
|-----------|----------|-------|------|
| Core services | Rust | `cd core && cargo build` | `cd core && cargo test` |
| API gateway | TypeScript | `cd api && npm install` | `cd api && npm test` |
| Experiments | Python | `pip install -e ".[dev]"` | `cd experiments && pytest` |

### Running the Stack

Infrastructure services (Neo4j, EventStoreDB, PostgreSQL, Redis) run via Docker Compose:

```bash
docker compose up -d
```

## Submitting Changes

### Pull Requests

1. Keep PRs focused — one logical change per PR
2. Ensure all tests pass before submitting
3. Update documentation if your change affects public interfaces or setup
4. Write a clear PR description explaining **what** changed and **why**

### Commit Messages

Write clear, descriptive commit messages. Use the imperative mood:

- "Add churn prediction to evaluation rubric"
- "Fix confidence score calculation for derived facts"
- "Update Neo4j schema for subscription events"

### Code Style

- **Rust:** Follow standard `rustfmt` formatting. Run `cargo fmt` before committing.
- **TypeScript:** Follow the existing project conventions. Run `npm run lint` if available.
- **Python:** Follow PEP 8. Use type hints where practical.

## Key Design Principles

Before contributing, understand these constraints (detailed in [CLAUDE.md](CLAUDE.md)):

- **Event log is append-only.** Never issue UPDATE or DELETE against the event store.
- **All responses include confidence scores.** No bare values in agent-facing output.
- **Ontology changes go through the ontology service.** Don't hardcode business logic elsewhere.
- **LLM calls go through the semantic engine.** Don't add direct LLM calls in other services.

## Reporting Bugs

Open a [GitHub Issue](https://github.com/BizJetAI/anidb/issues) with:

- Steps to reproduce
- Expected vs actual behavior
- Environment details (OS, Rust/Node/Python versions)

## Suggesting Features

Open a [GitHub Issue](https://github.com/BizJetAI/anidb/issues) with the "enhancement" label. Describe the use case and why it matters.

## Security Vulnerabilities

Do **not** open a public issue for security vulnerabilities. See [SECURITY.md](SECURITY.md) for responsible disclosure instructions.

## License

By contributing, you agree that your contributions will be licensed under the [Apache License 2.0](LICENSE).
