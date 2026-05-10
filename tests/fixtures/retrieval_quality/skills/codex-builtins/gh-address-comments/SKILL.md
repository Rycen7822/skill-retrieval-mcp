---
name: gh-address-comments
description: Codex curated GitHub skill for addressing pull request review comments, resolving threads, applying requested changes, and reporting verification.
tags: [codex-builtin, github, pr-comments, review, threads]
---
# gh-address-comments
## When to Use
Use when the user asks to address GitHub PR review comments, collect unresolved threads, patch requested changes, and respond with verification notes.
## Do Not Use When
Do not use for failing CI logs without review comments; use gh-fix-ci instead.
## Required Inputs
Repository path, PR number or URL, gh authentication, review comment scope, and whether to post replies.
## Workflow
1. Fetch unresolved review comments.
2. Group comments by file and theme.
3. Patch the requested changes.
4. Run checks and prepare reply summaries.
