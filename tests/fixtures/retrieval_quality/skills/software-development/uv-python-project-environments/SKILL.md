---
name: uv-python-project-environments
description: Manage Python projects using uv: sync dependencies, lock pyproject metadata, repair virtualenvs, refresh console scripts, and rename venv prompts.
tags: [python, uv, virtualenv, pyproject, environment]
---
# uv Python Project Environments
## When to Use
Use when the user asks to inspect pyproject.toml, run uv sync, repair a moved .venv, refresh stale shebangs, rename the virtualenv prompt, or verify uv lock consistency.
## Do Not Use When
Do not use for generic code implementation, GitHub pull request creation, or paper research tasks.
## Required Inputs
A uv-managed Python project containing pyproject.toml, uv.lock, or .venv metadata.
## Workflow
1. Inspect pyproject.toml, uv.lock, and .venv state.
2. Run uv lock, uv sync, or uv venv --allow-existing --prompt as needed.
3. Verify imports, console scripts, and active Python path.
