---
name: "huggingface-datasets"
description: "Codex curated Hugging Face skill for Dataset Viewer API workflows: splits, first rows, paginated rows, search, filters, parquet URLs, size, and statistics."
tags: [codex-builtin, huggingface, datasets, dataset-viewer, api]
---
# huggingface-datasets
## When to Use
Use for read-only Hugging Face Dataset Viewer exploration, metadata, split discovery, row pagination, search, filter, parquet, and dataset statistics.
## Do Not Use When
Do not use for training a model, publishing a paper, or generic GitHub issue work.
## Required Inputs
Dataset repository id, config, split, pagination limits, and whether private access is required.
## Workflow
1. Validate dataset availability.
2. Resolve configs and splits.
3. Preview or paginate rows.
4. Fetch parquet links, size, or statistics as needed.
