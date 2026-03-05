"""
ANIDB Synthetic Data Generator

Generates a deterministic 90-day SaaS business simulation with:
- 500 customers with realistic subscription behavior
- ~10,000 typed events
- Seeded churn signals, pricing changes, capacity events
- Ground truth decision outcomes for scoring

Usage:
    python synthetic-data/generate.py [--customers 500] [--days 90] [--seed 42]
"""
from __future__ import annotations

import argparse
import json
import random
import uuid
from dataclasses import dataclass, field
import os
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

START_DATE = datetime(2025, 1, 1, tzinfo=timezone.utc)

# Build default PG DSN from env vars (populated by sourcing .env)
_PG_DSN_DEFAULT = (
    "postgresql://"
    f"{os.environ.get('POSTGRES_USER', 'anidb')}:"
    f"{os.environ.get('POSTGRES_PASSWORD', 'anidb_dev')}@"
    f"{os.environ.get('POSTGRES_HOST', 'localhost')}:"
    f"{os.environ.get('POSTGRES_PORT', '5432')}/"
    f"{os.environ.get('POSTGRES_DB', 'anidb')}"
)

PLANS: dict[str, dict[str, Any]] = {
    "basic":      {"mrr": 15.0,  "seat_limit": 5},
    "pro":        {"mrr": 40.0,  "seat_limit": 20},
    "enterprise": {"mrr": 175.0, "seat_limit": 100},
}

# Ground-truth optimal prices (Lerner formula result, hardcoded for determinism)
OPTIMAL_PRICES = {"basic": 18, "pro": 45, "enterprise": 199}

ARCHETYPE_DISTRIBUTION = [
    ("healthy",         0.60),
    ("at_risk",         0.20),
    ("price_sensitive", 0.10),
    ("capacity_bound",  0.10),
]

# Must match core/event-log/src/schema.rs VALID_EVENT_TYPES
VALID_EVENT_TYPES = {
    "CustomerSubscribed", "CustomerCancelled", "PlanChanged", "PriceChanged",
    "UsageRecorded", "LoginEvent", "SupportTicketOpened", "SupportTicketClosed",
    "InvoicePaid", "InvoiceFailed", "TrialStarted", "TrialConverted",
    "FeatureUsage", "SeatCountChanged", "CapacityThresholdReached",
}


@dataclass
class SimConfig:
    n_customers: int = 500
    n_days: int = 90
    seed: int = 42
    event_log_url: str = "http://localhost:8010"
    pg_dsn: str = _PG_DSN_DEFAULT
    output_dir: str = "data"


@dataclass
class Customer:
    customer_id: str
    archetype: str
    plan: str
    mrr: float
    seats_used: int
    seat_limit: int
    subscribed_at: datetime
    churned: bool = False
    churn_day: int | None = None


# ---------------------------------------------------------------------------
# Pure simulation functions (importable without I/O deps)
# ---------------------------------------------------------------------------

def choose_archetype(rng: random.Random) -> str:
    r = rng.random()
    cumulative = 0.0
    for archetype, weight in ARCHETYPE_DISTRIBUTION:
        cumulative += weight
        if r < cumulative:
            return archetype
    return "healthy"


def make_customer(archetype: str, rng: random.Random) -> Customer:
    if archetype == "price_sensitive":
        plan = "basic"
    elif archetype == "capacity_bound":
        plan = rng.choice(["basic", "pro"])
    else:
        plan = rng.choices(["basic", "pro", "enterprise"], weights=[0.50, 0.35, 0.15])[0]

    plan_info = PLANS[plan]
    seat_limit = plan_info["seat_limit"]

    if archetype == "capacity_bound":
        seats_used = max(1, seat_limit - rng.randint(0, 1))
    else:
        seats_used = rng.randint(1, max(1, seat_limit // 3))

    subscribed_at = START_DATE - timedelta(days=rng.randint(0, 180))

    return Customer(
        customer_id=str(uuid.UUID(int=rng.getrandbits(128))),
        archetype=archetype,
        plan=plan,
        mrr=plan_info["mrr"],
        seats_used=seats_used,
        seat_limit=seat_limit,
        subscribed_at=subscribed_at,
    )


def _make_event(
    customer: Customer,
    event_type: str,
    payload: dict[str, Any],
    timestamp: datetime,
    correlation_id: str,
    actor: str = "system",
) -> dict[str, Any]:
    return {
        "stream_id": f"customer-{customer.customer_id}",
        "event_type": event_type,
        "payload": payload,
        "metadata": {
            "timestamp": timestamp.isoformat(),
            "actor": actor,
            "causation_id": None,
            "correlation_id": correlation_id,
            "ontology_version": 1,
        },
    }


def simulate_events(
    customers: list[Customer], config: SimConfig, rng: random.Random
) -> list[dict[str, Any]]:
    """Simulate all events for all customers. Mutates customer churn/seat state."""
    events: list[dict[str, Any]] = []
    correlation_id = str(uuid.UUID(int=rng.getrandbits(128)))

    # Subscription events (historical, before simulation window)
    for customer in customers:
        events.append(
            _make_event(
                customer,
                "CustomerSubscribed",
                {
                    "customer_id": customer.customer_id,
                    "plan": customer.plan,
                    "mrr": customer.mrr,
                    "seats_used": customer.seats_used,
                    "seat_limit": customer.seat_limit,
                },
                customer.subscribed_at,
                correlation_id,
                actor="generator",
            )
        )

    # Day-by-day simulation
    for day in range(config.n_days):
        date = START_DATE + timedelta(days=day)
        day_fraction = day / config.n_days
        is_final_14 = day >= (config.n_days - 14)

        for customer in customers:
            if customer.churned:
                continue

            archetype = customer.archetype

            # --- Login probability (archetype-driven) ---
            if archetype == "healthy":
                login_prob = 0.85
            elif archetype == "at_risk":
                # Declining: 0.70 → 0.20 over simulation period
                login_prob = 0.70 - 0.50 * day_fraction
            elif archetype == "price_sensitive":
                login_prob = 0.65
            else:  # capacity_bound
                login_prob = 0.95

            if rng.random() < login_prob:
                events.append(
                    _make_event(
                        customer,
                        "LoginEvent",
                        {
                            "customer_id": customer.customer_id,
                            "session_duration_minutes": rng.randint(5, 120),
                        },
                        date,
                        correlation_id,
                    )
                )

            # --- Monthly invoice (every 30 days from subscription) ---
            days_since_sub = (date - customer.subscribed_at).days
            if days_since_sub > 0 and days_since_sub % 30 == 0:
                fail_prob = 0.20 if archetype == "at_risk" else 0.02
                if rng.random() < fail_prob:
                    events.append(
                        _make_event(
                            customer,
                            "InvoiceFailed",
                            {
                                "customer_id": customer.customer_id,
                                "amount": customer.mrr,
                                "reason": "payment_declined",
                            },
                            date,
                            correlation_id,
                            actor="billing",
                        )
                    )
                else:
                    events.append(
                        _make_event(
                            customer,
                            "InvoicePaid",
                            {
                                "customer_id": customer.customer_id,
                                "amount": customer.mrr,
                            },
                            date,
                            correlation_id,
                            actor="billing",
                        )
                    )

            # --- Support tickets ---
            ticket_prob = {
                "healthy": 0.02,
                "at_risk": 0.12,
                "price_sensitive": 0.04,
                "capacity_bound": 0.06,
            }[archetype]

            if rng.random() < ticket_prob:
                ticket_id = str(uuid.UUID(int=rng.getrandbits(128)))
                events.append(
                    _make_event(
                        customer,
                        "SupportTicketOpened",
                        {
                            "customer_id": customer.customer_id,
                            "ticket_id": ticket_id,
                            "category": rng.choice(["billing", "technical", "feature_request"]),
                        },
                        date,
                        correlation_id,
                        actor="support",
                    )
                )
                # Probabilistic close within 1–5 days
                if rng.random() < 0.70:
                    close_date = date + timedelta(days=rng.randint(1, 5))
                    if close_date <= START_DATE + timedelta(days=config.n_days):
                        events.append(
                            _make_event(
                                customer,
                                "SupportTicketClosed",
                                {"customer_id": customer.customer_id, "ticket_id": ticket_id},
                                close_date,
                                correlation_id,
                                actor="support",
                            )
                        )

            # --- Capacity growth (every 15 days) ---
            if archetype == "capacity_bound" and day % 15 == 0:
                if customer.seats_used < customer.seat_limit:
                    customer.seats_used += 1
                    events.append(
                        _make_event(
                            customer,
                            "FeatureUsage",
                            {
                                "customer_id": customer.customer_id,
                                "feature": "seat_added",
                                "seats_used": customer.seats_used,
                                "seat_limit": customer.seat_limit,
                            },
                            date,
                            correlation_id,
                        )
                    )

            # --- Price change for price_sensitive at day 30 ---
            if archetype == "price_sensitive" and day == 30:
                events.append(
                    _make_event(
                        customer,
                        "PriceChanged",
                        {
                            "customer_id": customer.customer_id,
                            "old_price": customer.mrr,
                            "new_price": round(customer.mrr * 1.15, 2),
                            "tier": customer.plan,
                        },
                        date,
                        correlation_id,
                        actor="billing",
                    )
                )

            # --- Churn (final 14 days) ---
            if is_final_14:
                churn_prob = {
                    "at_risk": 0.12,
                    "price_sensitive": 0.01,
                    "healthy": 0.002,
                    "capacity_bound": 0.001,
                }[archetype]

                if rng.random() < churn_prob:
                    customer.churned = True
                    customer.churn_day = day
                    events.append(
                        _make_event(
                            customer,
                            "CustomerCancelled",
                            {
                                "customer_id": customer.customer_id,
                                "plan": customer.plan,
                                "mrr_lost": customer.mrr,
                                "reason": rng.choice(
                                    ["cancellation", "payment_failure", "downgrade"]
                                ),
                            },
                            date,
                            correlation_id,
                        )
                    )

    return events


def compute_ground_truth(customers: list[Customer]) -> dict[str, Any]:
    """Derive ground truth from simulation state. Call after simulate_events()."""
    churners = [c.customer_id for c in customers if c.churned]
    capacity_bound = [
        c.customer_id
        for c in customers
        if c.archetype == "capacity_bound" and c.seats_used >= c.seat_limit - 1
    ]
    return {
        "churners": churners,
        "optimal_prices": OPTIMAL_PRICES,
        "capacity_bound": capacity_bound,
    }


# ---------------------------------------------------------------------------
# I/O functions
# ---------------------------------------------------------------------------

def write_to_event_log(events: list[dict[str, Any]], config: SimConfig) -> None:
    import httpx

    batch_size = 100
    total = len(events)
    posted = 0
    errors = 0

    with httpx.Client(timeout=30.0) as client:
        for i in range(0, total, batch_size):
            batch = events[i : i + batch_size]
            try:
                resp = client.post(f"{config.event_log_url}/events/batch", json={"events": batch})
                resp.raise_for_status()
                posted += len(batch)
                print(f"  Event log: posted {posted}/{total}")
            except Exception as e:
                errors += len(batch)
                print(f"  WARNING: Failed to post batch {i}–{i+len(batch)}: {e}")

    if errors:
        print(f"  WARNING: {errors} events failed to post to event log")


def compute_customer_signals(
    customers: list[Customer], events: list[dict[str, Any]], config: SimConfig
) -> dict[str, dict[str, Any]]:
    """Derive confidence-store signals for each customer from the event stream."""
    from collections import defaultdict

    n_days = config.n_days
    mid_point = n_days // 2  # split for trend: first half vs second half
    sim_start = START_DATE

    # Aggregate per customer
    logins_early: dict[str, int] = defaultdict(int)
    logins_late: dict[str, int] = defaultdict(int)
    invoice_failures: dict[str, int] = defaultdict(int)
    support_tickets: dict[str, int] = defaultdict(int)

    for event in events:
        cid = event["payload"].get("customer_id")
        if not cid:
            continue
        ts_str = event["metadata"]["timestamp"]
        try:
            ts = datetime.fromisoformat(ts_str)
        except ValueError:
            continue
        day = (ts - sim_start).days

        if event["event_type"] == "LoginEvent":
            if day < mid_point:
                logins_early[cid] += 1
            else:
                logins_late[cid] += 1
        elif event["event_type"] == "InvoiceFailed":
            invoice_failures[cid] += 1
        elif event["event_type"] == "SupportTicketOpened":
            support_tickets[cid] += 1

    signals: dict[str, dict[str, Any]] = {}
    for c in customers:
        cid = c.customer_id
        early = logins_early.get(cid, 0)
        late = logins_late.get(cid, 0)
        # Trend: positive = increasing, negative = declining
        trend = (late - early) / max(1, early)
        seat_util = c.seats_used / c.seat_limit if c.seat_limit > 0 else 0.0
        fail_count = invoice_failures.get(cid, 0)
        ticket_count = support_tickets.get(cid, 0)

        # Simple churn risk score: declining usage + failures + tickets → higher risk
        churn_risk = min(1.0, max(0.0,
            0.40 * max(0.0, -trend)       # declining usage contributes up to 0.4
            + 0.30 * min(1.0, fail_count / 3.0)  # invoice failures up to 0.3
            + 0.20 * min(1.0, ticket_count / 5.0)  # support load up to 0.2
            + 0.10 * max(0.0, 1.0 - seat_util)   # low seat utilisation up to 0.1
        ))

        # customer_id is embedded in each fact value so the ANIDB agent
        # can identify which customer the fact belongs to (Fact struct drops entity_id)
        signals[cid] = {
            "usage_trend":             {"customer_id": cid, "value": round(trend, 4), "30d_logins_early": early, "30d_logins_late": late},
            "invoice_failure_count":   {"customer_id": cid, "value": fail_count},
            "support_ticket_count":    {"customer_id": cid, "value": ticket_count},
            "seat_utilization":        {"customer_id": cid, "value": round(seat_util, 4), "seats_used": c.seats_used, "seat_limit": c.seat_limit},
            "churn_risk_score":        {"customer_id": cid, "value": round(churn_risk, 4)},
            "plan":                    {"customer_id": cid, "value": c.plan, "mrr": c.mrr},
        }

    # Portfolio-level pricing analysis fact (one per simulation, not per customer)
    # entity_id = well-known UUID for the portfolio aggregate
    PORTFOLIO_ENTITY_ID = "00000000-0000-0000-0000-000000000001"
    plan_customers: dict[str, list[Customer]] = {"basic": [], "pro": [], "enterprise": []}
    for c in customers:
        if c.plan in plan_customers:
            plan_customers[c.plan].append(c)

    portfolio_pricing: dict[str, Any] = {
        "customer_id": PORTFOLIO_ENTITY_ID,
        "fact_type": "portfolio_pricing_analysis",
    }
    for tier, tier_customers in plan_customers.items():
        if not tier_customers:
            continue
        churned_in_tier = sum(1 for c in tier_customers if c.churned)
        churn_rate = churned_in_tier / len(tier_customers)
        portfolio_pricing[tier] = {
            "current_price": PLANS[tier]["mrr"],
            "customer_count": len(tier_customers),
            "churn_rate": round(churn_rate, 4),
            "optimal_price": OPTIMAL_PRICES[tier],
        }

    signals[PORTFOLIO_ENTITY_ID] = {
        "portfolio_pricing_analysis": portfolio_pricing,
    }

    return signals


def write_to_confidence_store(
    customers: list[Customer],
    events: list[dict[str, Any]],
    config: SimConfig,
    confidence_store_url: str = "http://localhost:8003",
) -> None:
    import httpx

    signals = compute_customer_signals(customers, events, config)
    total = sum(len(v) for v in signals.values())
    posted = 0
    errors = 0

    FACT_CONFIDENCE = {
        "usage_trend":           (0.85, "event_log_aggregation"),
        "invoice_failure_count": (0.95, "event_log_aggregation"),
        "support_ticket_count":  (0.90, "event_log_aggregation"),
        "seat_utilization":      (0.90, "customer_record"),
        "churn_risk_score":      (0.80, "derived_signal"),
        "plan":                  (1.00, "customer_record"),
    }

    with httpx.Client(timeout=30.0) as client:
        for cid, facts in signals.items():
            for fact_key, fact_payload in facts.items():
                conf_value, conf_source = FACT_CONFIDENCE.get(fact_key, (0.75, "derived"))
                body = {
                    "entity_id": cid,
                    "entity_type": "Customer",
                    "fact_key": fact_key,
                    "fact_value": fact_payload,
                    "confidence_value": conf_value,
                    "confidence_source": conf_source,
                    "derivation": None,
                }
                try:
                    resp = client.post(f"{confidence_store_url}/facts", json=body)
                    resp.raise_for_status()
                    posted += 1
                except Exception as e:
                    errors += 1
                    if errors <= 3:
                        print(f"  WARNING: Failed to post fact {fact_key} for {cid}: {e}")

    print(f"  Confidence store: {posted}/{total} facts posted" + (f", {errors} errors" if errors else ""))


_PG_SCHEMA = """
CREATE SCHEMA IF NOT EXISTS baseline;

CREATE TABLE IF NOT EXISTS baseline.customers (
    customer_id     UUID PRIMARY KEY,
    archetype       VARCHAR(50)    NOT NULL,
    plan            VARCHAR(50)    NOT NULL,
    mrr             DECIMAL(10,2)  NOT NULL,
    seats_used      INT            NOT NULL,
    seat_limit      INT            NOT NULL,
    subscribed_at   TIMESTAMPTZ    NOT NULL,
    churned         BOOLEAN        NOT NULL DEFAULT FALSE,
    churn_date      TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS baseline.events (
    event_id        UUID           DEFAULT gen_random_uuid() PRIMARY KEY,
    customer_id     UUID           NOT NULL,
    event_type      VARCHAR(100)   NOT NULL,
    payload         JSONB,
    occurred_at     TIMESTAMPTZ    NOT NULL
);

CREATE TABLE IF NOT EXISTS baseline.daily_logins (
    id              SERIAL         PRIMARY KEY,
    customer_id     UUID           NOT NULL,
    login_date      DATE           NOT NULL,
    login_count     INT            NOT NULL DEFAULT 1,
    UNIQUE(customer_id, login_date)
);

CREATE TABLE IF NOT EXISTS baseline.invoices (
    invoice_id      UUID           DEFAULT gen_random_uuid() PRIMARY KEY,
    customer_id     UUID           NOT NULL,
    amount          DECIMAL(10,2)  NOT NULL,
    status          VARCHAR(20)    NOT NULL,
    due_date        DATE           NOT NULL,
    paid_at         TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS baseline.support_tickets (
    ticket_id       UUID           PRIMARY KEY,
    customer_id     UUID           NOT NULL,
    opened_at       TIMESTAMPTZ    NOT NULL,
    closed_at       TIMESTAMPTZ,
    status          VARCHAR(20)    NOT NULL DEFAULT 'open'
);
"""


def write_to_postgres(
    customers: list[Customer], events: list[dict[str, Any]], config: SimConfig
) -> None:
    import psycopg2

    try:
        conn = psycopg2.connect(config.pg_dsn)
    except Exception as e:
        print(f"WARNING: Could not connect to PostgreSQL: {e}")
        print("Skipping PostgreSQL mirror write.")
        return

    try:
        with conn:
            with conn.cursor() as cur:
                cur.execute(_PG_SCHEMA)

                # Customers
                for c in customers:
                    churn_date = (
                        START_DATE + timedelta(days=c.churn_day) if c.churn_day is not None else None
                    )
                    cur.execute(
                        """
                        INSERT INTO baseline.customers
                            (customer_id, archetype, plan, mrr, seats_used, seat_limit,
                             subscribed_at, churned, churn_date)
                        VALUES (%s,%s,%s,%s,%s,%s,%s,%s,%s)
                        ON CONFLICT (customer_id) DO UPDATE SET
                            seats_used = EXCLUDED.seats_used,
                            churned    = EXCLUDED.churned,
                            churn_date = EXCLUDED.churn_date
                        """,
                        (
                            c.customer_id, c.archetype, c.plan, c.mrr,
                            c.seats_used, c.seat_limit, c.subscribed_at,
                            c.churned, churn_date,
                        ),
                    )

                # Events (denormalized into helper tables)
                for event in events:
                    ts = datetime.fromisoformat(event["metadata"]["timestamp"])
                    cid = event["payload"].get("customer_id")
                    cur.execute(
                        """
                        INSERT INTO baseline.events (customer_id, event_type, payload, occurred_at)
                        VALUES (%s, %s, %s, %s)
                        """,
                        (cid, event["event_type"], json.dumps(event["payload"]), ts),
                    )

                    if event["event_type"] == "LoginEvent":
                        cur.execute(
                            """
                            INSERT INTO baseline.daily_logins (customer_id, login_date, login_count)
                            VALUES (%s, %s, 1)
                            ON CONFLICT (customer_id, login_date)
                            DO UPDATE SET login_count = baseline.daily_logins.login_count + 1
                            """,
                            (cid, ts.date()),
                        )

                    elif event["event_type"] in ("InvoicePaid", "InvoiceFailed"):
                        status = "paid" if event["event_type"] == "InvoicePaid" else "failed"
                        paid_at = ts if status == "paid" else None
                        cur.execute(
                            """
                            INSERT INTO baseline.invoices
                                (customer_id, amount, status, due_date, paid_at)
                            VALUES (%s, %s, %s, %s, %s)
                            """,
                            (cid, event["payload"]["amount"], status, ts.date(), paid_at),
                        )

                    elif event["event_type"] == "SupportTicketOpened":
                        cur.execute(
                            """
                            INSERT INTO baseline.support_tickets
                                (ticket_id, customer_id, opened_at, status)
                            VALUES (%s, %s, %s, 'open')
                            ON CONFLICT (ticket_id) DO NOTHING
                            """,
                            (event["payload"]["ticket_id"], cid, ts),
                        )

                    elif event["event_type"] == "SupportTicketClosed":
                        cur.execute(
                            """
                            UPDATE baseline.support_tickets
                            SET closed_at = %s, status = 'closed'
                            WHERE ticket_id = %s
                            """,
                            (ts, event["payload"]["ticket_id"]),
                        )
    finally:
        conn.close()

    print("PostgreSQL mirror written successfully.")


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="Generate synthetic SaaS data for ANIDB experiments")
    parser.add_argument("--customers", type=int, default=500)
    parser.add_argument("--days",      type=int, default=90)
    parser.add_argument("--seed",      type=int, default=42)
    parser.add_argument("--event-log-url", default="http://localhost:8010")
    parser.add_argument("--pg-dsn",        default=_PG_DSN_DEFAULT)
    parser.add_argument("--output-dir",    default="data")
    parser.add_argument("--confidence-store-url", default="http://localhost:8003")
    parser.add_argument("--skip-event-log",       action="store_true")
    parser.add_argument("--skip-postgres",         action="store_true")
    parser.add_argument("--skip-confidence-store", action="store_true")
    args = parser.parse_args()

    config = SimConfig(
        n_customers=args.customers,
        n_days=args.days,
        seed=args.seed,
        event_log_url=args.event_log_url,
        pg_dsn=args.pg_dsn,
        output_dir=args.output_dir,
    )

    print(f"Generating: {config.n_customers} customers, {config.n_days} days, seed={config.seed}")
    rng = random.Random(config.seed)

    # Generate customers
    customers: list[Customer] = []
    for _ in range(config.n_customers):
        archetype = choose_archetype(rng)
        customers.append(make_customer(archetype, rng))

    counts: dict[str, int] = {}
    for c in customers:
        counts[c.archetype] = counts.get(c.archetype, 0) + 1
    print(f"Archetypes: {counts}")

    # Simulate events (mutates customer churn/seat state)
    print("Simulating events...")
    events = simulate_events(customers, config, rng)
    print(f"Generated {len(events)} events")

    # Ground truth (after simulation so churn state is populated)
    ground_truth = compute_ground_truth(customers)
    print(
        f"Ground truth: {len(ground_truth['churners'])} churners, "
        f"{len(ground_truth['capacity_bound'])} capacity_bound"
    )

    # Write ground truth
    output_dir = Path(config.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    gt_path = output_dir / "ground_truth.json"
    with open(gt_path, "w") as f:
        json.dump(ground_truth, f, indent=2)
    print(f"Ground truth written to {gt_path}")

    # Write to event log
    if not args.skip_event_log:
        print("Writing to event log...")
        write_to_event_log(events, config)

    # Write to PostgreSQL
    if not args.skip_postgres:
        print("Writing to PostgreSQL...")
        write_to_postgres(customers, events, config)

    # Write derived facts to confidence store (required for ANIDB agent)
    if not args.skip_confidence_store:
        print("Writing facts to confidence store...")
        write_to_confidence_store(customers, events, config, confidence_store_url=args.confidence_store_url)

    print("Done.")


if __name__ == "__main__":
    main()
