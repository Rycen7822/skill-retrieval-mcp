---
name: github-code-review
description: Review existing GitHub pull requests, patches, or git diffs for correctness, security, maintainability, and test coverage findings.
tags: [github, pull-request, code-review, diff, security]
---
# GitHub Code Review
## When to Use
Use when the user asks to review an existing pull request, inspect a patch, audit a git diff, or produce prioritized review findings without changing files.
## Do Not Use When
Do not use when the user asks to implement a feature, create a branch, commit changes, open a pull request, merge, push, or manage repository settings.
## Required Inputs
A git repository plus a diff, patch, commit range, or pull request reference.
## Workflow
1. Inspect git status and locate the base diff.
2. Read changed files around modified lines.
3. Identify correctness, security, maintainability, and missing test issues.
4. Return prioritized findings with evidence and avoid speculative rewrites.
