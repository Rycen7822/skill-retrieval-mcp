---
name: codex
description: Delegate coding tasks to the OpenAI Codex CLI agent for implementation, refactoring, PR review, batch issue fixing, and autonomous repository work.
tags: [codex, coding-agent, delegation, refactoring, cli]
---
# Codex CLI Agent
## When to Use
Use when the user asks to run or delegate work to Codex CLI, spawn an autonomous coding agent, batch-fix issues, or have a coding agent inspect a repository.
## Do Not Use When
Do not use when the task is to build an MCP server directly, manage uv environments, or clip web articles into notes.
## Required Inputs
A git repository, Codex CLI availability, task prompt, and safety boundary.
## Workflow
1. Inspect repository cleanliness and task scope.
2. Start Codex CLI with a bounded prompt.
3. Review generated changes and run tests.
4. Summarize agent output and verify files.
