---
name: github-repo-management
description: Clone, create, fork, configure, archive, transfer, release, and manage GitHub repositories, remotes, visibility, and settings.
tags: [github, repository, clone, fork, settings, release]
---
# GitHub Repository Management
## When to Use
Use when the user asks to clone a repository, create a new repository, fork a project, configure repository settings, manage remotes, archive, transfer, or create releases.
## Do Not Use When
Do not use when the user only wants to triage issues, review a pull request diff, or write code in an existing branch.
## Required Inputs
Repository owner/name, target visibility, remote URL, settings, or release metadata.
## Workflow
1. Inspect local git remote and GitHub owner context.
2. Clone, create, fork, configure, archive, transfer, or release the repository.
3. Verify remotes, default branch, visibility, permissions, and URL.
