from __future__ import annotations

from pathlib import Path


def test_retrieval_quality_fixture_meets_accuracy_and_precision_thresholds(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.evaluation import evaluate_retrieval_cases, load_retrieval_cases

    fixture_root = Path(__file__).parent / "fixtures" / "retrieval_quality"
    cases = load_retrieval_cases(fixture_root / "cases.jsonl")
    engine = SkillRetrievalEngine(roots=[fixture_root / "skills"], cache_path=tmp_path / "quality-cache.json")

    report = evaluate_retrieval_cases(engine, cases)

    assert report["case_count"] >= 48
    assert report["top1_accuracy"] >= 0.90
    assert report["hit_rate_at_k"] >= 0.98
    assert report["mean_average_precision_at_k"] >= 0.92
    assert report["mean_judged_precision_at_k"] >= 0.94
    assert report["forbidden_topk_violations"] == []
    assert report["missing_expected_topk"] == []


def test_retrieval_quality_dataset_is_balanced_and_fully_judged() -> None:
    from skill_retrieval_mcp.evaluation import load_retrieval_cases

    fixture_root = Path(__file__).parent / "fixtures" / "retrieval_quality"
    cases = load_retrieval_cases(fixture_root / "cases.jsonl")

    intents = {case.intent for case in cases}
    difficulties = {case.difficulty for case in cases}
    languages = {case.language for case in cases}
    all_gold = {skill_id for case in cases for skill_id in case.relevant_skill_ids}

    categories = {case.category for case in cases if case.category}
    token_budgets = {case.max_tokens for case in cases}

    assert len(cases) >= 500
    assert {
        "direct",
        "paraphrase",
        "zh",
        "hard_negative",
        "must_not",
        "category_filter",
        "multi_relevant",
        "noisy_context",
        "typo",
        "low_budget",
        "environment_context",
        "cross_language",
    }.issubset(intents)
    assert {"easy", "medium", "hard"}.issubset(difficulties)
    assert {"en", "zh", "mixed", "ja", "es"}.issubset(languages)
    assert {
        "github",
        "software-development",
        "note-taking",
        "research",
        "productivity",
        "autonomous-ai-agents",
        "data-science",
        "creative",
        "user-imported",
        "codex-builtins",
    }.issubset(categories)
    assert min(token_budgets) <= 300
    assert any(300 < budget <= 800 for budget in token_budgets)
    expected_builtin_skill_ids = {
        "hermes-agent",
        "codex",
        "ast-grep",
        "pdf",
        "nvmini",
        "structured-artifact-inspection",
        "rg-budget-search",
        "huggingface-datasets",
        "huggingface-gradio",
        "huggingface-community-evals",
        "huggingface-trackio",
        "huggingface-vision-trainer",
        "huggingface-jobs",
        "hf-cli",
        "huggingface-llm-trainer",
        "huggingface-papers",
        "transformers-js",
        "huggingface-paper-publisher",
        "gh-fix-ci",
        "gh-address-comments",
        "github",
        "yeet",
        "imagegen",
        "openai-docs",
        "plugin-creator",
        "skill-creator",
        "skill-installer",
    }

    assert len(all_gold) >= 50
    assert expected_builtin_skill_ids.issubset(all_gold)
    assert all(case.relevant_skill_ids for case in cases)
    assert all(case.expected_top1 in case.relevant_skill_ids for case in cases if case.expected_top1)
    assert all(case.forbidden_skill_ids for case in cases if case.intent in {"hard_negative", "must_not"})


def test_retrieval_quality_dataset_meets_algorithmic_diversity_gate() -> None:
    from skill_retrieval_mcp.evaluation import evaluate_case_diversity, load_retrieval_cases

    fixture_root = Path(__file__).parent / "fixtures" / "retrieval_quality"
    cases = load_retrieval_cases(fixture_root / "cases.jsonl")

    report = evaluate_case_diversity(cases, near_duplicate_threshold=0.86, mmr_lambda=0.65)

    assert report["case_count"] >= 500
    assert report["intent_entropy_norm"] >= 0.82
    assert report["language_entropy_norm"] >= 0.50
    assert report["category_entropy_norm"] >= 0.50
    assert report["gold_skill_entropy_norm"] >= 0.90
    assert report["mean_pairwise_cosine"] <= 0.08
    assert report["p95_pairwise_cosine"] <= 0.25
    assert report["max_pairwise_cosine"] <= 0.80
    assert report["near_duplicate_pairs"] == []
    assert report["mmr_mean_novelty"] >= 0.58
    assert report["mmr_low_novelty_cases"] == []
