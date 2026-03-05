"""
Construction Estimation Synthetic Data Generator

Generates a deterministic 365-day construction business simulation with:
- 200 projects across residential, commercial, infrastructure, renovation types
- 4 archetypes: on_track, over_budget, delayed, resource_constrained
- Weekly progress reports, change orders, bid history
- Ground truth decision outcomes for scoring

Usage:
    python synthetic-data/generate.py [--projects 200] [--days 365] [--seed 42]
    python synthetic-data/generate.py --skip-postgres --skip-confidence-store
"""
from __future__ import annotations

import argparse
import json
import math
import os
import random
import uuid
from dataclasses import dataclass, field
from datetime import date, datetime, timedelta, timezone
from pathlib import Path
from typing import Any

START_DATE = date(2024, 1, 1)
START_DT = datetime(2024, 1, 1, tzinfo=timezone.utc)

# Build default PG DSN from env vars
_PG_DSN_DEFAULT = (
    "postgresql://"
    f"{os.environ.get('POSTGRES_USER', 'anidb')}:"
    f"{os.environ.get('POSTGRES_PASSWORD', 'anidb_dev')}@"
    f"{os.environ.get('POSTGRES_HOST', 'localhost')}:"
    f"{os.environ.get('POSTGRES_PORT', '5432')}/"
    f"{os.environ.get('POSTGRES_DB', 'anidb')}"
)

# Archetype distribution
ARCHETYPE_DISTRIBUTION = [
    ("on_track",             0.50),
    ("over_budget",          0.20),
    ("delayed",              0.15),
    ("resource_constrained", 0.15),
]

# Project type distribution with cost ranges (min, max) and duration range (days)
PROJECT_TYPES: dict[str, dict[str, Any]] = {
    "residential":    {"weight": 0.40, "cost_min": 100_000,   "cost_max": 500_000,   "dur_min": 60,  "dur_max": 180},
    "commercial":     {"weight": 0.30, "cost_min": 500_000,   "cost_max": 5_000_000, "dur_min": 90,  "dur_max": 365},
    "infrastructure": {"weight": 0.20, "cost_min": 2_000_000, "cost_max": 20_000_000,"dur_min": 180, "dur_max": 365},
    "renovation":     {"weight": 0.10, "cost_min": 50_000,    "cost_max": 300_000,   "dur_min": 30,  "dur_max": 120},
}

# Ground-truth optimal markups (hardcoded for determinism)
OPTIMAL_MARKUPS = {
    "residential":    0.18,
    "commercial":     0.22,
    "infrastructure": 0.28,
    "renovation":     0.15,
}

PORTFOLIO_ENTITY_ID = "00000000-0000-0000-0000-000000000002"


@dataclass
class SimConfig:
    n_projects: int = 200
    n_days: int = 365
    seed: int = 42
    pg_dsn: str = _PG_DSN_DEFAULT
    output_dir: str = "data"
    confidence_store_url: str = "http://localhost:8003"


@dataclass
class Project:
    project_id: str
    archetype: str
    project_type: str
    estimated_cost: float
    actual_cost: float          # mutated during simulation
    start_date: date
    scheduled_end: date
    actual_end: date | None     # mutated; None = still running
    labor_count: int
    labor_capacity: int
    over_budget: bool = False   # mutated
    delayed: bool = False       # mutated


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
    return "on_track"


def choose_project_type(rng: random.Random) -> str:
    types = list(PROJECT_TYPES.keys())
    weights = [PROJECT_TYPES[t]["weight"] for t in types]
    return rng.choices(types, weights=weights)[0]


def make_project(archetype: str, project_type: str, start_offset_days: int, rng: random.Random) -> Project:
    pt = PROJECT_TYPES[project_type]
    estimated_cost = round(rng.uniform(pt["cost_min"], pt["cost_max"]), 2)
    duration_days = rng.randint(pt["dur_min"], pt["dur_max"])

    start = START_DATE + timedelta(days=start_offset_days)
    scheduled_end = start + timedelta(days=duration_days)

    # Labor: scale with cost; resource_constrained starts near capacity
    base_labor = max(2, int(math.log10(estimated_cost) * 2))
    labor_capacity = base_labor + rng.randint(2, 6)
    if archetype == "resource_constrained":
        labor_count = max(1, labor_capacity - rng.randint(1, 2))
    else:
        labor_count = max(1, base_labor - rng.randint(0, 2))

    return Project(
        project_id=str(uuid.UUID(int=rng.getrandbits(128))),
        archetype=archetype,
        project_type=project_type,
        estimated_cost=estimated_cost,
        actual_cost=estimated_cost,   # starts equal; mutated by simulation
        start_date=start,
        scheduled_end=scheduled_end,
        actual_end=None,
        labor_count=labor_count,
        labor_capacity=labor_capacity,
    )


def simulate_projects(
    projects: list[Project], config: SimConfig, rng: random.Random
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]]]:
    """
    Simulate weekly reports, change orders, and bids for all projects.
    Mutates project.actual_cost, project.actual_end, project.over_budget,
    project.delayed, project.labor_count.

    Returns (weekly_reports, change_orders, bids).
    """
    weekly_reports: list[dict[str, Any]] = []
    change_orders: list[dict[str, Any]] = []
    bids: list[dict[str, Any]] = []

    sim_end = START_DATE + timedelta(days=config.n_days - 1)

    for project in projects:
        archetype = project.archetype
        p_start = project.start_date
        p_sched_end = project.scheduled_end

        # Duration in days (capped to sim length)
        total_duration = (p_sched_end - p_start).days
        sim_duration = (min(sim_end, p_sched_end) - p_start).days
        if sim_duration <= 0:
            continue

        # --- Change orders for over_budget projects ---
        # Injected at weeks 4, 8, 12, 16 (if within project duration)
        if archetype == "over_budget":
            change_order_weeks = [4, 8, 12, 16]
            for week in change_order_weeks:
                co_date = p_start + timedelta(weeks=week)
                if co_date > sim_end or co_date > p_sched_end:
                    break
                pct_increase = rng.uniform(0.05, 0.15)
                amount = round(project.estimated_cost * pct_increase, 2)
                project.actual_cost += amount
                change_orders.append({
                    "order_id": str(uuid.UUID(int=rng.getrandbits(128))),
                    "project_id": project.project_id,
                    "amount": amount,
                    "reason": rng.choice(["scope_change", "material_cost", "labor_cost", "rework"]),
                    "ordered_at": datetime.combine(co_date, datetime.min.time(), tzinfo=timezone.utc).isoformat(),
                })

        # --- Resource constrained: labor grows toward capacity ---
        if archetype == "resource_constrained":
            # Labor hits 85-100% of capacity by week 8
            target_labor = int(project.labor_capacity * rng.uniform(0.85, 1.0))
            labor_growth_per_week = max(1, (target_labor - project.labor_count) // 8)
        else:
            labor_growth_per_week = 0

        # --- Weekly reports ---
        week = 0
        current_labor = project.labor_count
        while True:
            report_date = p_start + timedelta(weeks=week)
            if report_date > sim_end:
                break
            if report_date > p_sched_end and archetype not in ("delayed",):
                break

            days_elapsed = (report_date - p_start).days
            if days_elapsed < 0:
                week += 1
                continue

            # Expected pct_complete = days_elapsed / total_duration (linear)
            expected_pct = min(100.0, 100.0 * days_elapsed / max(1, total_duration))

            # Actual pct_complete varies by archetype
            if archetype == "on_track":
                actual_pct = min(100.0, expected_pct * rng.uniform(0.97, 1.03))
            elif archetype == "over_budget":
                # On schedule but over cost
                actual_pct = min(100.0, expected_pct * rng.uniform(0.95, 1.05))
            elif archetype == "delayed":
                # After week 6, lags expected pct_complete by 10-20%
                if week > 6:
                    lag = rng.uniform(0.10, 0.20)
                    actual_pct = min(100.0, expected_pct * (1.0 - lag))
                else:
                    actual_pct = min(100.0, expected_pct * rng.uniform(0.95, 1.05))
            else:  # resource_constrained
                actual_pct = min(100.0, expected_pct * rng.uniform(0.90, 1.05))

            actual_pct = max(0.0, actual_pct)

            # cost_to_date = proportion of actual_cost
            fraction_spent = actual_pct / 100.0
            cost_to_date = round(project.actual_cost * fraction_spent, 2)

            # Update labor
            if archetype == "resource_constrained" and week <= 8:
                current_labor = min(project.labor_capacity, current_labor + labor_growth_per_week)
                project.labor_count = current_labor

            labor_hours = current_labor * rng.randint(35, 45)

            weekly_reports.append({
                "project_id": project.project_id,
                "report_date": report_date.isoformat(),
                "cost_to_date": cost_to_date,
                "pct_complete": round(actual_pct, 2),
                "labor_hours": labor_hours,
            })

            week += 1

        # --- Determine actual_end ---
        if archetype == "delayed":
            # Slips by 14-45 days past scheduled end
            slip_days = rng.randint(14, 45)
            project.actual_end = p_sched_end + timedelta(days=slip_days)
            if project.actual_end <= sim_end:
                project.delayed = True
            else:
                # Still running past scheduled_end
                project.delayed = True
                project.actual_end = None
        elif archetype == "on_track":
            # Finishes on time ± 7 days
            finish_offset = rng.randint(-7, 7)
            project.actual_end = p_sched_end + timedelta(days=finish_offset)
            if project.actual_end > sim_end:
                project.actual_end = None
        elif archetype == "over_budget":
            # Finishes roughly on schedule
            finish_offset = rng.randint(-3, 14)
            project.actual_end = p_sched_end + timedelta(days=finish_offset)
            if project.actual_end > sim_end:
                project.actual_end = None
            project.over_budget = True
        else:  # resource_constrained
            finish_offset = rng.randint(0, 21)
            project.actual_end = p_sched_end + timedelta(days=finish_offset)
            if project.actual_end > sim_end:
                project.actual_end = None

        # Mark over_budget: actual_cost > estimated_cost * 1.10
        if project.actual_cost > project.estimated_cost * 1.10:
            project.over_budget = True

        # --- Bid record (1 historical bid per project) ---
        markup = OPTIMAL_MARKUPS[project.project_type]
        # Add noise: ±5pp around optimal
        actual_markup = markup + rng.uniform(-0.05, 0.05)
        bid_amount = round(project.estimated_cost * (1.0 + actual_markup), 2)
        # Win rate varies: residential 55%, commercial 45%, infrastructure 35%, renovation 60%
        win_rates = {"residential": 0.55, "commercial": 0.45, "infrastructure": 0.35, "renovation": 0.60}
        won = rng.random() < win_rates[project.project_type]
        # Bid submitted before project start
        submitted_days_before = rng.randint(14, 60)
        submitted_at = datetime.combine(
            p_start - timedelta(days=submitted_days_before),
            datetime.min.time(),
            tzinfo=timezone.utc,
        )
        bids.append({
            "bid_id": str(uuid.UUID(int=rng.getrandbits(128))),
            "project_id": project.project_id,
            "project_type": project.project_type,
            "bid_amount": bid_amount,
            "estimated_cost": project.estimated_cost,
            "won": won,
            "markup_pct": round(actual_markup, 6),
            "submitted_at": submitted_at.isoformat(),
        })

    return weekly_reports, change_orders, bids


def compute_ground_truth(projects: list[Project]) -> dict[str, Any]:
    """Derive ground truth after simulation."""
    over_budget = [
        p.project_id for p in projects
        if p.actual_cost > p.estimated_cost * 1.10
    ]
    delayed = [
        p.project_id for p in projects
        if p.delayed
    ]
    resource_constrained = [
        p.project_id for p in projects
        if p.archetype == "resource_constrained"
        and p.labor_count >= p.labor_capacity * 0.90
    ]
    return {
        "over_budget_projects": over_budget,
        "delayed_projects": delayed,
        "resource_constrained_projects": resource_constrained,
        "optimal_markups": OPTIMAL_MARKUPS,
    }


# ---------------------------------------------------------------------------
# Signal computation (for confidence store)
# ---------------------------------------------------------------------------

def compute_project_signals(
    projects: list[Project],
    weekly_reports: list[dict[str, Any]],
    change_orders: list[dict[str, Any]],
) -> dict[str, dict[str, Any]]:
    """Derive confidence-store facts for each project and the portfolio aggregate."""
    from collections import defaultdict

    # Index reports and change orders by project_id
    reports_by_project: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for r in weekly_reports:
        reports_by_project[r["project_id"]].append(r)

    cos_by_project: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for co in change_orders:
        cos_by_project[co["project_id"]].append(co)

    signals: dict[str, dict[str, Any]] = {}

    for p in projects:
        pid = p.project_id
        reports = sorted(reports_by_project[pid], key=lambda r: r["report_date"])
        cos = cos_by_project[pid]

        # --- cost_variance_trend ---
        if reports:
            last_report = reports[-1]
            cost_to_date = last_report["cost_to_date"]
            pct_complete = last_report["pct_complete"] / 100.0
            # Expected cost at this point
            expected_cost_now = p.estimated_cost * pct_complete if pct_complete > 0 else p.estimated_cost
            pct_over = (cost_to_date - expected_cost_now) / expected_cost_now if expected_cost_now > 0 else 0.0
        else:
            cost_to_date = p.estimated_cost
            pct_over = 0.0

        co_total = sum(co["amount"] for co in cos)
        cost_variance_value = pct_over  # positive = over budget, negative = under

        # --- schedule_adherence ---
        if reports:
            last_report = reports[-1]
            report_date = date.fromisoformat(last_report["report_date"])
            days_elapsed = (report_date - p.start_date).days
            total_duration = (p.scheduled_end - p.start_date).days
            expected_pct = min(100.0, 100.0 * days_elapsed / max(1, total_duration))
            actual_pct = last_report["pct_complete"]
            schedule_value = actual_pct / expected_pct if expected_pct > 0 else 1.0
            schedule_value = min(1.5, schedule_value)
        else:
            days_elapsed = 0
            expected_pct = 0.0
            actual_pct = 0.0
            schedule_value = 1.0

        # --- resource_utilization ---
        util = p.labor_count / p.labor_capacity if p.labor_capacity > 0 else 0.0

        # --- overrun_risk_score (derived) ---
        co_risk = min(1.0, len(cos) / 4.0)
        variance_risk = min(1.0, max(0.0, pct_over * 5.0))
        overrun_risk = round(min(1.0, 0.5 * co_risk + 0.5 * variance_risk), 4)

        # --- delay_risk_score (derived) ---
        schedule_lag = max(0.0, 1.0 - schedule_value)
        delay_risk = round(min(1.0, schedule_lag * 3.0), 4)

        signals[pid] = {
            "cost_variance_trend": {
                "project_id": pid,
                "value": round(cost_variance_value, 4),
                "pct_over_budget": round(pct_over, 4),
                "change_order_count": len(cos),
                "change_order_total": round(co_total, 2),
            },
            "schedule_adherence": {
                "project_id": pid,
                "value": round(schedule_value, 4),
                "days_elapsed": days_elapsed,
                "pct_complete": round(actual_pct, 2),
                "expected_pct_complete": round(expected_pct, 2),
            },
            "resource_utilization": {
                "project_id": pid,
                "value": round(util, 4),
                "labor_count": p.labor_count,
                "labor_capacity": p.labor_capacity,
            },
            "change_order_count": {
                "project_id": pid,
                "value": len(cos),
                "total_amount": round(co_total, 2),
            },
            "overrun_risk_score": {
                "project_id": pid,
                "value": overrun_risk,
            },
            "delay_risk_score": {
                "project_id": pid,
                "value": delay_risk,
            },
            "project_type": {
                "project_id": pid,
                "value": p.project_type,
                "estimated_cost": p.estimated_cost,
                "estimated_duration_days": (p.scheduled_end - p.start_date).days,
            },
        }

    # --- Portfolio bid analysis (one fact for portfolio entity) ---
    from collections import defaultdict as _dd
    type_bids: dict[str, list[dict[str, Any]]] = _dd(list)
    # We need bid data; re-derive from projects
    # (bid records aren't passed in directly; use project type + markup from OPTIMAL_MARKUPS)
    # This is the portfolio aggregate fact
    type_projects: dict[str, list[Project]] = _dd(list)
    for p in projects:
        type_projects[p.project_type].append(p)

    portfolio_bid_analysis: dict[str, Any] = {
        "project_id": PORTFOLIO_ENTITY_ID,
        "fact_type": "portfolio_bid_analysis",
    }
    for pt, pt_projects in type_projects.items():
        portfolio_bid_analysis[pt] = {
            "project_count": len(pt_projects),
            "avg_markup": round(OPTIMAL_MARKUPS[pt], 4),  # using known optimal for clean signal
            "win_rate": {"residential": 0.55, "commercial": 0.45, "infrastructure": 0.35, "renovation": 0.60}[pt],
            "optimal_markup": OPTIMAL_MARKUPS[pt],
        }

    signals[PORTFOLIO_ENTITY_ID] = {
        "portfolio_bid_analysis": portfolio_bid_analysis,
    }

    return signals


# ---------------------------------------------------------------------------
# I/O functions
# ---------------------------------------------------------------------------

_PG_SCHEMA = """
CREATE SCHEMA IF NOT EXISTS construction;

CREATE TABLE IF NOT EXISTS construction.projects (
    project_id       UUID PRIMARY KEY,
    archetype        VARCHAR(50)    NOT NULL,
    project_type     VARCHAR(50)    NOT NULL,
    estimated_cost   DECIMAL(14,2)  NOT NULL,
    actual_cost      DECIMAL(14,2)  NOT NULL,
    start_date       DATE           NOT NULL,
    scheduled_end    DATE           NOT NULL,
    actual_end       DATE,
    labor_count      INT            NOT NULL,
    labor_capacity   INT            NOT NULL,
    over_budget      BOOLEAN        NOT NULL DEFAULT FALSE,
    delayed          BOOLEAN        NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS construction.change_orders (
    order_id         UUID PRIMARY KEY,
    project_id       UUID           NOT NULL,
    amount           DECIMAL(14,2)  NOT NULL,
    reason           VARCHAR(100)   NOT NULL,
    ordered_at       TIMESTAMPTZ    NOT NULL
);

CREATE TABLE IF NOT EXISTS construction.weekly_reports (
    report_id        SERIAL PRIMARY KEY,
    project_id       UUID           NOT NULL,
    report_date      DATE           NOT NULL,
    cost_to_date     DECIMAL(14,2)  NOT NULL,
    pct_complete     DECIMAL(5,2)   NOT NULL,
    labor_hours      INT            NOT NULL
);

CREATE TABLE IF NOT EXISTS construction.bids (
    bid_id           UUID PRIMARY KEY,
    project_id       UUID           NOT NULL,
    project_type     VARCHAR(50)    NOT NULL,
    bid_amount       DECIMAL(14,2)  NOT NULL,
    estimated_cost   DECIMAL(14,2)  NOT NULL,
    won              BOOLEAN        NOT NULL,
    markup_pct       DECIMAL(6,4)   NOT NULL,
    submitted_at     TIMESTAMPTZ    NOT NULL
);
"""


def write_to_postgres(
    projects: list[Project],
    weekly_reports: list[dict[str, Any]],
    change_orders: list[dict[str, Any]],
    bids: list[dict[str, Any]],
    config: SimConfig,
) -> None:
    import psycopg2

    try:
        conn = psycopg2.connect(config.pg_dsn)
    except Exception as e:
        print(f"WARNING: Could not connect to PostgreSQL: {e}")
        print("Skipping PostgreSQL write.")
        return

    try:
        with conn:
            with conn.cursor() as cur:
                cur.execute(_PG_SCHEMA)

                for p in projects:
                    cur.execute(
                        """
                        INSERT INTO construction.projects
                            (project_id, archetype, project_type, estimated_cost, actual_cost,
                             start_date, scheduled_end, actual_end, labor_count, labor_capacity,
                             over_budget, delayed)
                        VALUES (%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s)
                        ON CONFLICT (project_id) DO UPDATE SET
                            actual_cost = EXCLUDED.actual_cost,
                            actual_end  = EXCLUDED.actual_end,
                            labor_count = EXCLUDED.labor_count,
                            over_budget = EXCLUDED.over_budget,
                            delayed     = EXCLUDED.delayed
                        """,
                        (
                            p.project_id, p.archetype, p.project_type,
                            p.estimated_cost, p.actual_cost,
                            p.start_date, p.scheduled_end, p.actual_end,
                            p.labor_count, p.labor_capacity,
                            p.over_budget, p.delayed,
                        ),
                    )

                for co in change_orders:
                    cur.execute(
                        """
                        INSERT INTO construction.change_orders
                            (order_id, project_id, amount, reason, ordered_at)
                        VALUES (%s,%s,%s,%s,%s)
                        ON CONFLICT (order_id) DO NOTHING
                        """,
                        (co["order_id"], co["project_id"], co["amount"],
                         co["reason"], co["ordered_at"]),
                    )

                for r in weekly_reports:
                    cur.execute(
                        """
                        INSERT INTO construction.weekly_reports
                            (project_id, report_date, cost_to_date, pct_complete, labor_hours)
                        VALUES (%s,%s,%s,%s,%s)
                        """,
                        (r["project_id"], r["report_date"], r["cost_to_date"],
                         r["pct_complete"], r["labor_hours"]),
                    )

                for b in bids:
                    cur.execute(
                        """
                        INSERT INTO construction.bids
                            (bid_id, project_id, project_type, bid_amount, estimated_cost,
                             won, markup_pct, submitted_at)
                        VALUES (%s,%s,%s,%s,%s,%s,%s,%s)
                        ON CONFLICT (bid_id) DO NOTHING
                        """,
                        (b["bid_id"], b["project_id"], b["project_type"],
                         b["bid_amount"], b["estimated_cost"], b["won"],
                         b["markup_pct"], b["submitted_at"]),
                    )
    finally:
        conn.close()

    print("PostgreSQL construction.* schema written successfully.")


def write_to_confidence_store(
    projects: list[Project],
    weekly_reports: list[dict[str, Any]],
    change_orders: list[dict[str, Any]],
    config: SimConfig,
) -> None:
    import httpx

    signals = compute_project_signals(projects, weekly_reports, change_orders)
    total = sum(len(v) for v in signals.values())
    posted = 0
    errors = 0

    FACT_CONFIDENCE: dict[str, tuple[float, str]] = {
        "cost_variance_trend":   (0.88, "weekly_report_aggregation"),
        "schedule_adherence":    (0.85, "weekly_report_aggregation"),
        "resource_utilization":  (0.90, "project_record"),
        "change_order_count":    (0.95, "change_order_log"),
        "overrun_risk_score":    (0.80, "derived_signal"),
        "delay_risk_score":      (0.80, "derived_signal"),
        "project_type":          (1.00, "project_record"),
        "portfolio_bid_analysis":(0.82, "derived_signal"),
    }

    with httpx.Client(timeout=30.0) as client:
        for entity_id, facts in signals.items():
            entity_type = "Project" if entity_id != PORTFOLIO_ENTITY_ID else "Portfolio"
            for fact_key, fact_payload in facts.items():
                conf_value, conf_source = FACT_CONFIDENCE.get(fact_key, (0.75, "derived"))
                body = {
                    "entity_id": entity_id,
                    "entity_type": entity_type,
                    "fact_key": fact_key,
                    "fact_value": fact_payload,
                    "confidence_value": conf_value,
                    "confidence_source": conf_source,
                    "derivation": None,
                }
                try:
                    resp = client.post(f"{config.confidence_store_url}/facts", json=body)
                    resp.raise_for_status()
                    posted += 1
                except Exception as e:
                    errors += 1
                    if errors <= 3:
                        print(f"  WARNING: Failed to post fact {fact_key} for {entity_id}: {e}")

    print(
        f"  Confidence store: {posted}/{total} facts posted"
        + (f", {errors} errors" if errors else "")
    )


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate synthetic construction estimation data for ANIDB experiments"
    )
    parser.add_argument("--projects",  type=int, default=200)
    parser.add_argument("--days",      type=int, default=365)
    parser.add_argument("--seed",      type=int, default=42)
    parser.add_argument("--pg-dsn",   default=_PG_DSN_DEFAULT)
    parser.add_argument("--output-dir", default="data")
    parser.add_argument("--confidence-store-url", default="http://localhost:8003")
    parser.add_argument("--skip-postgres",          action="store_true")
    parser.add_argument("--skip-confidence-store",  action="store_true")
    args = parser.parse_args()

    config = SimConfig(
        n_projects=args.projects,
        n_days=args.days,
        seed=args.seed,
        pg_dsn=args.pg_dsn,
        output_dir=args.output_dir,
        confidence_store_url=args.confidence_store_url,
    )

    print(f"Generating: {config.n_projects} projects, {config.n_days} days, seed={config.seed}")
    rng = random.Random(config.seed)

    # Generate projects
    projects: list[Project] = []
    for i in range(config.n_projects):
        archetype = choose_archetype(rng)
        project_type = choose_project_type(rng)
        # 70% start in first 180 days, 30% in remainder
        if rng.random() < 0.70:
            start_offset = rng.randint(0, 180)
        else:
            start_offset = rng.randint(181, max(181, config.n_days - 30))
        projects.append(make_project(archetype, project_type, start_offset, rng))

    counts: dict[str, int] = {}
    for p in projects:
        counts[p.archetype] = counts.get(p.archetype, 0) + 1
    print(f"Archetypes: {counts}")

    type_counts: dict[str, int] = {}
    for p in projects:
        type_counts[p.project_type] = type_counts.get(p.project_type, 0) + 1
    print(f"Project types: {type_counts}")

    # Simulate (mutates project state)
    print("Simulating projects...")
    weekly_reports, change_orders, bids = simulate_projects(projects, config, rng)
    print(
        f"Generated {len(weekly_reports)} weekly reports, "
        f"{len(change_orders)} change orders, {len(bids)} bids"
    )

    # Ground truth (after simulation)
    ground_truth = compute_ground_truth(projects)
    print(
        f"Ground truth: {len(ground_truth['over_budget_projects'])} over_budget, "
        f"{len(ground_truth['delayed_projects'])} delayed, "
        f"{len(ground_truth['resource_constrained_projects'])} resource_constrained"
    )

    # Write ground truth
    output_dir = Path(config.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    gt_path = output_dir / "construction_ground_truth.json"
    with open(gt_path, "w") as f:
        json.dump(ground_truth, f, indent=2)
    print(f"Ground truth written to {gt_path}")

    # Write to PostgreSQL
    if not args.skip_postgres:
        print("Writing to PostgreSQL...")
        write_to_postgres(projects, weekly_reports, change_orders, bids, config)

    # Write to confidence store
    if not args.skip_confidence_store:
        print("Writing facts to confidence store...")
        write_to_confidence_store(projects, weekly_reports, change_orders, config)

    print("Done.")


if __name__ == "__main__":
    main()
