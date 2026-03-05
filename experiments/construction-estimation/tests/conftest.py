"""
Shared fixtures for construction-estimation experiment tests.

All fixtures are deterministic (seed=42) and require no network access.
"""
from __future__ import annotations

import importlib.util
import random
import sys
from pathlib import Path

import pytest

# ---------------------------------------------------------------------------
# Path setup: allow importing from synthetic-data/, evaluation/ etc.
# ---------------------------------------------------------------------------
_EXPERIMENT_DIR = Path(__file__).parent.parent


def _load_module(pkg_name: str, file_path: Path):
    """Load a module from a file path and register it under pkg_name."""
    if pkg_name in sys.modules:
        return sys.modules[pkg_name]
    spec = importlib.util.spec_from_file_location(pkg_name, file_path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[pkg_name] = mod
    spec.loader.exec_module(mod)
    return mod


# Pre-register packages so cross-module imports resolve
_load_module(
    "construction_synthetic_data",
    _EXPERIMENT_DIR / "synthetic-data" / "__init__.py",
)
_load_module(
    "construction_evaluation",
    _EXPERIMENT_DIR / "evaluation" / "__init__.py",
)

SEED = 42


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def rng() -> random.Random:
    return random.Random(SEED)


@pytest.fixture
def mini_ground_truth() -> dict:
    return {
        "over_budget_projects":         ["uuid-ob-1", "uuid-ob-2", "uuid-ob-3"],
        "delayed_projects":             ["uuid-dl-1", "uuid-dl-2"],
        "resource_constrained_projects":["uuid-rc-1", "uuid-rc-2"],
        "optimal_markups": {
            "residential":    0.18,
            "commercial":     0.22,
            "infrastructure": 0.28,
            "renovation":     0.15,
        },
    }


@pytest.fixture
def perfect_overrun_decisions() -> dict:
    return {"flagged_project_ids": ["uuid-ob-1", "uuid-ob-2", "uuid-ob-3"]}


@pytest.fixture
def partial_overrun_decisions() -> dict:
    """Agent gets 2 out of 3 over-budget projects, plus 2 false positives."""
    return {
        "flagged_project_ids": [
            "uuid-ob-1",
            "uuid-wrong-1",
            "uuid-ob-2",
            "uuid-wrong-2",
        ]
    }


@pytest.fixture
def perfect_delay_decisions() -> dict:
    return {"flagged_project_ids": ["uuid-dl-1", "uuid-dl-2"]}


@pytest.fixture
def perfect_resource_decisions() -> dict:
    return {"flagged_project_ids": ["uuid-rc-1", "uuid-rc-2"]}


@pytest.fixture
def perfect_bid_decisions() -> dict:
    return {
        "recommendations": {
            "residential":    0.18,
            "commercial":     0.22,
            "infrastructure": 0.28,
            "renovation":     0.15,
        }
    }


@pytest.fixture
def off_bid_decisions() -> dict:
    """Agent is ~10% off on all types."""
    return {
        "recommendations": {
            "residential":    0.198,
            "commercial":     0.242,
            "infrastructure": 0.308,
            "renovation":     0.165,
        }
    }


@pytest.fixture
def empty_decisions() -> dict:
    return {"flagged_project_ids": [], "recommendations": {}}


@pytest.fixture
def mini_sim_config():
    """A small SimConfig for fast unit tests (no network)."""
    from construction_synthetic_data import generate as gen  # type: ignore[import]
    return gen.SimConfig(n_projects=20, n_days=180, seed=SEED)


@pytest.fixture
def mini_projects(rng, mini_sim_config):
    """20 deterministic projects (no simulation run yet)."""
    from construction_synthetic_data import generate as gen  # type: ignore[import]
    projects = []
    for _ in range(mini_sim_config.n_projects):
        archetype = gen.choose_archetype(rng)
        project_type = gen.choose_project_type(rng)
        start_offset = rng.randint(0, 90)
        projects.append(gen.make_project(archetype, project_type, start_offset, rng))
    return projects


@pytest.fixture
def mini_simulation(mini_projects, mini_sim_config):
    """Weekly reports, change orders, and bids from the mini simulation."""
    import random as _random
    from construction_synthetic_data import generate as gen  # type: ignore[import]

    rng2 = _random.Random(SEED)
    # Regenerate projects fresh so mutation doesn't affect other fixtures
    fresh_projects = []
    for _ in range(mini_sim_config.n_projects):
        archetype = gen.choose_archetype(rng2)
        project_type = gen.choose_project_type(rng2)
        start_offset = rng2.randint(0, 90)
        fresh_projects.append(gen.make_project(archetype, project_type, start_offset, rng2))

    weekly_reports, change_orders, bids = gen.simulate_projects(fresh_projects, mini_sim_config, rng2)
    return fresh_projects, weekly_reports, change_orders, bids
