---
name: "openai-docs"
description: "Codex .system built-in skill for current OpenAI product and API documentation, model selection, prompt upgrades, API migration, and official-doc citations."
tags: ["codex-builtin", "codex-system", "openai-docs", "api-docs", "model-guidance"]
---
# openai-docs
## When to Use
Use when the user asks how to build with OpenAI APIs or products, choose a current OpenAI model, migrate prompts/models, or cite official OpenAI docs.
## Do Not Use When
Do not use for non-OpenAI provider documentation or implementing an app before the docs question is resolved.
## Required Inputs
OpenAI product or API surface, desired use case, current model or prompt if migrating, and whether citations are required.
## Workflow
1. Search official OpenAI docs.
2. Fetch the relevant section.
3. Preserve explicit model targets unless the user asks for latest.
4. Answer with concise grounded guidance.
