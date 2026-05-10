---
name: github-pr-workflow
description: Create branches, commit changes, push updates, open GitHub pull requests, update PR metadata, and monitor CI checks.
tags: [github, pull-request, branch, commit, ci, push]
---
# GitHub PR Workflow
## When to Use
Use when the user asks to create a branch, make commits, push code, open or update a pull request, watch CI, or report the PR URL.
## Do Not Use When
Do not use when the user only wants an independent code review of an existing pull request or diff without making changes.
## Required Inputs
A git repository, remote origin, branch target, and requested change or PR metadata.
## Workflow
1. Check git status and current branch.
2. Create or update a feature branch.
3. Stage and commit requested changes.
4. Push to GitHub and open or update the PR.
5. Verify CI status and report branch, commit, and PR URL.
