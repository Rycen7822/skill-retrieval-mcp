from __future__ import annotations

from pathlib import Path


def write_skill(root: Path, rel: str, body: str) -> Path:
    skill_dir = root / rel
    skill_dir.mkdir(parents=True, exist_ok=True)
    path = skill_dir / "SKILL.md"
    path.write_text(body.strip() + "\n", encoding="utf-8")
    return path


CODE_REVIEW_SKILL = """
---
name: github-code-review
description: Review GitHub pull requests or git diffs for correctness, security, and maintainability findings.
tags: [github, code-review, security, git]
---

# GitHub Code Review

## When to Use

Use when the user asks to review an existing pull request, patch, or git diff and produce prioritized findings.

## Do Not Use When

Do not use when the user asks you to implement a feature, push changes, create a pull request, or merge code.

## Required Inputs

A git repository and a diff or pull request reference.

## Workflow

1. Inspect repository state and identify the base diff.
2. Read relevant files around changed lines.
3. Identify correctness, security, maintainability, and test coverage issues.
4. Run targeted tests or static checks when available.
5. Return prioritized review findings with file and line evidence.

## Verification

Confirm every finding is backed by code evidence and avoid speculative issues.

## Pitfalls

Do not rewrite the user code during review unless explicitly asked.
"""


PR_WORKFLOW_SKILL = """
---
name: github-pr-workflow
description: Create, update, and manage GitHub pull requests using git and gh.
tags: [github, pull-request, git, workflow]
---

# GitHub PR Workflow

## When to Use

Use when the user asks to create a branch, commit changes, open a PR, update PR metadata, or inspect CI status.

## Do Not Use When

Do not use when the user only wants an independent code review of an existing diff.

## Workflow

1. Inspect git status and current branch.
2. Create or update a feature branch.
3. Commit requested changes.
4. Open or update the pull request.
5. Check CI status and report the PR URL.

## Verification

Verify the branch, commits, PR URL, and CI state.
"""


IMPLEMENT_SKILL = """
---
name: test-driven-development
description: Implement features and bug fixes by writing failing tests first, then production code, then refactoring.
tags: [testing, implementation, feature, bugfix]
---

# Test Driven Development

## When to Use

Use when the user asks to implement a feature, fix a bug, or change behavior in code.

## Do Not Use When

Do not use when the user only asks for a review without changing code.

## Workflow

1. Write a failing test for the requested behavior.
2. Run the test and confirm it fails for the expected reason.
3. Write minimal production code.
4. Run the specific test and full test suite.
5. Refactor while keeping tests green.

## Verification

All targeted tests and regression tests pass.
"""
