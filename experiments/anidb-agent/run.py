"""
ANIDB-Powered Agent

Uses the ANIDB semantic engine to make intent queries and receive
confidence-weighted context bundles for decision-making.
One call per decision class — the bundle contains the full context.

Usage:
    python anidb-agent/run.py [--decision-class churn|pricing|capacity|all]
"""
from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import httpx

SEMANTIC_ENGINE_URL = "http://localhost:8001"

INTENT_QUERIES = {
    "churn":    "identify customers at risk of churning in next 30 days",
    "pricing":  "recommend optimal price changes for each subscription tier",
    "capacity": "identify customers approaching seat capacity limits",
}


def call_intent_read(intent: str, base_url: str) -> dict:
    """POST to /intent-read and return the ContextBundle."""
    payload = {
        "intent": intent,
        "context": {
            "decision_class": None,
            "entity_refs": [],
            "time_horizon": None,
            "min_confidence": None,
        },
    }
    with httpx.Client(timeout=60.0) as client:
        resp = client.post(f"{base_url}/intent-read", json=payload)
        resp.raise_for_status()
        return resp.json()


def _extract_customer_ids(bundle: dict) -> list[str]:
    """Pull unique customer IDs out of a ContextBundle's facts and causal_context."""
    seen: dict[str, None] = {}

    def collect(value) -> None:
        if isinstance(value, str):
            # UUIDs are 36 chars with hyphens
            if len(value) == 36 and value.count("-") == 4:
                seen[value] = None
        elif isinstance(value, dict):
            for v in value.values():
                collect(v)
        elif isinstance(value, list):
            for item in value:
                collect(item)

    for fact in bundle.get("facts", []):
        collect(fact.get("value"))

    causal = bundle.get("causal_context", {})
    collect(causal)

    return list(seen.keys())


def _uuids_from_value(value) -> list[str]:
    """Recursively extract UUID-shaped strings from any JSON value."""
    found: list[str] = []
    if isinstance(value, str) and len(value) == 36 and value.count("-") == 4:
        found.append(value)
    elif isinstance(value, dict):
        for v in value.values():
            found.extend(_uuids_from_value(v))
    elif isinstance(value, list):
        for item in value:
            found.extend(_uuids_from_value(item))
    return found


def parse_churn_decision(bundle: dict) -> dict:
    """Extract at-risk customer IDs from a ContextBundle.

    Facts with key containing 'churn' or 'risk' are treated as churn signals.
    causal_context is a plain narrative string — we scan it for UUID patterns.
    """
    flagged: list[str] = []

    for fact in bundle.get("facts", []):
        key = fact.get("key", "").lower()
        value = fact.get("value")
        confidence = fact.get("confidence", {})
        score = confidence.get("score", 0.0) if isinstance(confidence, dict) else 0.0

        is_churn_fact = any(k in key for k in ("churn", "risk", "cancel", "at_risk"))
        if is_churn_fact or score > 0.5:
            flagged.extend(_uuids_from_value(value))

    # causal_context is a narrative string; scan for UUIDs as a best-effort
    causal_str = bundle.get("causal_context", "")
    if isinstance(causal_str, str):
        import re
        uuids_in_narrative = re.findall(
            r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", causal_str
        )
        flagged.extend(uuids_in_narrative)

    return {"flagged_customer_ids": list(dict.fromkeys(flagged))[:10]}


def parse_pricing_decision(bundle: dict) -> dict:
    """Extract pricing recommendations from a ContextBundle."""
    recommendations: dict[str, float] = {}
    tiers = {"basic", "pro", "enterprise"}

    for fact in bundle.get("facts", []):
        key = fact.get("key", "")
        value = fact.get("value")

        if not isinstance(value, dict):
            continue

        # Portfolio pricing analysis fact — primary source
        # key = "Customer.portfolio_pricing_analysis" (entity_type prefix added by confidence store)
        if "portfolio_pricing_analysis" in key:
            for tier in tiers:
                tier_data = value.get(tier)
                if isinstance(tier_data, dict) and "optimal_price" in tier_data:
                    try:
                        recommendations[tier] = float(tier_data["optimal_price"])
                    except (TypeError, ValueError):
                        pass

        key_lower = key.lower()

        # Direct tier → price mapping in value dict
        for tier in tiers:
            if tier in value:
                try:
                    recommendations[tier] = float(value[tier])
                except (TypeError, ValueError):
                    pass

        # Structured: {"tier": "basic", "recommended_price": 18.0}
        if "tier" in value and "recommended_price" in value:
            tier = str(value["tier"]).lower()
            if tier in tiers:
                try:
                    recommendations[tier] = float(value["recommended_price"])
                except (TypeError, ValueError):
                    pass

        # {"price": 18.0} where key contains tier name
        for tier in tiers:
            if tier in key_lower and "price" in value:
                try:
                    recommendations[tier] = float(value["price"])
                except (TypeError, ValueError):
                    pass

    return {"recommendations": recommendations}


def parse_capacity_decision(bundle: dict) -> dict:
    """Extract capacity-constrained customer IDs from a ContextBundle."""
    flagged: list[str] = []

    for fact in bundle.get("facts", []):
        key = fact.get("key", "").lower()
        value = fact.get("value")

        is_capacity_fact = any(k in key for k in ("seat", "capacity", "limit", "utiliz"))
        if not is_capacity_fact:
            continue

        if isinstance(value, dict):
            cid = value.get("customer_id") or value.get("id")
            seats_used = value.get("seats_used")
            seat_limit = value.get("seat_limit")
            if cid:
                if seats_used is not None and seat_limit is not None:
                    if int(seats_used) >= int(seat_limit) - 2:
                        flagged.append(str(cid))
                else:
                    flagged.append(str(cid))
        else:
            flagged.extend(_uuids_from_value(value))

    # Scan causal narrative for UUIDs as fallback
    causal_str = bundle.get("causal_context", "")
    if isinstance(causal_str, str) and "capacity" in causal_str.lower():
        import re
        uuids_in_narrative = re.findall(
            r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", causal_str
        )
        flagged.extend(uuids_in_narrative)

    return {"flagged_customer_ids": list(dict.fromkeys(flagged))}


PARSERS = {
    "churn":    parse_churn_decision,
    "pricing":  parse_pricing_decision,
    "capacity": parse_capacity_decision,
}


def run_decision(decision_class: str, base_url: str) -> dict:
    intent = INTENT_QUERIES[decision_class]
    print(f"  Intent: {intent!r}")

    bundle = call_intent_read(intent, base_url)
    n_facts = len(bundle.get("facts", []))
    confidence = bundle.get("confidence", "N/A")
    print(f"  Bundle: {n_facts} facts, confidence={confidence}")

    decision = PARSERS[decision_class](bundle)
    decision["bundle_confidence"] = confidence
    decision["bundle_facts_count"] = n_facts
    return decision


def main() -> None:
    parser = argparse.ArgumentParser(description="Run ANIDB-powered agent")
    parser.add_argument(
        "--decision-class",
        choices=["churn", "pricing", "capacity", "all"],
        default="all",
    )
    parser.add_argument("--semantic-engine-url", default=SEMANTIC_ENGINE_URL)
    parser.add_argument("--output-dir", default="results")
    args = parser.parse_args()

    classes = (
        ["churn", "pricing", "capacity"] if args.decision_class == "all" else [args.decision_class]
    )

    decisions: dict = {
        "agent": "anidb",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "decisions": {},
    }

    for dc in classes:
        print(f"Running {dc} decision (ANIDB agent)...")
        try:
            result = run_decision(dc, args.semantic_engine_url)
            decisions["decisions"][dc] = result
            print(f"  {dc}: {result}")
        except httpx.ConnectError:
            print(f"  ERROR: Cannot connect to semantic engine at {args.semantic_engine_url}")
            print("  Is the ANIDB stack running? Try: ./dev.sh")
            decisions["decisions"][dc] = {"error": "service_unavailable"}
        except httpx.HTTPStatusError as e:
            print(f"  ERROR: HTTP {e.response.status_code}: {e.response.text}")
            decisions["decisions"][dc] = {"error": f"http_{e.response.status_code}"}
        except Exception as e:
            print(f"  ERROR: {e}")
            decisions["decisions"][dc] = {"error": str(e)}

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "anidb_decisions.json"
    with open(output_path, "w") as f:
        json.dump(decisions, f, indent=2, default=str)
    print(f"Results written to {output_path}")


if __name__ == "__main__":
    main()
