"""
Construction Estimation SQL Baseline Agent

A well-implemented SQL agent using GPT-5.1 with full schema access.
Operates against the construction.* PostgreSQL schema.
Gets every advantage: full schema docs, example queries, up to 10 tool-use turns.

Usage:
    python baseline-agent/run.py [--decision-class overrun|delay|resource|bid|all]
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
PostgreSQL schema (schema name: construction). Simulation period: 2024-01-01 to 2024-12-31 (365 days).

construction.projects
  project_id       UUID PRIMARY KEY
  archetype        VARCHAR(50)   -- 'on_track', 'over_budget', 'delayed', 'resource_constrained'
  project_type     VARCHAR(50)   -- 'residential', 'commercial', 'infrastructure', 'renovation'
  estimated_cost   DECIMAL(14,2) -- original budget
  actual_cost      DECIMAL(14,2) -- running total including change orders
  start_date       DATE
  scheduled_end    DATE
  actual_end       DATE          -- NULL if still in progress
  labor_count      INT           -- current headcount
  labor_capacity   INT           -- maximum headcount
  over_budget      BOOLEAN
  delayed          BOOLEAN

construction.change_orders
  order_id         UUID PRIMARY KEY
  project_id       UUID
  amount           DECIMAL(14,2)
  reason           VARCHAR(100)  -- 'scope_change', 'material_cost', 'labor_cost', 'rework'
  ordered_at       TIMESTAMPTZ

construction.weekly_reports
  report_id        SERIAL PRIMARY KEY
  project_id       UUID
  report_date      DATE
  cost_to_date     DECIMAL(14,2) -- cumulative cost as of this report
  pct_complete     DECIMAL(5,2)  -- 0.00 to 100.00
  labor_hours      INT

construction.bids
  bid_id           UUID PRIMARY KEY
  project_id       UUID
  project_type     VARCHAR(50)
  bid_amount       DECIMAL(14,2)
  estimated_cost   DECIMAL(14,2)
  won              BOOLEAN
  markup_pct       DECIMAL(6,4)  -- bid_amount/estimated_cost - 1
  submitted_at     TIMESTAMPTZ

Key facts:
  Simulation start:  2024-01-01
  Simulation end:    2024-12-31
  Over-budget threshold: actual_cost > estimated_cost * 1.10
  Delay threshold: actual_end > scheduled_end + 14 days, OR actual_end IS NULL and today > scheduled_end
  Resource constraint threshold: labor_count >= labor_capacity * 0.85
"""

DECISION_PROMPTS = {
    "overrun": (
        "You are a construction project analyst with full access to a construction company's project database. "
        "Your task: identify construction projects at risk of exceeding their budget by more than 10 percent. "
        "Look for: rising cost_to_date relative to pct_complete in weekly_reports, "
        "accumulating change orders, or actual_cost already exceeding estimated_cost * 1.10. "
        "Use multiple queries to build your analysis. "
        "Return a JSON object (no markdown, no code block) with key 'flagged_project_ids' "
        "containing a list of project UUIDs ordered by overrun risk (highest risk first). "
        'Example: {"flagged_project_ids": ["uuid-1", "uuid-2"]}'
    ),
    "delay": (
        "You are a construction schedule analyst with full access to a construction company's project database. "
        "Your task: identify construction projects at risk of missing their scheduled completion date. "
        "Look for: pct_complete lagging behind expected progress (days_elapsed / total_duration), "
        "projects where actual_end > scheduled_end + 14 days, or projects still running past their scheduled_end. "
        "Use multiple queries to build your analysis. "
        "Return a JSON object (no markdown, no code block) with key 'flagged_project_ids' "
        "containing a list of project UUIDs ordered by delay risk (highest risk first). "
        'Example: {"flagged_project_ids": ["uuid-3", "uuid-4"]}'
    ),
    "resource": (
        "You are a resource planning analyst with full access to a construction company's project database. "
        "Your task: identify construction projects approaching labor capacity limits. "
        "Look for: projects where labor_count >= labor_capacity * 0.85, "
        "or where labor_hours trend in weekly_reports suggests rapid growth. "
        "Return a JSON object (no markdown, no code block) with key 'flagged_project_ids' "
        "containing a list of project UUIDs ordered by urgency (most constrained first). "
        'Example: {"flagged_project_ids": ["uuid-5", "uuid-6"]}'
    ),
    "bid": (
        "You are a bid strategy analyst with full access to a construction company's project database. "
        "Your task: recommend optimal bid markup percentages by construction project type. "
        "Current project types: residential, commercial, infrastructure, renovation. "
        "Analyze the bids table: compute average markup_pct and win rates by project_type. "
        "Consider that higher markups reduce win rate; find the markup that maximizes expected margin. "
        "Return a JSON object (no markdown, no code block) with key 'recommendations' "
        "mapping project_type to recommended markup as a decimal (e.g. 0.18 = 18%). "
        'Example: {"recommendations": {"residential": 0.18, "commercial": 0.22, '
        '"infrastructure": 0.28, "renovation": 0.15}}'
    ),
}

TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "run_query",
            "description": "Execute a SQL SELECT query against the construction schema. Returns up to 200 rows as JSON.",
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
            rows = [dict(zip(cols, row)) for row in cur.fetchmany(200)]
            return json.dumps(rows, default=str)
    except Exception as e:
        return f"Query error: {e}"


def _extract_json(text: str) -> dict:
    """Extract a JSON object from LLM response text."""
    text = text.strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass
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
                "You are an expert construction project analyst. Use SQL tools to query the database "
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
    parser = argparse.ArgumentParser(description="Run construction estimation SQL baseline agent")
    parser.add_argument(
        "--decision-class",
        choices=["overrun", "delay", "resource", "bid", "all"],
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
        ["overrun", "delay", "resource", "bid"]
        if args.decision_class == "all"
        else [args.decision_class]
    )

    decisions: dict = {
        "agent": "baseline",
        "experiment": "construction-estimation",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "decisions": {},
    }

    for dc in classes:
        print(f"Running {dc} decision (construction baseline SQL agent)...")
        result = run_decision(dc, conn, client)
        decisions["decisions"][dc] = result
        print(f"  {dc}: {result}")

    conn.close()

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "construction_baseline_decisions.json"
    with open(output_path, "w") as f:
        json.dump(decisions, f, indent=2, default=str)
    print(f"Results written to {output_path}")


if __name__ == "__main__":
    main()
