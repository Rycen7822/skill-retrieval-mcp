---
name: gh-fix-ci
description: Codex curated GitHub skill for debugging and fixing failing GitHub Actions checks on pull requests with gh logs and focused patches.
tags: [codex-builtin, github, actions, ci, pr-checks]
---
# gh-fix-ci
## When to Use
Use when a PR has failing GitHub Actions checks and the agent must inspect gh check status, logs, root cause, and implement a focused fix.
## Do Not Use When
Do not use for non-GitHub Actions providers or generic repository management without CI failure logs.
## Required Inputs
Repository path, PR number or URL, gh authentication, failing check names, and permission to patch.
## Workflow
1. Verify gh auth and PR identity.
2. Inspect failing checks and Actions logs.
3. Summarize root cause.
4. Apply a minimal fix and rerun relevant tests.
