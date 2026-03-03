"""
ANIDB Synthetic Data Generator

Generates a deterministic 90-day SaaS business simulation with:
- 500 customers with realistic subscription behavior
- ~10,000 typed events
- Seeded churn signals, pricing changes, capacity events
- Ground truth decision outcomes for scoring

Usage:
    python synthetic-data/generate.py --customers 500 --days 90
"""
import argparse


def main():
    parser = argparse.ArgumentParser(description="Generate synthetic SaaS data")
    parser.add_argument("--customers", type=int, default=500)
    parser.add_argument("--days", type=int, default=90)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    print(
        f"TODO: Generate data for {args.customers} customers "
        f"over {args.days} days (seed={args.seed})"
    )


if __name__ == "__main__":
    main()
