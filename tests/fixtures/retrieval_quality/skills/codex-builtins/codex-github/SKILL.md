---
name: "github"
description: "Codex curated GitHub skill for general GitHub app workflows: repository metadata, issues, pull requests, comments, and connector-backed context."
tags: [codex-builtin, github, connector, issues, pull-requests]
---
# github
## When to Use
Use when Codex needs its curated GitHub connector workflow for repository metadata, issues, PR details, comments, or app-backed context.
## Do Not Use When
Do not use for Hermes github umbrella skill routing unless the request is specifically about Codex curated GitHub plugin behavior.
## Required Inputs
Repository owner/name, issue or PR target, desired GitHub operation, and auth context.
## Workflow
1. Resolve repository and target issue or PR.
2. Fetch metadata through the GitHub connector or gh.
3. Perform the requested read or write operation.
4. Verify URLs and status.
