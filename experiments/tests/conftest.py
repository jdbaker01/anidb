"""
Shared fixtures for ANIDB experiment tests.

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
# without requiring `pip install -e .`
# ---------------------------------------------------------------------------
_EXPERIMENTS_DIR = Path(__file__).parent.parent

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
_load_module("synthetic_data", _EXPERIMENTS_DIR / "synthetic-data" / "__init__.py")
_load_module("evaluation",     _EXPERIMENTS_DIR / "evaluation" / "__init__.py")

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

SEED = 42


@pytest.fixture
def rng() -> random.Random:
    return random.Random(SEED)


@pytest.fixture
def mini_ground_truth() -> dict:
    return {
        "churners": ["uuid-churn-1", "uuid-churn-2", "uuid-churn-3"],
        "optimal_prices": {"basic": 18, "pro": 45, "enterprise": 199},
        "capacity_bound": ["uuid-cap-1", "uuid-cap-2"],
    }


@pytest.fixture
def perfect_churn_decisions() -> dict:
    """Agent correctly identifies all 3 churners as top 3."""
    return {"flagged_customer_ids": ["uuid-churn-1", "uuid-churn-2", "uuid-churn-3"]}


@pytest.fixture
def partial_churn_decisions() -> dict:
    """Agent gets 2 out of 3 churners, plus 3 false positives."""
    return {
        "flagged_customer_ids": [
            "uuid-churn-1",
            "uuid-wrong-1",
            "uuid-churn-2",
            "uuid-wrong-2",
            "uuid-wrong-3",
        ]
    }


@pytest.fixture
def empty_decisions() -> dict:
    return {"flagged_customer_ids": [], "recommendations": {}}


@pytest.fixture
def perfect_pricing_decisions() -> dict:
    """Agent recommends exact optimal prices."""
    return {"recommendations": {"basic": 18.0, "pro": 45.0, "enterprise": 199.0}}


@pytest.fixture
def off_pricing_decisions() -> dict:
    """Agent is 10% off on all tiers."""
    return {"recommendations": {"basic": 19.8, "pro": 49.5, "enterprise": 218.9}}


@pytest.fixture
def perfect_capacity_decisions() -> dict:
    return {"flagged_customer_ids": ["uuid-cap-1", "uuid-cap-2"]}


@pytest.fixture
def mini_sim_config():
    """A small SimConfig for fast unit tests (no network)."""
    from synthetic_data import generate as gen  # type: ignore[import]
    return gen.SimConfig(n_customers=20, n_days=30, seed=SEED)


@pytest.fixture
def mini_customers(rng, mini_sim_config):
    """20 deterministic customers."""
    from synthetic_data import generate as gen  # type: ignore[import]
    customers = []
    for _ in range(mini_sim_config.n_customers):
        archetype = gen.choose_archetype(rng)
        customers.append(gen.make_customer(archetype, rng))
    return customers


@pytest.fixture
def mini_events(mini_customers, mini_sim_config):
    """Events from the mini simulation (mutates customer state)."""
    import random as _random
    from synthetic_data import generate as gen  # type: ignore[import]
    rng2 = _random.Random(SEED)
    # Re-generate customers fresh so we don't mutate the shared fixture
    customers = []
    for _ in range(mini_sim_config.n_customers):
        archetype = gen.choose_archetype(rng2)
        customers.append(gen.make_customer(archetype, rng2))
    events = gen.simulate_events(customers, mini_sim_config, rng2)
    return events, customers
