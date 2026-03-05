"""
SQL Baseline Agent

A well-implemented SQL agent using GPT-5.1 with full schema access.
Operates against a PostgreSQL mirror of the event data.
Gets every advantage: full schema docs, example queries, up to 10 tool-use turns.

Usage:
    python baseline-agent/run.py [--decision-class churn|pricing|capacity|all]
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

_PG_DSN_DEFAULT = (
    "postgresql://"
    f"{os.environ.get('POSTGRES_USER', 'anidb')}:"
    f"{os.environ.get('POSTGRES_PASSWORD', 'anidb_dev')}@"
    f"{os.environ.get('POSTGRES_HOST', 'localhost')}:"
    f"{os.environ.get('POSTGRES_PORT', '5432')}/"
    f"{os.environ.get('POSTGRES_DB', 'anidb')}"
)

import psycopg2
from openai import OpenAI

SCHEMA_DESCRIPTION = """
PostgreSQL schema (schema name: baseline). Simulation period: 2025-01-01 to 2025-04-01 (90 days).

baseline.customers
  customer_id   UUID PRIMARY KEY
  archetype     VARCHAR(50)   -- 'healthy', 'at_risk', 'price_sensitive', 'capacity_bound'
  plan          VARCHAR(50)   -- 'basic' ($15/mo, 5 seats), 'pro' ($40/mo, 20 seats), 'enterprise' ($175/mo, 100 seats)
  mrr           DECIMAL(10,2) -- monthly recurring revenue
  seats_used    INT
  seat_limit    INT
  subscribed_at TIMESTAMPTZ
  churned       BOOLEAN
  churn_date    TIMESTAMPTZ   -- NULL if not churned

baseline.events
  event_id      UUID PRIMARY KEY
  customer_id   UUID
  event_type    VARCHAR(100)  -- CustomerSubscribed, CustomerCancelled, PlanChanged, PriceChanged,
                              --   LoginEvent, SupportTicketOpened, SupportTicketClosed,
                              --   InvoicePaid, InvoiceFailed, FeatureUsage, SeatCountChanged,
                              --   CapacityThresholdReached
  payload       JSONB
  occurred_at   TIMESTAMPTZ

baseline.daily_logins
  id            SERIAL PRIMARY KEY
  customer_id   UUID
  login_date    DATE
  login_count   INT           -- logins in that calendar day

baseline.invoices
  invoice_id    UUID PRIMARY KEY
  customer_id   UUID
  amount        DECIMAL(10,2)
  status        VARCHAR(20)   -- 'paid' or 'failed'
  due_date      DATE
  paid_at       TIMESTAMPTZ   -- NULL if failed

baseline.support_tickets
  ticket_id     UUID PRIMARY KEY
  customer_id   UUID
  opened_at     TIMESTAMPTZ
  closed_at     TIMESTAMPTZ   -- NULL if still open
  status        VARCHAR(20)   -- 'open' or 'closed'

Key date ranges:
  Simulation start:  2025-01-01
  Simulation end:    2025-04-01
  Final 14 days:     2025-03-18 to 2025-04-01  (where churn occurs)
  Final 30 days:     2025-03-02 to 2025-04-01
"""

DECISION_PROMPTS = {
    "churn": (
        "You are a business analyst with full access to a SaaS company's customer database. "
        "Your task: identify the top 10 customers most at risk of churning in the next 30 days. "
        "Look for: declining login frequency (compare recent 30 days vs earlier), "
        "failed invoices, high support ticket volume, or low engagement. "
        "Use multiple queries to build your analysis. "
        "Return a JSON object (no markdown, no code block) with key 'flagged_customer_ids' "
        "containing a list of up to 10 customer UUIDs ordered by churn risk (highest risk first). "
        'Example: {"flagged_customer_ids": ["uuid-1", "uuid-2"]}'
    ),
    "pricing": (
        "You are a pricing analyst with full access to a SaaS company's customer database. "
        "Current prices: basic=$15/mo, pro=$40/mo, enterprise=$175/mo. "
        "Your task: recommend optimal prices for each tier based on customer behavior. "
        "Consider: price change events, churn rates per plan, revenue concentration, "
        "and customer price sensitivity signals. "
        "Return a JSON object (no markdown, no code block) with key 'recommendations' "
        "mapping tier name to recommended monthly price as a number. "
        'Example: {"recommendations": {"basic": 18.0, "pro": 45.0, "enterprise": 199.0}}'
    ),
    "capacity": (
        "You are a capacity analyst with full access to a SaaS company's customer database. "
        "Your task: identify customers who are approaching their seat capacity limits and "
        "will likely need a plan upgrade within the next 30 days. "
        "Look for: seats_used near seat_limit, recent seat growth via FeatureUsed events. "
        "Return a JSON object (no markdown, no code block) with key 'flagged_customer_ids' "
        "containing a list of customer UUIDs ordered by urgency. "
        'Example: {"flagged_customer_ids": ["uuid-3", "uuid-4"]}'
    ),
}

TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "run_query",
            "description": "Execute a SQL SELECT query against the baseline schema. Returns up to 100 rows as JSON.",
            "parameters": {
                "type": "object",
                "properties": {
                    "sql": {
                        "type": "string",
                        "description": "SQL SELECT statement to execute (SELECT only; no INSERT/UPDATE/DELETE).",
                    }
                },
                "required": ["sql"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "get_schema",
            "description": "Get full database schema documentation.",
            "parameters": {"type": "object", "properties": {}},
        },
    },
]


def _run_query(sql: str, conn) -> str:
    if not sql.strip().upper().startswith("SELECT"):
        return "Error: Only SELECT queries are allowed."
    try:
        with conn.cursor() as cur:
            cur.execute(sql)
            if not cur.description:
                return "[]"
            cols = [d[0] for d in cur.description]
            rows = [dict(zip(cols, row)) for row in cur.fetchmany(100)]
            return json.dumps(rows, default=str)
    except Exception as e:
        return f"Query error: {e}"


def _extract_json(text: str) -> dict:
    """Extract a JSON object from LLM response text."""
    text = text.strip()
    # Direct parse
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass
    # Find first {...} block
    match = re.search(r"\{[^{}]*(?:\{[^{}]*\}[^{}]*)?\}", text, re.DOTALL)
    if match:
        try:
            return json.loads(match.group())
        except json.JSONDecodeError:
            pass
    return {"raw_response": text}


def run_decision(decision_class: str, conn, client: OpenAI) -> dict:
    """Run one decision class with GPT-5.1 tool-use loop (max 10 turns)."""
    messages = [
        {
            "role": "system",
            "content": (
                "You are an expert data analyst. Use SQL tools to query the database "
                "and make data-driven decisions. After your analysis, return your final answer "
                "as a plain JSON object (no markdown code blocks).\n\n"
                + SCHEMA_DESCRIPTION
            ),
        },
        {"role": "user", "content": DECISION_PROMPTS[decision_class]},
    ]

    for _turn in range(10):
        response = client.chat.completions.create(
            model="gpt-5.1",
            messages=messages,
            tools=TOOLS,
            tool_choice="auto",
        )
        msg = response.choices[0].message
        messages.append(msg.model_dump(exclude_none=True))

        if not msg.tool_calls:
            return _extract_json(msg.content or "{}")

        tool_results = []
        for tc in msg.tool_calls:
            fn_name = tc.function.name
            fn_args = json.loads(tc.function.arguments or "{}")

            if fn_name == "get_schema":
                result = SCHEMA_DESCRIPTION
            elif fn_name == "run_query":
                result = _run_query(fn_args.get("sql", ""), conn)
            else:
                result = f"Unknown tool: {fn_name}"

            tool_results.append({
                "role": "tool",
                "tool_call_id": tc.id,
                "content": result,
            })
        messages.extend(tool_results)

    return {"error": "max_turns_exceeded"}


def main() -> None:
    parser = argparse.ArgumentParser(description="Run SQL baseline agent")
    parser.add_argument(
        "--decision-class",
        choices=["churn", "pricing", "capacity", "all"],
        default="all",
    )
    parser.add_argument("--pg-dsn", default=_PG_DSN_DEFAULT)
    parser.add_argument("--output-dir", default="results")
    args = parser.parse_args()

    api_key = os.getenv("OPENAI_API_KEY")
    if not api_key:
        print("ERROR: OPENAI_API_KEY environment variable not set", file=sys.stderr)
        sys.exit(1)

    try:
        conn = psycopg2.connect(args.pg_dsn)
    except Exception as e:
        print(f"ERROR: Could not connect to PostgreSQL: {e}", file=sys.stderr)
        sys.exit(1)

    client = OpenAI(api_key=api_key)
    classes = (
        ["churn", "pricing", "capacity"] if args.decision_class == "all" else [args.decision_class]
    )

    decisions: dict = {
        "agent": "baseline",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "decisions": {},
    }

    for dc in classes:
        print(f"Running {dc} decision (baseline SQL agent)...")
        result = run_decision(dc, conn, client)
        decisions["decisions"][dc] = result
        print(f"  {dc}: {result}")

    conn.close()

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "baseline_decisions.json"
    with open(output_path, "w") as f:
        json.dump(decisions, f, indent=2, default=str)
    print(f"Results written to {output_path}")


if __name__ == "__main__":
    main()
