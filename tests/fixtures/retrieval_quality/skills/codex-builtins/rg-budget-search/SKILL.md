---
name: rg-budget-search
description: Codex built-in skill for budgeted ripgrep-style repository search that avoids huge terminal output by narrowing files and bounded hit windows.
tags: [codex-builtin, ripgrep, search-budget, code-search]
---
# rg-budget-search
## When to Use
Use when searching a large repository with strict terminal-output budget, first files-only then bounded hits and exact spans.
## Do Not Use When
Do not use when an AST structural query is required; use ast-grep for code-shape matching.
## Required Inputs
Search pattern, target directory, file globs, and output budget.
## Workflow
1. Run a files-only search first.
2. Narrow by path or glob.
3. Read bounded hit spans.
4. Record exactly which ranges were inspected.
