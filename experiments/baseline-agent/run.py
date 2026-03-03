"""
SQL Baseline Agent

A well-implemented SQL agent using Claude with full schema access.
Operates against a PostgreSQL mirror of the event data.
Gets every advantage: full schema docs, example queries, unlimited retries.

Usage:
    python baseline-agent/run.py --scenario churn
"""
import argparse


def main():
    parser = argparse.ArgumentParser(description="Run SQL baseline agent")
    parser.add_argument(
        "--scenario",
        choices=["churn", "pricing", "capacity"],
        required=True,
    )
    args = parser.parse_args()

    print(f"TODO: Run baseline SQL agent for scenario: {args.scenario}")


if __name__ == "__main__":
    main()
