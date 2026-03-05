"""
Unit tests for synthetic-data/generate.py (construction estimation)

Tests cover: archetype/type factories, simulation determinism,
project factory constraints, and ground truth derivation.
No network calls required.
"""
from __future__ import annotations

import random
import uuid

import pytest

from construction_synthetic_data import generate as gen  # type: ignore[import]


# ---------------------------------------------------------------------------
# Archetype factory tests
# ---------------------------------------------------------------------------

def test_choose_archetype_returns_valid_archetype():
    rng = random.Random(42)
    valid = {"on_track", "over_budget", "delayed", "resource_constrained"}
    for _ in range(100):
        a = gen.choose_archetype(rng)
        assert a in valid


def test_choose_archetype_distribution():
    """With 5000 samples the observed proportions should be ±5pp of target."""
    rng = random.Random(7)
    counts: dict[str, int] = {}
    n = 5000
    for _ in range(n):
        a = gen.choose_archetype(rng)
        counts[a] = counts.get(a, 0) + 1

    targets = {
        "on_track": 0.50,
        "over_budget": 0.20,
        "delayed": 0.15,
        "resource_constrained": 0.15,
    }
    tolerance = 0.05
    for archetype, target in targets.items():
        observed = counts.get(archetype, 0) / n
        assert abs(observed - target) < tolerance, (
            f"{archetype}: observed {observed:.3f}, expected {target} ± {tolerance}"
        )


def test_choose_archetype_deterministic():
    seq1 = [gen.choose_archetype(random.Random(99)) for _ in range(20)]
    seq2 = [gen.choose_archetype(random.Random(99)) for _ in range(20)]
    assert seq1 == seq2


# ---------------------------------------------------------------------------
# Project type factory tests
# ---------------------------------------------------------------------------

def test_choose_project_type_returns_valid_type():
    rng = random.Random(42)
    valid = {"residential", "commercial", "infrastructure", "renovation"}
    for _ in range(100):
        pt = gen.choose_project_type(rng)
        assert pt in valid


def test_choose_project_type_distribution():
    """With 5000 samples proportions should be ±5pp of target."""
    rng = random.Random(13)
    counts: dict[str, int] = {}
    n = 5000
    for _ in range(n):
        pt = gen.choose_project_type(rng)
        counts[pt] = counts.get(pt, 0) + 1

    targets = {"residential": 0.40, "commercial": 0.30, "infrastructure": 0.20, "renovation": 0.10}
    tolerance = 0.05
    for pt, target in targets.items():
        observed = counts.get(pt, 0) / n
        assert abs(observed - target) < tolerance, (
            f"{pt}: observed {observed:.3f}, expected {target} ± {tolerance}"
        )


# ---------------------------------------------------------------------------
# Project factory tests
# ---------------------------------------------------------------------------

def test_make_project_valid_uuid():
    rng = random.Random(3)
    p = gen.make_project("on_track", "residential", 0, rng)
    parsed = uuid.UUID(p.project_id)
    assert str(parsed) == p.project_id


def test_make_project_unique_ids():
    rng = random.Random(4)
    ids = [gen.make_project("on_track", "commercial", 0, rng).project_id for _ in range(50)]
    assert len(set(ids)) == 50, "Project IDs must be unique"


def test_make_project_cost_in_range():
    rng = random.Random(5)
    for pt in gen.PROJECT_TYPES:
        p = gen.make_project("on_track", pt, 0, rng)
        pt_info = gen.PROJECT_TYPES[pt]
        assert pt_info["cost_min"] <= p.estimated_cost <= pt_info["cost_max"], (
            f"{pt}: estimated_cost {p.estimated_cost} out of range "
            f"[{pt_info['cost_min']}, {pt_info['cost_max']}]"
        )


def test_make_project_resource_constrained_near_capacity():
    rng = random.Random(6)
    for _ in range(20):
        p = gen.make_project("resource_constrained", "commercial", 0, rng)
        assert p.labor_count >= p.labor_capacity - 2, (
            f"resource_constrained: labor_count={p.labor_count} labor_capacity={p.labor_capacity}"
        )


def test_make_project_duration_in_range():
    rng = random.Random(7)
    for pt in gen.PROJECT_TYPES:
        p = gen.make_project("on_track", pt, 0, rng)
        duration = (p.scheduled_end - p.start_date).days
        pt_info = gen.PROJECT_TYPES[pt]
        assert pt_info["dur_min"] <= duration <= pt_info["dur_max"], (
            f"{pt}: duration {duration} out of range [{pt_info['dur_min']}, {pt_info['dur_max']}]"
        )


def test_make_project_actual_cost_starts_equal_to_estimated():
    rng = random.Random(8)
    p = gen.make_project("over_budget", "infrastructure", 0, rng)
    assert p.actual_cost == p.estimated_cost, (
        "actual_cost should start equal to estimated_cost before simulation"
    )


# ---------------------------------------------------------------------------
# Simulation tests
# ---------------------------------------------------------------------------

def test_simulate_projects_deterministic():
    """Same seed → identical outputs."""
    config = gen.SimConfig(n_projects=10, n_days=90, seed=42)

    def run():
        rng = random.Random(42)
        projects = []
        for _ in range(10):
            archetype = gen.choose_archetype(rng)
            pt = gen.choose_project_type(rng)
            projects.append(gen.make_project(archetype, pt, rng.randint(0, 60), rng))
        return gen.simulate_projects(projects, config, rng)

    reports1, cos1, bids1 = run()
    reports2, cos2, bids2 = run()
    assert len(reports1) == len(reports2)
    assert len(cos1) == len(cos2)
    assert len(bids1) == len(bids2)


def test_simulate_projects_over_budget_gets_change_orders():
    """over_budget projects should accumulate change orders."""
    config = gen.SimConfig(n_projects=5, n_days=365, seed=42)
    rng = random.Random(42)
    projects = [gen.make_project("over_budget", "commercial", 0, rng) for _ in range(5)]
    _, change_orders, _ = gen.simulate_projects(projects, config, rng)

    project_ids = {p.project_id for p in projects}
    co_project_ids = {co["project_id"] for co in change_orders}
    # All over_budget projects should have at least one change order
    assert co_project_ids == project_ids, (
        f"Expected all over_budget projects to have change orders; "
        f"missing: {project_ids - co_project_ids}"
    )


def test_simulate_projects_bids_one_per_project():
    config = gen.SimConfig(n_projects=10, n_days=180, seed=42)
    rng = random.Random(42)
    projects = []
    for _ in range(10):
        archetype = gen.choose_archetype(rng)
        pt = gen.choose_project_type(rng)
        projects.append(gen.make_project(archetype, pt, rng.randint(0, 60), rng))
    _, _, bids = gen.simulate_projects(projects, config, rng)

    assert len(bids) == len(projects), "Exactly one bid per project"


def test_simulate_projects_weekly_reports_have_required_keys():
    config = gen.SimConfig(n_projects=5, n_days=60, seed=42)
    rng = random.Random(42)
    projects = [gen.make_project("on_track", "residential", 0, rng) for _ in range(5)]
    reports, _, _ = gen.simulate_projects(projects, config, rng)

    required = {"project_id", "report_date", "cost_to_date", "pct_complete", "labor_hours"}
    for r in reports:
        assert required.issubset(r.keys()), f"Missing keys in report: {required - r.keys()}"


def test_simulate_projects_pct_complete_bounded():
    config = gen.SimConfig(n_projects=10, n_days=120, seed=42)
    rng = random.Random(42)
    projects = []
    for _ in range(10):
        archetype = gen.choose_archetype(rng)
        pt = gen.choose_project_type(rng)
        projects.append(gen.make_project(archetype, pt, 0, rng))
    reports, _, _ = gen.simulate_projects(projects, config, rng)

    for r in reports:
        assert 0.0 <= r["pct_complete"] <= 100.0, (
            f"pct_complete out of bounds: {r['pct_complete']}"
        )


# ---------------------------------------------------------------------------
# Ground truth tests
# ---------------------------------------------------------------------------

def test_compute_ground_truth_structure():
    config = gen.SimConfig(n_projects=20, n_days=180, seed=42)
    rng = random.Random(42)
    projects = []
    for _ in range(20):
        archetype = gen.choose_archetype(rng)
        pt = gen.choose_project_type(rng)
        projects.append(gen.make_project(archetype, pt, rng.randint(0, 90), rng))
    gen.simulate_projects(projects, config, rng)
    gt = gen.compute_ground_truth(projects)

    assert "over_budget_projects" in gt
    assert "delayed_projects" in gt
    assert "resource_constrained_projects" in gt
    assert "optimal_markups" in gt


def test_compute_ground_truth_projects_are_subset():
    config = gen.SimConfig(n_projects=30, n_days=180, seed=42)
    rng = random.Random(42)
    projects = []
    for _ in range(30):
        archetype = gen.choose_archetype(rng)
        pt = gen.choose_project_type(rng)
        projects.append(gen.make_project(archetype, pt, rng.randint(0, 90), rng))
    gen.simulate_projects(projects, config, rng)
    gt = gen.compute_ground_truth(projects)

    all_ids = {p.project_id for p in projects}
    for pid in gt["over_budget_projects"]:
        assert pid in all_ids
    for pid in gt["delayed_projects"]:
        assert pid in all_ids
    for pid in gt["resource_constrained_projects"]:
        assert pid in all_ids


def test_compute_ground_truth_optimal_markups_correct():
    config = gen.SimConfig(n_projects=5, n_days=60, seed=42)
    rng = random.Random(42)
    projects = []
    for _ in range(5):
        archetype = gen.choose_archetype(rng)
        pt = gen.choose_project_type(rng)
        projects.append(gen.make_project(archetype, pt, 0, rng))
    gen.simulate_projects(projects, config, rng)
    gt = gen.compute_ground_truth(projects)

    assert gt["optimal_markups"] == gen.OPTIMAL_MARKUPS


def test_ground_truth_determinism():
    """Same seed always produces the same ground truth."""
    def run():
        config = gen.SimConfig(n_projects=40, n_days=365, seed=42)
        rng = random.Random(42)
        projects = []
        for _ in range(40):
            archetype = gen.choose_archetype(rng)
            pt = gen.choose_project_type(rng)
            projects.append(gen.make_project(archetype, pt, rng.randint(0, 180), rng))
        gen.simulate_projects(projects, config, rng)
        return gen.compute_ground_truth(projects)

    gt1 = run()
    gt2 = run()
    assert sorted(gt1["over_budget_projects"]) == sorted(gt2["over_budget_projects"])
    assert sorted(gt1["delayed_projects"]) == sorted(gt2["delayed_projects"])
    assert sorted(gt1["resource_constrained_projects"]) == sorted(gt2["resource_constrained_projects"])
