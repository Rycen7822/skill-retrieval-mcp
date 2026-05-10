---
name: mcp-builder
description: Build, test, evaluate, and document Model Context Protocol MCP servers, tools, schemas, transports, and read-only evaluation suites.
tags: [mcp, server, tools, schema, evaluation]
---
# MCP Builder
## When to Use
Use when the user asks to design an MCP server, add MCP tools, define tool schemas, evaluate MCP usability, or create MCP client configuration.
## Do Not Use When
Do not use when the user asks to delegate coding to Codex CLI, fix a uv virtualenv, or triage GitHub issues.
## Required Inputs
Target API or local capability, tool names, schemas, transport, and evaluation goals.
## Workflow
1. Define minimal read-only or controlled tool surface.
2. Implement typed schemas and server transport.
3. Add integration tests for list_tools and call_tool.
4. Create evaluation tasks and document client configuration.
