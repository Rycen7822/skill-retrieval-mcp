---
name: "huggingface-community-evals"
description: Codex curated Hugging Face skill for preparing, running, and packaging community evaluation tasks and benchmark submissions.
tags: [codex-builtin, huggingface, evals, benchmarks, community]
---
# huggingface-community-evals
## When to Use
Use for Hugging Face community eval harnesses, benchmark task packaging, eval metadata, result validation, or leaderboard-oriented workflows.
## Do Not Use When
Do not use for ordinary unit-test debugging or PDF visual checks.
## Required Inputs
Evaluation task, dataset, metric, expected submission format, and any leaderboard constraints.
## Workflow
1. Inspect eval specification.
2. Prepare data and task config.
3. Run or validate evaluation.
4. Package results with reproducibility notes.
