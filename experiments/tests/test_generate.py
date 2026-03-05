"""
Unit tests for synthetic-data/generate.py

Tests cover: archetype factory, event probability bounds,
and ground truth determinism. No network calls required.
"""
from __future__ import annotations

import random
import uuid

import pytest

from synthetic_data import generate as gen  # type: ignore[import]


# ---------------------------------------------------------------------------
# Archetype factory tests
# ---------------------------------------------------------------------------

def test_choose_archetype_returns_valid_archetype():
    rng = random.Random(42)
    for _ in range(100):
        a = gen.choose_archetype(rng)
        assert a in {"healthy", "at_risk", "price_sensitive", "capacity_bound"}


def test_choose_archetype_distribution():
    """With 5000 samples the observed proportions should be ±5pp of target."""
    rng = random.Random(7)
    counts: dict[str, int] = {}
    n = 5000
    for _ in range(n):
        a = gen.choose_archetype(rng)
        counts[a] = counts.get(a, 0) + 1

    targets = {"healthy": 0.60, "at_risk": 0.20, "price_sensitive": 0.10, "capacity_bound": 0.10}
    tolerance = 0.05
    for archetype, target in targets.items():
        observed = counts.get(archetype, 0) / n
        assert abs(observed - target) < tolerance, (
            f"{archetype}: observed {observed:.3f}, expected {target} ± {tolerance}"
        )


def test_choose_archetype_deterministic():
    """Same seed → same sequence."""
    seq1 = [gen.choose_archetype(random.Random(99)) for _ in range(20)]
    seq2 = [gen.choose_archetype(random.Random(99)) for _ in range(20)]
    assert seq1 == seq2


def test_make_customer_price_sensitive_always_basic():
    rng = random.Random(1)
    for _ in range(20):
        c = gen.make_customer("price_sensitive", rng)
        assert c.plan == "basic", "price_sensitive customers must be on basic plan"


def test_make_customer_capacity_bound_near_seat_limit():
    rng = random.Random(2)
    for _ in range(20):
        c = gen.make_customer("capacity_bound", rng)
        assert c.seats_used >= c.seat_limit - 1, (
            f"capacity_bound customer has seats_used={c.seats_used} but limit={c.seat_limit}"
        )


def test_make_customer_valid_uuid():
    rng = random.Random(3)
    c = gen.make_customer("healthy", rng)
    # Should not raise
    parsed = uuid.UUID(c.customer_id)
    assert str(parsed) == c.customer_id


def test_make_customer_unique_ids():
    rng = random.Random(4)
    ids = [gen.make_customer("healthy", rng).customer_id for _ in range(100)]
    assert len(set(ids)) == 100, "Customer IDs must be unique"


def test_make_customer_mrr_matches_plan():
    rng = random.Random(5)
    for archetype in ("healthy", "at_risk", "price_sensitive", "capacity_bound"):
        c = gen.make_customer(archetype, rng)
        expected_mrr = gen.PLANS[c.plan]["mrr"]
        assert c.mrr == expected_mrr


# ---------------------------------------------------------------------------
# Event simulation tests
# ---------------------------------------------------------------------------

def test_simulate_events_deterministic():
    """Same seed and config → identical event list."""
    config = gen.SimConfig(n_customers=10, n_days=10, seed=42)

    def run():
        rng = random.Random(42)
        customers = [gen.make_customer(gen.choose_archetype(rng), rng) for _ in range(10)]
        return gen.simulate_events(customers, config, rng)

    events1 = run()
    events2 = run()
    assert len(events1) == len(events2)
    for e1, e2 in zip(events1, events2):
        assert e1["event_type"] == e2["event_type"]
        assert e1["stream_id"] == e2["stream_id"]


def test_simulate_events_valid_event_types():
    config = gen.SimConfig(n_customers=15, n_days=20, seed=42)
    rng = random.Random(42)
    customers = [gen.make_customer(gen.choose_archetype(rng), rng) for _ in range(15)]
    events = gen.simulate_events(customers, config, rng)

    for e in events:
        assert e["event_type"] in gen.VALID_EVENT_TYPES, (
            f"Unknown event type: {e['event_type']}"
        )


def test_simulate_events_stream_id_format():
    config = gen.SimConfig(n_customers=5, n_days=5, seed=42)
    rng = random.Random(42)
    customers = [gen.make_customer(gen.choose_archetype(rng), rng) for _ in range(5)]
    events = gen.simulate_events(customers, config, rng)

    for e in events:
        assert e["stream_id"].startswith("customer-"), (
            f"stream_id must start with 'customer-': {e['stream_id']}"
        )


def test_simulate_events_metadata_keys():
    config = gen.SimConfig(n_customers=5, n_days=5, seed=42)
    rng = random.Random(42)
    customers = [gen.make_customer(gen.choose_archetype(rng), rng) for _ in range(5)]
    events = gen.simulate_events(customers, config, rng)

    required_meta = {"timestamp", "actor", "causation_id", "correlation_id", "ontology_version"}
    for e in events:
        assert required_meta.issubset(e["metadata"].keys())


# ---------------------------------------------------------------------------
# Ground truth tests
# ---------------------------------------------------------------------------

def test_compute_ground_truth_churners_are_subset_of_customers():
    config = gen.SimConfig(n_customers=30, n_days=30, seed=42)
    rng = random.Random(42)
    customers = [gen.make_customer(gen.choose_archetype(rng), rng) for _ in range(30)]
    gen.simulate_events(customers, config, rng)
    gt = gen.compute_ground_truth(customers)

    customer_ids = {c.customer_id for c in customers}
    for cid in gt["churners"]:
        assert cid in customer_ids


def test_compute_ground_truth_capacity_bound_are_subset():
    config = gen.SimConfig(n_customers=30, n_days=30, seed=42)
    rng = random.Random(42)
    customers = [gen.make_customer(gen.choose_archetype(rng), rng) for _ in range(30)]
    gen.simulate_events(customers, config, rng)
    gt = gen.compute_ground_truth(customers)

    customer_ids = {c.customer_id for c in customers}
    for cid in gt["capacity_bound"]:
        assert cid in customer_ids


def test_compute_ground_truth_optimal_prices_correct():
    config = gen.SimConfig(n_customers=5, n_days=5, seed=42)
    rng = random.Random(42)
    customers = [gen.make_customer(gen.choose_archetype(rng), rng) for _ in range(5)]
    gen.simulate_events(customers, config, rng)
    gt = gen.compute_ground_truth(customers)

    assert gt["optimal_prices"] == gen.OPTIMAL_PRICES


def test_ground_truth_determinism():
    """Same seed always produces the same churner list."""
    def run():
        config = gen.SimConfig(n_customers=50, n_days=90, seed=42)
        rng = random.Random(42)
        customers = [gen.make_customer(gen.choose_archetype(rng), rng) for _ in range(50)]
        gen.simulate_events(customers, config, rng)
        return gen.compute_ground_truth(customers)

    gt1 = run()
    gt2 = run()
    assert gt1["churners"] == gt2["churners"]
    assert gt1["capacity_bound"] == gt2["capacity_bound"]
