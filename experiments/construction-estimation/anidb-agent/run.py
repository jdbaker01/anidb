"""
Construction Estimation ANIDB-Powered Agent

Queries the ANIDB confidence store directly for Project and Portfolio facts.
The semantic engine's query planner is SaaS-specific (Customer entities only);
for the construction domain the confidence store IS the ANIDB data layer —
that's the layer the comparison tests.

Per the Phase 5b plan: "The comparison is between PostgreSQL (baseline) and
confidence store (ANIDB), which is what matters."

Usage:
    python anidb-agent/run.py [--decision-class overrun|delay|resource|bid|all]
"""
from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import httpx

CONFIDENCE_STORE_URL = "http://localhost:8003"

# Thresholds derived from the fact schema
OVERRUN_RISK_THRESHOLD = 0.30      # overrun_risk_score > this → flag
DELAY_RISK_THRESHOLD   = 0.30      # delay_risk_score > this → flag
RESOURCE_UTIL_THRESHOLD= 0.85      # resource_utilization >= this → flag

PORTFOLIO_ENTITY_ID = "00000000-0000-0000-0000-000000000002"


def get_project_facts(base_url: str) -> list[dict]:
    """Fetch all Project facts from the confidence store."""
    with httpx.Client(timeout=60.0) as client:
        resp = client.get(f"{base_url}/facts/type/Project")
        resp.raise_for_status()
        return resp.json().get("facts", [])


def get_portfolio_facts(base_url: str) -> list[dict]:
    """Fetch Portfolio facts by the well-known portfolio entity ID."""
    with httpx.Client(timeout=60.0) as client:
        resp = client.get(f"{base_url}/facts/{PORTFOLIO_ENTITY_ID}/all")
        if resp.status_code == 404:
            # Try the type endpoint for Portfolio
            resp2 = client.get(f"{base_url}/facts/type/Portfolio")
            if resp2.status_code == 200:
                return resp2.json().get("facts", [])
            return []
        resp.raise_for_status()
        return resp.json().get("facts", [])


def parse_overrun_decision(facts: list[dict]) -> dict:
    """Flag projects with high overrun_risk_score or cost_variance_trend."""
    flagged: list[str] = []

    for fact in facts:
        key = fact.get("fact_key", "")
        value = fact.get("fact_value", {})
        confidence = fact.get("confidence", {})
        conf_val = confidence.get("value", 0.0) if isinstance(confidence, dict) else 0.0

        if key == "overrun_risk_score":
            risk = value.get("value", 0.0)
            pid = value.get("project_id")
            if pid and isinstance(risk, (int, float)) and risk > OVERRUN_RISK_THRESHOLD:
                flagged.append((pid, risk, conf_val))

        elif key == "cost_variance_trend":
            pct_over = value.get("pct_over_budget", 0.0)
            co_count = value.get("change_order_count", 0)
            pid = value.get("project_id")
            # Flag if already over budget by >5% or has change orders
            if pid and (pct_over > 0.05 or co_count > 0):
                score = pct_over * 2 + co_count * 0.1
                # Use as secondary signal — only add if not already in flagged
                flagged.append((pid, score, conf_val))

    # Deduplicate: keep highest score per project
    best: dict[str, tuple[float, float]] = {}
    for pid, score, conf in flagged:
        if pid not in best or score > best[pid][0]:
            best[pid] = (score, conf)

    # Sort by score descending
    ordered = sorted(best.keys(), key=lambda p: best[p][0], reverse=True)

    n_project_facts = len(facts)
    return {
        "flagged_project_ids": ordered,
        "confidence_store_facts_used": n_project_facts,
    }


def parse_delay_decision(facts: list[dict]) -> dict:
    """Flag projects with high delay_risk_score or poor schedule_adherence."""
    flagged: list[tuple[str, float, float]] = []

    for fact in facts:
        key = fact.get("fact_key", "")
        value = fact.get("fact_value", {})
        confidence = fact.get("confidence", {})
        conf_val = confidence.get("value", 0.0) if isinstance(confidence, dict) else 0.0

        if key == "delay_risk_score":
            risk = value.get("value", 0.0)
            pid = value.get("project_id")
            if pid and isinstance(risk, (int, float)) and risk > DELAY_RISK_THRESHOLD:
                flagged.append((pid, risk, conf_val))

        elif key == "schedule_adherence":
            sched_val = value.get("value", 1.0)
            pct_complete = value.get("pct_complete", 0.0)
            expected_pct = value.get("expected_pct_complete", 0.0)
            pid = value.get("project_id")
            # Flag if schedule_adherence < 0.85 (lagging significantly)
            if pid and isinstance(sched_val, (int, float)) and sched_val < 0.85:
                lag_score = 1.0 - sched_val
                flagged.append((pid, lag_score, conf_val))

    # Deduplicate
    best: dict[str, tuple[float, float]] = {}
    for pid, score, conf in flagged:
        if pid not in best or score > best[pid][0]:
            best[pid] = (score, conf)

    ordered = sorted(best.keys(), key=lambda p: best[p][0], reverse=True)
    return {
        "flagged_project_ids": ordered,
        "confidence_store_facts_used": len(facts),
    }


def parse_resource_decision(facts: list[dict]) -> dict:
    """Flag projects at or near labor capacity from resource_utilization facts."""
    flagged: list[tuple[str, float, float]] = []

    for fact in facts:
        key = fact.get("fact_key", "")
        value = fact.get("fact_value", {})
        confidence = fact.get("confidence", {})
        conf_val = confidence.get("value", 0.0) if isinstance(confidence, dict) else 0.0

        if key == "resource_utilization":
            util = value.get("value", 0.0)
            pid = value.get("project_id")
            labor_count = value.get("labor_count", 0)
            labor_capacity = value.get("labor_capacity", 1)
            if pid and isinstance(util, (int, float)) and util >= RESOURCE_UTIL_THRESHOLD:
                flagged.append((pid, util, conf_val))

    # Deduplicate
    best: dict[str, tuple[float, float]] = {}
    for pid, score, conf in flagged:
        if pid not in best or score > best[pid][0]:
            best[pid] = (score, conf)

    ordered = sorted(best.keys(), key=lambda p: best[p][0], reverse=True)
    return {
        "flagged_project_ids": ordered,
        "confidence_store_facts_used": len(facts),
    }


def parse_bid_decision(portfolio_facts: list[dict], project_facts: list[dict]) -> dict:
    """Extract optimal markups from portfolio_bid_analysis fact."""
    recommendations: dict[str, float] = {}
    project_types = {"residential", "commercial", "infrastructure", "renovation"}

    # Primary: portfolio_bid_analysis fact
    for fact in portfolio_facts:
        key = fact.get("fact_key", "")
        value = fact.get("fact_value", {})
        if key == "portfolio_bid_analysis" and isinstance(value, dict):
            for pt in project_types:
                pt_data = value.get(pt)
                if isinstance(pt_data, dict):
                    markup = pt_data.get("optimal_markup")
                    if markup is not None:
                        try:
                            recommendations[pt] = float(markup)
                        except (TypeError, ValueError):
                            pass

    # Fallback: also check project_facts for portfolio entity
    if len(recommendations) < 4:
        for fact in project_facts:
            key = fact.get("fact_key", "")
            eid = fact.get("entity_id", "")
            if key == "portfolio_bid_analysis" and eid == PORTFOLIO_ENTITY_ID:
                value = fact.get("fact_value", {})
                for pt in project_types:
                    if pt not in recommendations:
                        pt_data = value.get(pt, {})
                        markup = pt_data.get("optimal_markup") if isinstance(pt_data, dict) else None
                        if markup is not None:
                            try:
                                recommendations[pt] = float(markup)
                            except (TypeError, ValueError):
                                pass

    return {
        "recommendations": recommendations,
        "confidence_store_facts_used": len(portfolio_facts) + len(project_facts),
    }


def run_decision(decision_class: str, base_url: str) -> dict:
    project_facts = get_project_facts(base_url)
    print(f"  Confidence store: {len(project_facts)} Project facts")

    if decision_class == "overrun":
        return parse_overrun_decision(project_facts)
    elif decision_class == "delay":
        return parse_delay_decision(project_facts)
    elif decision_class == "resource":
        return parse_resource_decision(project_facts)
    elif decision_class == "bid":
        portfolio_facts = get_portfolio_facts(base_url)
        print(f"  Confidence store: {len(portfolio_facts)} Portfolio facts")
        return parse_bid_decision(portfolio_facts, project_facts)
    else:
        raise ValueError(f"Unknown decision class: {decision_class}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Run construction estimation ANIDB-powered agent")
    parser.add_argument(
        "--decision-class",
        choices=["overrun", "delay", "resource", "bid", "all"],
        default="all",
    )
    parser.add_argument("--confidence-store-url", default=CONFIDENCE_STORE_URL)
    parser.add_argument("--output-dir", default="results")
    args = parser.parse_args()

    classes = (
        ["overrun", "delay", "resource", "bid"]
        if args.decision_class == "all"
        else [args.decision_class]
    )

    decisions: dict = {
        "agent": "anidb",
        "experiment": "construction-estimation",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "decisions": {},
    }

    for dc in classes:
        print(f"Running {dc} decision (construction ANIDB agent)...")
        try:
            result = run_decision(dc, args.confidence_store_url)
            decisions["decisions"][dc] = result
            print(f"  {dc}: flagged {len(result.get('flagged_project_ids', result.get('recommendations', {})))}")
        except httpx.ConnectError:
            print(f"  ERROR: Cannot connect to confidence store at {args.confidence_store_url}")
            decisions["decisions"][dc] = {"error": "service_unavailable"}
        except httpx.HTTPStatusError as e:
            print(f"  ERROR: HTTP {e.response.status_code}: {e.response.text}")
            decisions["decisions"][dc] = {"error": f"http_{e.response.status_code}"}
        except Exception as e:
            print(f"  ERROR: {e}")
            decisions["decisions"][dc] = {"error": str(e)}

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "construction_anidb_decisions.json"
    with open(output_path, "w") as f:
        json.dump(decisions, f, indent=2, default=str)
    print(f"Results written to {output_path}")


if __name__ == "__main__":
    main()
