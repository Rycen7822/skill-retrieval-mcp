---
name: ast-grep
description: Codex built-in skill for structural code search with ast-grep when queries depend on AST shapes, call patterns, decorators, scopes, or required child nodes.
tags: [codex-builtin, ast-grep, structural-search, code-search]
---
# ast-grep
## When to Use
Use when the task needs AST-aware structural search instead of plain text grep, such as finding function definitions, call shapes, imports, decorators, or missing child nodes.
## Do Not Use When
Do not use for ordinary keyword, filename, documentation, or broad repository text search.
## Required Inputs
Target language, structural pattern, repository path, and a small validation snippet when possible.
## Workflow
1. Narrow the candidate files if the repository is large.
2. Write an ast-grep pattern or rule.
3. Run ast-grep on a small target.
4. Expand only after validating the match shape.
