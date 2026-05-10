---
name: "huggingface-trackio"
description: Codex curated Hugging Face skill for experiment tracking with Trackio logs, metrics, dashboards, and run comparisons.
tags: [codex-builtin, huggingface, trackio, experiment-tracking, metrics]
---
# huggingface-trackio
## When to Use
Use when adding or debugging Trackio experiment tracking, logging metrics, comparing runs, or publishing lightweight dashboards.
## Do Not Use When
Do not use for static paper writing or plain dataset browsing.
## Required Inputs
Training script or experiment run, metric names, logging destination, and comparison goal.
## Workflow
1. Add or inspect Trackio initialization.
2. Log scalar metrics and artifacts.
3. Run a small smoke experiment.
4. Review dashboard or exported run summary.
