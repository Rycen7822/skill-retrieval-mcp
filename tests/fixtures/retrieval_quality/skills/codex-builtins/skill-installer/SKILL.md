---
name: "skill-installer"
description: "Codex .system built-in skill for installing Codex skills into CODEX_HOME/skills from curated OpenAI skills or GitHub repository paths, including private repos when credentials exist."
tags: ["codex-builtin", "codex-system", "skill-installer", "install", "github"]
---
# skill-installer
## When to Use
Use when the user asks to list installable Codex skills, install curated skills, or install skills from another GitHub repository path.
## Do Not Use When
Do not use when authoring a brand-new skill locally; use skill-creator for authoring.
## Required Inputs
Skill source, curated or GitHub path, destination CODEX_HOME, desired skill names, and network/auth availability.
## Workflow
1. List curated or experimental skills if needed.
2. Resolve source repo/path.
3. Run the installer script or equivalent download.
4. Tell the user to restart Codex and verify installation.
