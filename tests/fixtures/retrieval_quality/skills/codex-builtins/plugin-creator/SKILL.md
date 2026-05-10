---
name: "plugin-creator"
description: "Codex .system built-in skill for creating and scaffolding Codex plugin directories with .codex-plugin/plugin.json, optional skills, hooks, scripts, assets, MCP, apps, and marketplace entries."
tags: ["codex-builtin", "codex-system", "plugin-creator", "scaffold", "codex-plugin"]
---
# plugin-creator
## When to Use
Use when the user wants to create a new Codex plugin, scaffold plugin structure, add marketplace metadata, or generate plugin placeholder files.
## Do Not Use When
Do not use for writing a standalone Hermes plugin plan or creating a normal skill without plugin structure.
## Required Inputs
Plugin name, target parent directory, desired optional folders, and whether marketplace metadata should be generated.
## Workflow
1. Normalize the plugin name.
2. Run or emulate the plugin scaffold.
3. Fill plugin.json placeholders.
4. Verify the plugin directory and marketplace entry.
