---
name: "huggingface-gradio"
description: Codex curated Hugging Face skill for building, debugging, and deploying Gradio demos and Spaces interfaces.
tags: [codex-builtin, huggingface, gradio, spaces, demo]
---
# huggingface-gradio
## When to Use
Use when implementing or troubleshooting a Gradio app, UI component, event handler, queue, launch setting, or Hugging Face Space demo.
## Do Not Use When
Do not use for dataset API browsing or GitHub Actions CI fixes.
## Required Inputs
App file, desired UI behavior, model or function endpoint, and local or Space deployment target.
## Workflow
1. Inspect the Gradio interface and callbacks.
2. Patch components or events.
3. Run locally if possible.
4. Verify the demo launches and expected interactions work.
