# Retrieval Quality Test Set

This directory defines a labelled, reproducible retrieval-quality evaluation for Skill Retrieval MCP. It complements the stress harness by measuring selection correctness and precision rather than only API stability.

## Files

- `tests/fixtures/retrieval_quality/skills/` — synthetic but realistic skill library with hard near-neighbour skills.
- `tests/fixtures/retrieval_quality/cases.jsonl` — labelled retrieval cases.
- `src/skill_retrieval_mcp/evaluation.py` — reusable evaluator and metrics.
- `tests/test_retrieval_quality.py` — deterministic synthetic-fixture quality gate.
- `tests/test_builtin_skill_roots.py` — environment-aware smoke gate for installed Hermes Agent and Codex built-in skill roots when they exist locally.
- `scripts/evaluate_retrieval_quality.py` — CLI report generator.

## Case schema

Each JSONL case contains:

- `case_id`: stable case identifier.
- `intent`: one of `direct`, `paraphrase`, `zh`, `hard_negative`, `must_not`, `category_filter`, `multi_relevant`, `noisy_context`, `typo`, `low_budget`, `environment_context`, or `cross_language`.
- `difficulty`: `easy`, `medium`, or `hard`.
- `language`: currently includes `en`, `zh`, `mixed`, `ja`, and `es` labels. Non-English cases keep structured English anchors where needed so the lexical retriever can still be evaluated deterministically.
- `relevant_skill_ids`: all acceptable answers.
- `expected_top1`: stricter expected rank-1 skill when the case is unambiguous.
- `forbidden_skill_ids`: known false positives for precision checks.
- SearchRequest fields: `raw_user_request`, `description_query`, `workflow_query`, `must_have`, `must_not`, `environment`, `category`, `k`, `max_tokens`, `mmr_lambda`.

## Metrics

The evaluator reports:

- `top1_accuracy`: rank-1 correctness. Uses `expected_top1` when provided, otherwise accepts any relevant skill.
- `hit_rate_at_k`: at least one relevant skill appears in the returned top-k.
- `mean_average_precision_at_k`: standard AP@k averaged over cases.
- `mean_judged_precision_at_k`: precision over judged labels only: relevant vs explicitly forbidden skills. Unjudged neighbours are neutral.
- `forbidden_topk_violations`: cases where a known false positive entered top-k.
- `missing_expected_topk`: cases where no relevant skill entered top-k.
- `diversity.intent_entropy_norm`, `diversity.language_entropy_norm`, `diversity.category_entropy_norm`, `diversity.gold_skill_entropy_norm`: normalised Shannon entropy for distribution balance.
- `diversity.mean_pairwise_cosine`, `diversity.p95_pairwise_cosine`, `diversity.max_pairwise_cosine`: TF-IDF cosine similarity over case request surfaces, used to detect duplicate-like samples and over-concentrated phrasing.
- `diversity.near_duplicate_pairs`: high-cosine pairs above the duplicate threshold.
- `diversity.mmr_mean_novelty`, `diversity.mmr_low_novelty_cases`: MMR-style novelty of each case against previously selected cases, used to catch samples that add little incremental coverage.

## Run

```bash
uv run pytest tests/test_retrieval_quality.py -q
uv run pytest tests/test_builtin_skill_roots.py -q
uv run python scripts/evaluate_retrieval_quality.py \
  --json-out reports/retrieval-quality/latest.json
```

Default quality gates:

- top-1 accuracy >= 0.90
- hit rate@k >= 0.98
- mean AP@k >= 0.92
- judged precision@k >= 0.94
- no forbidden top-k violations
- no missing expected top-k cases
- diversity entropy gates: intent >= 0.82, language >= 0.50, category >= 0.50, gold skill >= 0.90
- TF-IDF cosine gates: mean <= 0.08, p95 <= 0.25, max <= 0.80, no near-duplicate pairs above 0.86
- MMR novelty gate: mean novelty >= 0.58 and no low-novelty cases below 0.28

## Design notes

The fixture now contains 500 cases over at least 50 gold skills. It covers the original GitHub/research/note-taking/development set plus additional synthetic skills in productivity-adjacent content processing, data-science, creative writing, user-imported tools, Hermes plugin planning, the `hermes-agent` skill, and Codex built-ins from `$CODEX_HOME/skills` / `.system` plus curated Hugging Face and GitHub plugin skills. The diversity gate in `tests/test_retrieval_quality.py` requires:

- at least 500 cases and 50 gold skills;
- direct, paraphrase, Chinese, hard-negative, must-not, category-filter, multi-relevant, noisy-context, typo, low-budget, environment-context, and cross-language intents;
- English, Chinese, mixed Chinese/English, Japanese-labelled, and Spanish-labelled request surfaces;
- low and medium token budgets, including budgets at or below 300 tokens;
- category coverage for `github`, `software-development`, `note-taking`, `research`, `productivity`, `autonomous-ai-agents`, `data-science`, `creative`, `user-imported`, and `codex-builtins`.

Precision-labelled hard-negative, must-not, paraphrase, multilingual, noisy-context, low-budget, and environment-context cases set `k=1` when `forbidden_skill_ids` are known near-neighbour false positives. This makes the precision gate evaluate the selected top skill rather than penalizing useful preview candidates that appear lower in the exploratory top-k list. Category and multi-relevant cases still exercise broader top-k retrieval.

The fixture intentionally includes overlapping GitHub, research, note-taking, development, data-science, creative, document-conversion, plotting, and agent-integration skills. The hard-negative and must-not cases use near neighbours such as:

- `github-code-review` vs `github-pr-workflow`
- `test-driven-development` vs `systematic-debugging`
- `clip` vs `llm-wiki`
- `paper-reliability-verification` vs `research-paper-writing`
- `mcp-builder` vs `codex`
- `docling` vs `docx-reference-gbt7714-report-repair`
- `csv-taxonomy-consolidation` vs `academic-plotting`
- `obsidian` vs `clip`/`llm-wiki`
- `external-agent-hermes-plugin-planning` vs `mcp-builder`
- `hermes-agent` vs generic autonomous-agent delegation skills
- Codex built-in near neighbours such as `ast-grep` vs `rg-budget-search`, `huggingface-vision-trainer` vs `huggingface-llm-trainer`, `skill-creator` vs `skill-installer`, and `gh-fix-ci` vs `gh-address-comments`

This makes the set useful for detecting regressions in precision, not only recall.
