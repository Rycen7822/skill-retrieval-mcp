# Retrieval quality fixture

This fixture is a labelled, synthetic skill library used to evaluate skill-level retrieval quality.

## Scope

The dataset intentionally mixes:

- direct skill-id requests;
- paraphrases and near-neighbour disambiguation;
- Chinese, English, mixed Chinese/English, Japanese-labelled, and Spanish-labelled requests;
- hard negatives and explicit `must_not` constraints;
- category-filtered routing;
- noisy user context with irrelevant distractors;
- typo-heavy raw requests paired with normalized structured query fields;
- low token-budget searches;
- environment-sensitive hints such as WSL, Windows/Chrome, pandas, matplotlib, OCR, Hermes plugins, Hermes Agent configuration, and Codex built-in skill roots;
- multi-relevant routing cases where several skills are acceptable.

The fixture explicitly includes the `hermes-agent` skill plus Codex built-ins mirrored from `$CODEX_HOME/skills`, `$CODEX_HOME/skills/.system`, active Codex plugin skills, and curated Hugging Face/GitHub Codex skills. The real local roots are also smoke-tested in `tests/test_builtin_skill_roots.py` when they exist on the machine running pytest.

Each JSONL case is fully judged with `relevant_skill_ids`; precision-critical cases also include `expected_top1` and `forbidden_skill_ids`.

The fixture is also guarded by algorithmic diversity checks:

- normalised entropy for intent, language, category, and gold-skill distributions;
- TF-IDF cosine similarity to detect near-duplicates and over-concentrated phrasing;
- MMR-style novelty to flag cases that add little incremental coverage.
