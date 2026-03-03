"""
ANIDB Evaluation Framework

Scores both agents against pre-defined ground truth outcomes.
- Churn: recall and intervention timing
- Pricing: margin improvement vs counterfactual
- Capacity: lead time accuracy

Usage:
    python evaluation/compare.py --output results/
"""
import argparse


def main():
    parser = argparse.ArgumentParser(description="Compare agent performance")
    parser.add_argument("--output", default="results/")
    args = parser.parse_args()

    print(f"TODO: Run evaluation, output to {args.output}")


if __name__ == "__main__":
    main()
