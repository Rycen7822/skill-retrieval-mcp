---
name: external-agent-hermes-plugin-planning
description: Plan integrations between third-party autonomous agent CLIs/runtimes and Hermes plugins, adapters, commands, configuration, and safety boundaries.
tags: [hermes, plugin, adapter, agent, integration, planning]
---
# External Agent Hermes Plugin Planning
## When to Use
Use when the user wants a design plan for integrating an external autonomous agent or runtime into Hermes as a plugin, adapter, command, or skill surface.
## Do Not Use When
Do not use when the user asks to build a generic MCP server, run Codex directly, or fix uv virtualenv state.
## Required Inputs
External agent CLI details, desired Hermes command surface, config model, safety constraints, and testing expectations.
## Workflow
1. Inspect the external runtime interface and Hermes plugin requirements.
2. Define adapter boundaries, command routing, configuration, and risk controls.
3. Plan tests, documentation, and rollout strategy.
4. Produce an implementation roadmap without prematurely writing code.
