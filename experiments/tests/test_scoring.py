"""
Unit tests for evaluation/compare.py scoring functions.

Tests cover all three decision classes with perfect, partial, and empty decisions.
No network calls required.
"""
from __future__ import annotations

import pytest

from evaluation.compare import (  # type: ignore[import]
    ChurnMetrics,
    CapacityMetrics,
    PricingMetrics,
    comparison_table,
    score_capacity,
    score_churn,
    score_pricing,
)


# ---------------------------------------------------------------------------
# Churn scoring
# ---------------------------------------------------------------------------

def test_score_churn_perfect(mini_ground_truth, perfect_churn_decisions):
    m = score_churn(perfect_churn_decisions, mini_ground_truth)
    assert m.precision_at_10 == pytest.approx(1.0)
    assert m.recall_at_10 == pytest.approx(1.0)
    assert m.n_correct == 3
    assert m.n_actual == 3


def test_score_churn_empty(mini_ground_truth, empty_decisions):
    m = score_churn(empty_decisions, mini_ground_truth)
    assert m.precision_at_10 == pytest.approx(0.0)
    assert m.recall_at_10 == pytest.approx(0.0)
    assert m.n_flagged == 0
    assert m.n_correct == 0


def test_score_churn_partial(mini_ground_truth, partial_churn_decisions):
    m = score_churn(partial_churn_decisions, mini_ground_truth)
    # 5 flagged, 2 correct
    assert m.n_flagged == 5
    assert m.n_correct == 2
    assert m.precision_at_10 == pytest.approx(2 / 5)
    assert m.recall_at_10 == pytest.approx(2 / 3)


def test_score_churn_top_10_limit(mini_ground_truth):
    """Only the first 10 flagged customers are scored."""
    # 15 flagged, but only first 10 are considered
    # churners: uuid-churn-1, uuid-churn-2, uuid-churn-3
    # Put all 3 churners after position 10 — should not be counted
    flagged = [f"uuid-wrong-{i}" for i in range(10)] + [
        "uuid-churn-1", "uuid-churn-2", "uuid-churn-3"
    ]
    decisions = {"flagged_customer_ids": flagged}
    m = score_churn(decisions, mini_ground_truth)
    assert m.n_correct == 0
    assert m.recall_at_10 == pytest.approx(0.0)


def test_score_churn_no_actual_churners():
    gt = {"churners": [], "optimal_prices": {}, "capacity_bound": []}
    decisions = {"flagged_customer_ids": ["uuid-1"]}
    m = score_churn(decisions, gt)
    assert m.recall_at_10 == pytest.approx(0.0)
    assert m.n_actual == 0


# ---------------------------------------------------------------------------
# Pricing scoring
# ---------------------------------------------------------------------------

def test_score_pricing_perfect(mini_ground_truth, perfect_pricing_decisions):
    m = score_pricing(perfect_pricing_decisions, mini_ground_truth)
    assert m.pct_error_basic == pytest.approx(0.0)
    assert m.pct_error_pro == pytest.approx(0.0)
    assert m.pct_error_enterprise == pytest.approx(0.0)
    assert m.weighted_avg_error == pytest.approx(0.0)


def test_score_pricing_off_by_10pct(mini_ground_truth, off_pricing_decisions):
    m = score_pricing(off_pricing_decisions, mini_ground_truth)
    # basic: |19.8 - 18| / 18 = 0.10
    assert m.pct_error_basic == pytest.approx(0.10, abs=0.01)


def test_score_pricing_missing_tier(mini_ground_truth):
    """Missing tier → None error, excluded from average."""
    decisions = {"recommendations": {"basic": 18.0}}  # only basic
    m = score_pricing(decisions, mini_ground_truth)
    assert m.pct_error_basic == pytest.approx(0.0)
    assert m.pct_error_pro is None
    assert m.pct_error_enterprise is None
    assert m.weighted_avg_error == pytest.approx(0.0)


def test_score_pricing_empty_recommendations(mini_ground_truth, empty_decisions):
    m = score_pricing(empty_decisions, mini_ground_truth)
    assert m.pct_error_basic is None
    assert m.pct_error_pro is None
    assert m.pct_error_enterprise is None
    assert m.weighted_avg_error is None


# ---------------------------------------------------------------------------
# Capacity scoring
# ---------------------------------------------------------------------------

def test_score_capacity_perfect(mini_ground_truth, perfect_capacity_decisions):
    m = score_capacity(perfect_capacity_decisions, mini_ground_truth)
    assert m.precision == pytest.approx(1.0)
    assert m.recall == pytest.approx(1.0)
    assert m.n_correct == 2
    assert m.n_actual == 2


def test_score_capacity_empty(mini_ground_truth, empty_decisions):
    m = score_capacity(empty_decisions, mini_ground_truth)
    assert m.precision == pytest.approx(0.0)
    assert m.recall == pytest.approx(0.0)
    assert m.n_flagged == 0


def test_score_capacity_partial(mini_ground_truth):
    decisions = {"flagged_customer_ids": ["uuid-cap-1", "uuid-wrong-1"]}
    m = score_capacity(decisions, mini_ground_truth)
    # 2 flagged, 1 correct
    assert m.n_correct == 1
    assert m.precision == pytest.approx(0.5)
    assert m.recall == pytest.approx(0.5)


def test_score_capacity_no_actual():
    gt = {"churners": [], "optimal_prices": {}, "capacity_bound": []}
    decisions = {"flagged_customer_ids": ["uuid-1"]}
    m = score_capacity(decisions, gt)
    assert m.recall == pytest.approx(0.0)
    assert m.n_actual == 0


# ---------------------------------------------------------------------------
# Comparison table
# ---------------------------------------------------------------------------

def test_comparison_table_contains_section_headers(mini_ground_truth):
    scores = {
        "churn": ChurnMetrics(
            precision_at_10=0.8, recall_at_10=0.6,
            n_flagged=10, n_actual=5, n_correct=4,
        ),
        "pricing": PricingMetrics(
            pct_error_basic=0.05, pct_error_pro=0.10,
            pct_error_enterprise=0.02, weighted_avg_error=0.057,
        ),
        "capacity": CapacityMetrics(
            precision=0.9, recall=0.7,
            n_flagged=10, n_actual=8, n_correct=7,
        ),
    }
    table = comparison_table(scores, scores)
    assert "CHURN INTERVENTION" in table
    assert "PRICING OPTIMIZATION" in table
    assert "CAPACITY" in table
    assert "Baseline" in table
    assert "ANIDB" in table


def test_comparison_table_shows_percentages(mini_ground_truth):
    scores = {
        "churn": ChurnMetrics(
            precision_at_10=1.0, recall_at_10=1.0,
            n_flagged=3, n_actual=3, n_correct=3,
        ),
        "pricing": PricingMetrics(
            pct_error_basic=0.0, pct_error_pro=0.0,
            pct_error_enterprise=0.0, weighted_avg_error=0.0,
        ),
        "capacity": CapacityMetrics(
            precision=1.0, recall=1.0,
            n_flagged=2, n_actual=2, n_correct=2,
        ),
    }
    table = comparison_table(scores, scores)
    # Perfect scores → 100.0% in table
    assert "100.0%" in table
