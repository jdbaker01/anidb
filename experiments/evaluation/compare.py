"""
ANIDB Evaluation Framework

Scores both agents against pre-defined ground truth outcomes.
  - Churn:   precision@10, recall@10
  - Pricing: % error per tier, weighted average error
  - Capacity: precision, recall

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
class ChurnMetrics:
    precision_at_10: float
    recall_at_10: float
    n_flagged: int
    n_actual: int
    n_correct: int


@dataclass
class PricingMetrics:
    pct_error_basic: float | None
    pct_error_pro: float | None
    pct_error_enterprise: float | None
    weighted_avg_error: float | None


@dataclass
class CapacityMetrics:
    precision: float
    recall: float
    n_flagged: int
    n_actual: int
    n_correct: int


# ---------------------------------------------------------------------------
# Scoring functions
# ---------------------------------------------------------------------------

def score_churn(decisions: dict, ground_truth: dict) -> ChurnMetrics:
    actual_churners = set(ground_truth.get("churners", []))
    flagged = decisions.get("flagged_customer_ids", [])[:10]

    n_correct = len(set(flagged) & actual_churners)
    n_flagged = len(flagged)
    n_actual = len(actual_churners)

    precision = n_correct / n_flagged if n_flagged > 0 else 0.0
    recall = n_correct / n_actual if n_actual > 0 else 0.0

    return ChurnMetrics(
        precision_at_10=precision,
        recall_at_10=recall,
        n_flagged=n_flagged,
        n_actual=n_actual,
        n_correct=n_correct,
    )


def score_pricing(decisions: dict, ground_truth: dict) -> PricingMetrics:
    optimal = ground_truth.get("optimal_prices", {})
    recs = decisions.get("recommendations", {})

    def pct_err(tier: str) -> float | None:
        if tier not in recs or tier not in optimal:
            return None
        try:
            rec = float(recs[tier])
            opt = float(optimal[tier])
        except (TypeError, ValueError):
            return None
        return abs(rec - opt) / opt if opt != 0 else None

    errors = [
        e for e in (pct_err("basic"), pct_err("pro"), pct_err("enterprise"))
        if e is not None
    ]
    avg = sum(errors) / len(errors) if errors else None

    return PricingMetrics(
        pct_error_basic=pct_err("basic"),
        pct_error_pro=pct_err("pro"),
        pct_error_enterprise=pct_err("enterprise"),
        weighted_avg_error=avg,
    )


def score_capacity(decisions: dict, ground_truth: dict) -> CapacityMetrics:
    actual = set(ground_truth.get("capacity_bound", []))
    flagged = set(decisions.get("flagged_customer_ids", []))

    n_correct = len(flagged & actual)
    n_flagged = len(flagged)
    n_actual = len(actual)

    precision = n_correct / n_flagged if n_flagged > 0 else 0.0
    recall = n_correct / n_actual if n_actual > 0 else 0.0

    return CapacityMetrics(
        precision=precision,
        recall=recall,
        n_flagged=n_flagged,
        n_actual=n_actual,
        n_correct=n_correct,
    )


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

def comparison_table(baseline_scores: dict, anidb_scores: dict) -> str:
    lines = []
    W = 70

    def header(title: str) -> None:
        lines.append(f"\n{title}")
        lines.append(f"  {'Metric':<30} {'Baseline':>12} {'ANIDB':>12}")
        lines.append(f"  {'-' * 54}")

    def row(label: str, b_val, a_val, fmt: str = ".1%") -> None:
        b_str = format(b_val, fmt) if b_val is not None else "N/A"
        a_str = format(a_val, fmt) if a_val is not None else "N/A"
        lines.append(f"  {label:<30} {b_str:>12} {a_str:>12}")

    lines.append("=" * W)
    lines.append("ANIDB vs SQL Baseline — Decision Quality Comparison")
    lines.append("=" * W)

    bc, ac = baseline_scores["churn"], anidb_scores["churn"]
    header("CHURN INTERVENTION")
    row("Precision@10",       bc.precision_at_10, ac.precision_at_10)
    row("Recall@10",          bc.recall_at_10,    ac.recall_at_10)
    row("Customers Flagged",  bc.n_flagged,       ac.n_flagged,  fmt="d")
    row("Correct Flags",      bc.n_correct,       ac.n_correct,  fmt="d")

    bp, ap = baseline_scores["pricing"], anidb_scores["pricing"]
    header("PRICING OPTIMIZATION (% error — lower is better)")
    row("Basic tier error",      bp.pct_error_basic,      ap.pct_error_basic)
    row("Pro tier error",        bp.pct_error_pro,        ap.pct_error_pro)
    row("Enterprise tier error", bp.pct_error_enterprise, ap.pct_error_enterprise)
    row("Weighted avg error",    bp.weighted_avg_error,   ap.weighted_avg_error)

    bk, ak = baseline_scores["capacity"], anidb_scores["capacity"]
    header("CAPACITY / INVENTORY")
    row("Precision",           bk.precision, ak.precision)
    row("Recall",              bk.recall,    ak.recall)
    row("Customers Flagged",   bk.n_flagged, ak.n_flagged, fmt="d")
    row("Correct Flags",       bk.n_correct, ak.n_correct, fmt="d")

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

    bc, ac = baseline_scores["churn"], anidb_scores["churn"]
    bar_chart(
        "Churn Intervention: Baseline vs ANIDB",
        ["Precision@10", "Recall@10"],
        [bc.precision_at_10, bc.recall_at_10],
        [ac.precision_at_10, ac.recall_at_10],
        "churn_comparison.png",
    )

    bp, ap = baseline_scores["pricing"], anidb_scores["pricing"]
    bar_chart(
        "Pricing Optimization: Price Error by Tier",
        ["Basic", "Pro", "Enterprise"],
        [bp.pct_error_basic or 0, bp.pct_error_pro or 0, bp.pct_error_enterprise or 0],
        [ap.pct_error_basic or 0, ap.pct_error_pro or 0, ap.pct_error_enterprise or 0],
        "pricing_comparison.png",
        ylabel="% Error (lower is better)",
        ylim=None,
    )

    bk, ak = baseline_scores["capacity"], anidb_scores["capacity"]
    bar_chart(
        "Capacity Detection: Baseline vs ANIDB",
        ["Precision", "Recall"],
        [bk.precision, bk.recall],
        [ak.precision, ak.recall],
        "capacity_comparison.png",
    )

    print(f"Charts written to {charts_dir}/")


def generate_report(
    baseline_scores: dict, anidb_scores: dict, output_dir: Path
) -> dict:
    def _dc(scores: dict) -> dict:
        return {
            "churn": asdict(scores["churn"]),
            "pricing": asdict(scores["pricing"]),
            "capacity": asdict(scores["capacity"]),
        }

    report = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "baseline": _dc(baseline_scores),
        "anidb": _dc(anidb_scores),
    }

    output_dir.mkdir(parents=True, exist_ok=True)
    report_path = output_dir / "report.json"
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2, default=str)
    print(f"Report written to {report_path}")
    return report


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="Compare ANIDB vs baseline agent performance")
    parser.add_argument("--output",   default="results")
    parser.add_argument("--data-dir", default="data")
    args = parser.parse_args()

    output_dir = Path(args.output)
    data_dir = Path(args.data_dir)

    gt_path = data_dir / "ground_truth.json"
    if not gt_path.exists():
        print(f"ERROR: Ground truth not found at {gt_path}")
        print("Run synthetic-data/generate.py first.")
        return

    baseline_path = output_dir / "baseline_decisions.json"
    anidb_path = output_dir / "anidb_decisions.json"

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
        "churn":    score_churn(bd.get("churn", {}),    ground_truth),
        "pricing":  score_pricing(bd.get("pricing", {}),  ground_truth),
        "capacity": score_capacity(bd.get("capacity", {}), ground_truth),
    }
    anidb_scores = {
        "churn":    score_churn(ad.get("churn", {}),    ground_truth),
        "pricing":  score_pricing(ad.get("pricing", {}),  ground_truth),
        "capacity": score_capacity(ad.get("capacity", {}), ground_truth),
    }

    print(comparison_table(baseline_scores, anidb_scores))
    generate_charts(baseline_scores, anidb_scores, output_dir)
    generate_report(baseline_scores, anidb_scores, output_dir)


if __name__ == "__main__":
    main()
