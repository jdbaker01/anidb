"""
Unit tests for evaluation/compare.py scoring functions (construction estimation).

Tests cover all four decision classes with perfect, partial, and empty decisions.
No network calls required.
"""
from __future__ import annotations

import pytest

from construction_evaluation import compare as cmp  # type: ignore[import]


# ---------------------------------------------------------------------------
# Overrun scoring
# ---------------------------------------------------------------------------

def test_score_overrun_perfect(mini_ground_truth, perfect_overrun_decisions):
    m = cmp.score_overrun(perfect_overrun_decisions, mini_ground_truth)
    assert m.precision == pytest.approx(1.0)
    assert m.recall == pytest.approx(1.0)
    assert m.n_correct == 3
    assert m.n_actual == 3


def test_score_overrun_empty(mini_ground_truth, empty_decisions):
    m = cmp.score_overrun(empty_decisions, mini_ground_truth)
    assert m.precision == pytest.approx(0.0)
    assert m.recall == pytest.approx(0.0)
    assert m.n_flagged == 0
    assert m.n_correct == 0


def test_score_overrun_partial(mini_ground_truth, partial_overrun_decisions):
    m = cmp.score_overrun(partial_overrun_decisions, mini_ground_truth)
    # 4 flagged, 2 correct (uuid-ob-1, uuid-ob-2)
    assert m.n_flagged == 4
    assert m.n_correct == 2
    assert m.precision == pytest.approx(2 / 4)
    assert m.recall == pytest.approx(2 / 3)


def test_score_overrun_no_actual():
    gt = {"over_budget_projects": [], "delayed_projects": [], "resource_constrained_projects": [], "optimal_markups": {}}
    decisions = {"flagged_project_ids": ["uuid-1"]}
    m = cmp.score_overrun(decisions, gt)
    assert m.recall == pytest.approx(0.0)
    assert m.n_actual == 0


def test_score_overrun_false_positives_only(mini_ground_truth):
    decisions = {"flagged_project_ids": ["uuid-wrong-1", "uuid-wrong-2"]}
    m = cmp.score_overrun(decisions, mini_ground_truth)
    assert m.n_correct == 0
    assert m.precision == pytest.approx(0.0)
    assert m.recall == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Delay scoring
# ---------------------------------------------------------------------------

def test_score_delay_perfect(mini_ground_truth, perfect_delay_decisions):
    m = cmp.score_delay(perfect_delay_decisions, mini_ground_truth)
    assert m.precision == pytest.approx(1.0)
    assert m.recall == pytest.approx(1.0)
    assert m.n_correct == 2
    assert m.n_actual == 2


def test_score_delay_empty(mini_ground_truth, empty_decisions):
    m = cmp.score_delay(empty_decisions, mini_ground_truth)
    assert m.precision == pytest.approx(0.0)
    assert m.recall == pytest.approx(0.0)
    assert m.n_flagged == 0


def test_score_delay_partial(mini_ground_truth):
    decisions = {"flagged_project_ids": ["uuid-dl-1", "uuid-wrong-1", "uuid-wrong-2"]}
    m = cmp.score_delay(decisions, mini_ground_truth)
    assert m.n_correct == 1
    assert m.n_flagged == 3
    assert m.precision == pytest.approx(1 / 3)
    assert m.recall == pytest.approx(1 / 2)


def test_score_delay_no_actual():
    gt = {"over_budget_projects": [], "delayed_projects": [], "resource_constrained_projects": [], "optimal_markups": {}}
    decisions = {"flagged_project_ids": ["uuid-1"]}
    m = cmp.score_delay(decisions, gt)
    assert m.recall == pytest.approx(0.0)
    assert m.n_actual == 0


# ---------------------------------------------------------------------------
# Resource scoring
# ---------------------------------------------------------------------------

def test_score_resource_perfect(mini_ground_truth, perfect_resource_decisions):
    m = cmp.score_resource(perfect_resource_decisions, mini_ground_truth)
    assert m.precision == pytest.approx(1.0)
    assert m.recall == pytest.approx(1.0)
    assert m.n_correct == 2
    assert m.n_actual == 2


def test_score_resource_empty(mini_ground_truth, empty_decisions):
    m = cmp.score_resource(empty_decisions, mini_ground_truth)
    assert m.precision == pytest.approx(0.0)
    assert m.recall == pytest.approx(0.0)
    assert m.n_flagged == 0


def test_score_resource_partial(mini_ground_truth):
    decisions = {"flagged_project_ids": ["uuid-rc-1", "uuid-wrong-1"]}
    m = cmp.score_resource(decisions, mini_ground_truth)
    assert m.n_correct == 1
    assert m.precision == pytest.approx(0.5)
    assert m.recall == pytest.approx(0.5)


def test_score_resource_no_actual():
    gt = {"over_budget_projects": [], "delayed_projects": [], "resource_constrained_projects": [], "optimal_markups": {}}
    decisions = {"flagged_project_ids": ["uuid-1"]}
    m = cmp.score_resource(decisions, gt)
    assert m.recall == pytest.approx(0.0)
    assert m.n_actual == 0


# ---------------------------------------------------------------------------
# Bid scoring
# ---------------------------------------------------------------------------

def test_score_bid_perfect(mini_ground_truth, perfect_bid_decisions):
    m = cmp.score_bid(perfect_bid_decisions, mini_ground_truth)
    assert m.pct_error_residential == pytest.approx(0.0)
    assert m.pct_error_commercial == pytest.approx(0.0)
    assert m.pct_error_infrastructure == pytest.approx(0.0)
    assert m.pct_error_renovation == pytest.approx(0.0)
    assert m.weighted_avg_error == pytest.approx(0.0)


def test_score_bid_off_by_10pct(mini_ground_truth, off_bid_decisions):
    m = cmp.score_bid(off_bid_decisions, mini_ground_truth)
    # residential: |0.198 - 0.18| / 0.18 = 0.10
    assert m.pct_error_residential == pytest.approx(0.10, abs=0.01)


def test_score_bid_missing_type(mini_ground_truth):
    """Missing type → None error, excluded from average."""
    decisions = {"recommendations": {"residential": 0.18, "commercial": 0.22}}
    m = cmp.score_bid(decisions, mini_ground_truth)
    assert m.pct_error_residential == pytest.approx(0.0)
    assert m.pct_error_commercial == pytest.approx(0.0)
    assert m.pct_error_infrastructure is None
    assert m.pct_error_renovation is None
    # avg only over 2 available types
    assert m.weighted_avg_error == pytest.approx(0.0)


def test_score_bid_empty_recommendations(mini_ground_truth, empty_decisions):
    m = cmp.score_bid(empty_decisions, mini_ground_truth)
    assert m.pct_error_residential is None
    assert m.pct_error_commercial is None
    assert m.pct_error_infrastructure is None
    assert m.pct_error_renovation is None
    assert m.weighted_avg_error is None


# ---------------------------------------------------------------------------
# Comparison table
# ---------------------------------------------------------------------------

def test_comparison_table_contains_section_headers(mini_ground_truth):
    scores = {
        "overrun": cmp.OverrunMetrics(
            precision=0.8, recall=0.6, n_flagged=10, n_actual=8, n_correct=6,
        ),
        "delay": cmp.DelayMetrics(
            precision=0.7, recall=0.5, n_flagged=8, n_actual=6, n_correct=4,
        ),
        "resource": cmp.ResourceMetrics(
            precision=0.9, recall=0.8, n_flagged=5, n_actual=4, n_correct=4,
        ),
        "bid": cmp.BidMetrics(
            pct_error_residential=0.05, pct_error_commercial=0.10,
            pct_error_infrastructure=0.02, pct_error_renovation=0.08,
            weighted_avg_error=0.0625,
        ),
    }
    table = cmp.comparison_table(scores, scores)
    assert "COST OVERRUN RISK" in table
    assert "SCHEDULE DELAY RISK" in table
    assert "RESOURCE BOTTLENECK" in table
    assert "BID ACCURACY" in table
    assert "Baseline" in table
    assert "ANIDB" in table


def test_comparison_table_shows_percentages(mini_ground_truth):
    scores = {
        "overrun": cmp.OverrunMetrics(
            precision=1.0, recall=1.0, n_flagged=3, n_actual=3, n_correct=3,
        ),
        "delay": cmp.DelayMetrics(
            precision=1.0, recall=1.0, n_flagged=2, n_actual=2, n_correct=2,
        ),
        "resource": cmp.ResourceMetrics(
            precision=1.0, recall=1.0, n_flagged=2, n_actual=2, n_correct=2,
        ),
        "bid": cmp.BidMetrics(
            pct_error_residential=0.0, pct_error_commercial=0.0,
            pct_error_infrastructure=0.0, pct_error_renovation=0.0,
            weighted_avg_error=0.0,
        ),
    }
    table = cmp.comparison_table(scores, scores)
    assert "100.0%" in table


def test_comparison_table_shows_na_for_missing(mini_ground_truth):
    scores = {
        "overrun": cmp.OverrunMetrics(
            precision=0.0, recall=0.0, n_flagged=0, n_actual=0, n_correct=0,
        ),
        "delay": cmp.DelayMetrics(
            precision=0.0, recall=0.0, n_flagged=0, n_actual=0, n_correct=0,
        ),
        "resource": cmp.ResourceMetrics(
            precision=0.0, recall=0.0, n_flagged=0, n_actual=0, n_correct=0,
        ),
        "bid": cmp.BidMetrics(
            pct_error_residential=None, pct_error_commercial=None,
            pct_error_infrastructure=None, pct_error_renovation=None,
            weighted_avg_error=None,
        ),
    }
    table = cmp.comparison_table(scores, scores)
    assert "N/A" in table
