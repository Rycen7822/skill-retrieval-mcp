#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from skill_retrieval_mcp.core import SkillRetrievalEngine
from skill_retrieval_mcp.evaluation import evaluate_case_diversity, evaluate_retrieval_cases, load_retrieval_cases


def _default_fixture_root() -> Path:
    return Path(__file__).resolve().parents[1] / "tests" / "fixtures" / "retrieval_quality"


def _threshold_failures(report: dict[str, Any], args: argparse.Namespace) -> list[str]:
    failures: list[str] = []
    checks = [
        ("top1_accuracy", args.min_top1),
        ("hit_rate_at_k", args.min_hit_rate),
        ("mean_average_precision_at_k", args.min_map),
        ("mean_judged_precision_at_k", args.min_judged_precision),
    ]
    for metric, minimum in checks:
        value = float(report.get(metric, 0.0))
        if value < minimum:
            failures.append(f"{metric}={value:.4f} < {minimum:.4f}")
    if report.get("forbidden_topk_violations"):
        failures.append(f"forbidden_topk_violations={len(report['forbidden_topk_violations'])}")
    if report.get("missing_expected_topk"):
        failures.append(f"missing_expected_topk={len(report['missing_expected_topk'])}")
    diversity = report.get("diversity") or {}
    if diversity:
        minimum_checks = [
            ("diversity.intent_entropy_norm", float(diversity.get("intent_entropy_norm", 0.0)), args.min_intent_entropy),
            ("diversity.language_entropy_norm", float(diversity.get("language_entropy_norm", 0.0)), args.min_language_entropy),
            ("diversity.category_entropy_norm", float(diversity.get("category_entropy_norm", 0.0)), args.min_category_entropy),
            ("diversity.gold_skill_entropy_norm", float(diversity.get("gold_skill_entropy_norm", 0.0)), args.min_gold_entropy),
            ("diversity.mmr_mean_novelty", float(diversity.get("mmr_mean_novelty", 0.0)), args.min_mmr_novelty),
        ]
        for metric, value, minimum in minimum_checks:
            if value < minimum:
                failures.append(f"{metric}={value:.4f} < {minimum:.4f}")
        maximum_checks = [
            ("diversity.mean_pairwise_cosine", float(diversity.get("mean_pairwise_cosine", 1.0)), args.max_mean_cosine),
            ("diversity.p95_pairwise_cosine", float(diversity.get("p95_pairwise_cosine", 1.0)), args.max_p95_cosine),
            ("diversity.max_pairwise_cosine", float(diversity.get("max_pairwise_cosine", 1.0)), args.max_max_cosine),
        ]
        for metric, value, maximum in maximum_checks:
            if value > maximum:
                failures.append(f"{metric}={value:.4f} > {maximum:.4f}")
        if diversity.get("near_duplicate_pairs"):
            failures.append(f"diversity.near_duplicate_pairs={len(diversity['near_duplicate_pairs'])}")
        if diversity.get("mmr_low_novelty_cases"):
            failures.append(f"diversity.mmr_low_novelty_cases={len(diversity['mmr_low_novelty_cases'])}")
    return failures


def main() -> int:
    fixture_root = _default_fixture_root()
    parser = argparse.ArgumentParser(description="Evaluate skill retrieval accuracy and precision on labelled JSONL cases.")
    parser.add_argument("--cases", type=Path, default=fixture_root / "cases.jsonl", help="JSONL retrieval test cases")
    parser.add_argument("--skill-root", type=Path, default=fixture_root / "skills", help="Skill root to index")
    parser.add_argument("--cache", type=Path, default=Path(".cache/retrieval-quality-index.json"), help="Index cache path")
    parser.add_argument("--json-out", type=Path, default=None, help="Optional path to write full JSON report")
    parser.add_argument("--min-top1", type=float, default=0.90, help="Minimum top-1 accuracy")
    parser.add_argument("--min-hit-rate", type=float, default=0.98, help="Minimum hit rate at requested k")
    parser.add_argument("--min-map", type=float, default=0.92, help="Minimum mean average precision at requested k")
    parser.add_argument("--min-judged-precision", type=float, default=0.94, help="Minimum judged precision over relevant/forbidden labels")
    parser.add_argument("--near-duplicate-threshold", type=float, default=0.86, help="TF-IDF cosine threshold for duplicate-like cases")
    parser.add_argument("--min-intent-entropy", type=float, default=0.82, help="Minimum normalised intent entropy")
    parser.add_argument("--min-language-entropy", type=float, default=0.50, help="Minimum normalised language entropy")
    parser.add_argument("--min-category-entropy", type=float, default=0.50, help="Minimum normalised category entropy")
    parser.add_argument("--min-gold-entropy", type=float, default=0.90, help="Minimum normalised gold skill entropy")
    parser.add_argument("--max-mean-cosine", type=float, default=0.08, help="Maximum mean pairwise TF-IDF cosine")
    parser.add_argument("--max-p95-cosine", type=float, default=0.25, help="Maximum p95 pairwise TF-IDF cosine")
    parser.add_argument("--max-max-cosine", type=float, default=0.80, help="Maximum pairwise TF-IDF cosine")
    parser.add_argument("--min-mmr-novelty", type=float, default=0.58, help="Minimum mean MMR novelty over the dataset")
    parser.add_argument("--skip-diversity", action="store_true", help="Skip algorithmic dataset-diversity metrics")
    parser.add_argument("--print-outcomes", action="store_true", help="Print per-case outcomes as JSON")
    args = parser.parse_args()

    cases = load_retrieval_cases(args.cases)
    engine = SkillRetrievalEngine(roots=[args.skill_root], cache_path=args.cache)
    report = evaluate_retrieval_cases(engine, cases)
    if not args.skip_diversity:
        report["diversity"] = evaluate_case_diversity(
            cases,
            near_duplicate_threshold=args.near_duplicate_threshold,
            mmr_lambda=0.65,
        )
    failures = _threshold_failures(report, args)
    report["threshold_failures"] = failures

    summary = {
        key: report[key]
        for key in [
            "case_count",
            "top1_accuracy",
            "hit_rate_at_k",
            "mean_average_precision_at_k",
            "mean_judged_precision_at_k",
            "forbidden_topk_violations",
            "missing_expected_topk",
            "top1_misses",
            "threshold_failures",
        ]
    }
    if "diversity" in report:
        diversity = report["diversity"]
        summary["diversity"] = {
            key: diversity[key]
            for key in [
                "intent_entropy_norm",
                "language_entropy_norm",
                "category_entropy_norm",
                "gold_skill_entropy_norm",
                "mean_pairwise_cosine",
                "p95_pairwise_cosine",
                "max_pairwise_cosine",
                "mmr_mean_novelty",
                "mmr_low_novelty_cases",
                "near_duplicate_pairs",
            ]
        }
    if args.print_outcomes:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        print(json.dumps(summary, ensure_ascii=False, indent=2))

    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
