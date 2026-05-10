---
name: "hf-cli"
description: "Codex curated Hugging Face skill for the Hub `hf` CLI: authentication checks, repository management, uploads, downloads, and metadata inspection."
tags: [codex-builtin, huggingface, cli, hub, upload]
---
# hf-cli
## When to Use
Use when the task requires Hugging Face CLI or Hub API operations: auth status, repo management, uploads, downloads, or model/dataset card metadata.
## Do Not Use When
Do not use for GitHub CLI workflows or dataset row pagination that belongs to the Dataset Viewer skill.
## Required Inputs
Hub repo id, operation type, token availability, files or patterns, and whether changes are read-only or write actions.
## Workflow
1. Check Hub authentication.
2. Resolve model, dataset, or Space repo target.
3. Run the minimal CLI/API operation.
4. Verify URL, revision, or file listing.
