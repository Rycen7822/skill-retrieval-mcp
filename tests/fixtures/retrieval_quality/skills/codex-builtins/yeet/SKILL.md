---
name: yeet
description: Codex curated GitHub skill for publishing local changes by confirming scope, staging, committing, pushing a branch, and opening a draft pull request.
tags: [codex-builtin, github, publish, commit, push, pull-request, safety]
---
# yeet
## When to Use
Use when the user explicitly wants the full publish flow from a local checkout: branch setup, staging, commit, push, and opening a draft PR.
## Do Not Use When
Do not use for review-only tasks, CI log debugging, or local cleanup without an explicit publish/push/PR request.
## Required Inputs
Repository path, intended change scope, remote target, branch/base information, gh authentication, and checks to run before publishing.
## Workflow
1. Inspect git status and diff.
2. Confirm intended scope and branch strategy.
3. Stage only approved files and commit.
4. Push the branch and open a draft PR, then report links and checks.
