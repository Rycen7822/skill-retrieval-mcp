---
name: structured-artifact-inspection
description: Codex plugin skill for inspecting structured artifacts, generated reports, JSON/HTML outputs, and viewer-backed evidence without losing provenance.
tags: [codex-builtin, artifact, inspection, viewer, provenance]
---
# structured-artifact-inspection
## When to Use
Use when a Codex plugin artifact viewer should inspect structured outputs, generated evidence, report bundles, or machine-readable artifacts.
## Do Not Use When
Do not use for generic source-code search or GitHub PR checks.
## Required Inputs
Artifact path or viewer target, expected schema or report type, and the verification question.
## Workflow
1. Open or render the artifact through the viewer.
2. Inspect key fields and provenance.
3. Summarize anomalies and verification status.
