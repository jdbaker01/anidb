"""
Construction Estimation Evaluation Framework

Scores both agents against pre-defined ground truth outcomes.
  - Overrun:   precision, recall vs over_budget_projects
  - Delay:     precision, recall vs delayed_projects
  - Resource:  precision, recall vs resource_constrained_projects
  - Bid:       pct_error per project type vs optimal_markups

Usage:
    python evaluation/compare.py [--output results/] [--data-dir data/]
"""
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass, asdict
from datetime import datetime, timezone
from pathlib import Path


# ---------------------------------------------------------------------------
# Metric dataclasses
# ---------------------------------------------------------------------------

@dataclass
class OverrunMetrics:
    precision: float
    recall: float
    n_flagged: int
    n_actual: int
    n_correct: int


@dataclass
class DelayMetrics:
    precision: float
    recall: float
    n_flagged: int
    n_actual: int
    n_correct: int


@dataclass
class ResourceMetrics:
    precision: float
    recall: float
    n_flagged: int
    n_actual: int
    n_correct: int


@dataclass
class BidMetrics:
    pct_error_residential: float | None
    pct_error_commercial: float | None
    pct_error_infrastructure: float | None
    pct_error_renovation: float | None
    weighted_avg_error: float | None


# ---------------------------------------------------------------------------
# Scoring functions
# ---------------------------------------------------------------------------

def score_overrun(decisions: dict, ground_truth: dict) -> OverrunMetrics:
    actual = set(ground_truth.get("over_budget_projects", []))
    flagged = set(decisions.get("flagged_project_ids", []))

    n_correct = len(flagged & actual)
    n_flagged = len(flagged)
    n_actual = len(actual)

    precision = n_correct / n_flagged if n_flagged > 0 else 0.0
    recall = n_correct / n_actual if n_actual > 0 else 0.0

    return OverrunMetrics(
        precision=precision,
        recall=recall,
        n_flagged=n_flagged,
        n_actual=n_actual,
        n_correct=n_correct,
    )


def score_delay(decisions: dict, ground_truth: dict) -> DelayMetrics:
    actual = set(ground_truth.get("delayed_projects", []))
    flagged = set(decisions.get("flagged_project_ids", []))

    n_correct = len(flagged & actual)
    n_flagged = len(flagged)
    n_actual = len(actual)

    precision = n_correct / n_flagged if n_flagged > 0 else 0.0
    recall = n_correct / n_actual if n_actual > 0 else 0.0

    return DelayMetrics(
        precision=precision,
        recall=recall,
        n_flagged=n_flagged,
        n_actual=n_actual,
        n_correct=n_correct,
    )


def score_resource(decisions: dict, ground_truth: dict) -> ResourceMetrics:
    actual = set(ground_truth.get("resource_constrained_projects", []))
    flagged = set(decisions.get("flagged_project_ids", []))

    n_correct = len(flagged & actual)
    n_flagged = len(flagged)
    n_actual = len(actual)

    precision = n_correct / n_flagged if n_flagged > 0 else 0.0
    recall = n_correct / n_actual if n_actual > 0 else 0.0

    return ResourceMetrics(
        precision=precision,
        recall=recall,
        n_flagged=n_flagged,
        n_actual=n_actual,
        n_correct=n_correct,
    )


def score_bid(decisions: dict, ground_truth: dict) -> BidMetrics:
    optimal = ground_truth.get("optimal_markups", {})
    recs = decisions.get("recommendations", {})

    def pct_err(pt: str) -> float | None:
        if pt not in recs or pt not in optimal:
            return None
        try:
            rec = float(recs[pt])
            opt = float(optimal[pt])
        except (TypeError, ValueError):
            return None
        return abs(rec - opt) / opt if opt != 0 else None

    errors = [
        e for e in (
            pct_err("residential"),
            pct_err("commercial"),
            pct_err("infrastructure"),
            pct_err("renovation"),
        )
        if e is not None
    ]
    avg = sum(errors) / len(errors) if errors else None

    return BidMetrics(
        pct_error_residential=pct_err("residential"),
        pct_error_commercial=pct_err("commercial"),
        pct_error_infrastructure=pct_err("infrastructure"),
        pct_error_renovation=pct_err("renovation"),
        weighted_avg_error=avg,
    )


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

def comparison_table(baseline_scores: dict, anidb_scores: dict) -> str:
    lines = []
    W = 72

    def header(title: str) -> None:
        lines.append(f"\n{title}")
        lines.append(f"  {'Metric':<30} {'Baseline':>12} {'ANIDB':>12}")
        lines.append(f"  {'-' * 54}")

    def row(label: str, b_val, a_val, fmt: str = ".1%") -> None:
        b_str = format(b_val, fmt) if b_val is not None else "N/A"
        a_str = format(a_val, fmt) if a_val is not None else "N/A"
        lines.append(f"  {label:<30} {b_str:>12} {a_str:>12}")

    lines.append("=" * W)
    lines.append("ANIDB vs SQL Baseline — Construction Estimation Decision Quality")
    lines.append("=" * W)

    bo, ao = baseline_scores["overrun"], anidb_scores["overrun"]
    header("COST OVERRUN RISK")
    row("Precision",          bo.precision, ao.precision)
    row("Recall",             bo.recall,    ao.recall)
    row("Projects Flagged",   bo.n_flagged, ao.n_flagged, fmt="d")
    row("Correct Flags",      bo.n_correct, ao.n_correct, fmt="d")

    bd, ad = baseline_scores["delay"], anidb_scores["delay"]
    header("SCHEDULE DELAY RISK")
    row("Precision",          bd.precision, ad.precision)
    row("Recall",             bd.recall,    ad.recall)
    row("Projects Flagged",   bd.n_flagged, ad.n_flagged, fmt="d")
    row("Correct Flags",      bd.n_correct, ad.n_correct, fmt="d")

    br, ar = baseline_scores["resource"], anidb_scores["resource"]
    header("RESOURCE BOTTLENECK")
    row("Precision",          br.precision, ar.precision)
    row("Recall",             br.recall,    ar.recall)
    row("Projects Flagged",   br.n_flagged, ar.n_flagged, fmt="d")
    row("Correct Flags",      br.n_correct, ar.n_correct, fmt="d")

    bb, ab = baseline_scores["bid"], anidb_scores["bid"]
    header("BID ACCURACY (% error — lower is better)")
    row("Residential error",   bb.pct_error_residential,   ab.pct_error_residential)
    row("Commercial error",    bb.pct_error_commercial,    ab.pct_error_commercial)
    row("Infrastructure error",bb.pct_error_infrastructure,ab.pct_error_infrastructure)
    row("Renovation error",    bb.pct_error_renovation,    ab.pct_error_renovation)
    row("Weighted avg error",  bb.weighted_avg_error,      ab.weighted_avg_error)

    lines.append("\n" + "=" * W)
    return "\n".join(lines)


def generate_charts(baseline_scores: dict, anidb_scores: dict, output_dir: Path) -> None:
    try:
        import matplotlib.pyplot as plt
    except ImportError:
        print("matplotlib not installed; skipping charts (pip install matplotlib)")
        return

    charts_dir = output_dir / "charts"
    charts_dir.mkdir(parents=True, exist_ok=True)

    blue, orange = "#5B8DB8", "#F4A261"

    def bar_chart(
        title: str,
        metric_labels: list[str],
        baseline_vals: list[float],
        anidb_vals: list[float],
        filename: str,
        ylabel: str = "Score",
        ylim: tuple | None = (0, 1.0),
    ) -> None:
        fig, ax = plt.subplots(figsize=(8, 5))
        x = range(len(metric_labels))
        ax.bar([i - 0.2 for i in x], baseline_vals, width=0.4, label="Baseline (SQL)", color=blue)
        ax.bar([i + 0.2 for i in x], anidb_vals,    width=0.4, label="ANIDB",          color=orange)
        ax.set_xticks(list(x))
        ax.set_xticklabels(metric_labels)
        if ylim:
            ax.set_ylim(*ylim)
        ax.set_ylabel(ylabel)
        ax.set_title(title)
        ax.legend()
        plt.tight_layout()
        plt.savefig(charts_dir / filename, dpi=150)
        plt.close()

    bo, ao = baseline_scores["overrun"], anidb_scores["overrun"]
    bar_chart(
        "Cost Overrun Risk: Baseline vs ANIDB",
        ["Precision", "Recall"],
        [bo.precision, bo.recall],
        [ao.precision, ao.recall],
        "construction_overrun_comparison.png",
    )

    bd, ad = baseline_scores["delay"], anidb_scores["delay"]
    bar_chart(
        "Schedule Delay Risk: Baseline vs ANIDB",
        ["Precision", "Recall"],
        [bd.precision, bd.recall],
        [ad.precision, ad.recall],
        "construction_delay_comparison.png",
    )

    br, ar = baseline_scores["resource"], anidb_scores["resource"]
    bar_chart(
        "Resource Bottleneck: Baseline vs ANIDB",
        ["Precision", "Recall"],
        [br.precision, br.recall],
        [ar.precision, ar.recall],
        "construction_resource_comparison.png",
    )

    bb, ab = baseline_scores["bid"], anidb_scores["bid"]
    bar_chart(
        "Bid Accuracy: Price Error by Project Type",
        ["Residential", "Commercial", "Infrastructure", "Renovation"],
        [bb.pct_error_residential or 0, bb.pct_error_commercial or 0,
         bb.pct_error_infrastructure or 0, bb.pct_error_renovation or 0],
        [ab.pct_error_residential or 0, ab.pct_error_commercial or 0,
         ab.pct_error_infrastructure or 0, ab.pct_error_renovation or 0],
        "construction_bid_comparison.png",
        ylabel="% Error (lower is better)",
        ylim=None,
    )

    print(f"Charts written to {charts_dir}/")


def generate_report(
    baseline_scores: dict, anidb_scores: dict, output_dir: Path
) -> dict:
    def _dc(scores: dict) -> dict:
        return {
            "overrun":  asdict(scores["overrun"]),
            "delay":    asdict(scores["delay"]),
            "resource": asdict(scores["resource"]),
            "bid":      asdict(scores["bid"]),
        }

    report = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "experiment": "construction-estimation",
        "baseline": _dc(baseline_scores),
        "anidb": _dc(anidb_scores),
    }

    output_dir.mkdir(parents=True, exist_ok=True)
    report_path = output_dir / "construction_report.json"
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2, default=str)
    print(f"Report written to {report_path}")
    return report


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compare construction estimation ANIDB vs baseline agent performance"
    )
    parser.add_argument("--output",   default="results")
    parser.add_argument("--data-dir", default="data")
    args = parser.parse_args()

    output_dir = Path(args.output)
    data_dir = Path(args.data_dir)

    gt_path = data_dir / "construction_ground_truth.json"
    if not gt_path.exists():
        print(f"ERROR: Ground truth not found at {gt_path}")
        print("Run synthetic-data/generate.py first.")
        return

    baseline_path = output_dir / "construction_baseline_decisions.json"
    anidb_path = output_dir / "construction_anidb_decisions.json"

    for path, label in [(baseline_path, "Baseline"), (anidb_path, "ANIDB")]:
        if not path.exists():
            script = "baseline-agent/run.py" if label == "Baseline" else "anidb-agent/run.py"
            print(f"ERROR: {label} decisions not found at {path}")
            print(f"Run {script} first.")
            return

    with open(gt_path) as f:
        ground_truth = json.load(f)
    with open(baseline_path) as f:
        baseline_data = json.load(f)
    with open(anidb_path) as f:
        anidb_data = json.load(f)

    bd = baseline_data.get("decisions", {})
    ad = anidb_data.get("decisions", {})

    baseline_scores = {
        "overrun":  score_overrun(bd.get("overrun", {}),   ground_truth),
        "delay":    score_delay(bd.get("delay", {}),       ground_truth),
        "resource": score_resource(bd.get("resource", {}), ground_truth),
        "bid":      score_bid(bd.get("bid", {}),           ground_truth),
    }
    anidb_scores = {
        "overrun":  score_overrun(ad.get("overrun", {}),   ground_truth),
        "delay":    score_delay(ad.get("delay", {}),       ground_truth),
        "resource": score_resource(ad.get("resource", {}), ground_truth),
        "bid":      score_bid(ad.get("bid", {}),           ground_truth),
    }

    print(comparison_table(baseline_scores, anidb_scores))
    generate_charts(baseline_scores, anidb_scores, output_dir)
    generate_report(baseline_scores, anidb_scores, output_dir)


if __name__ == "__main__":
    main()
