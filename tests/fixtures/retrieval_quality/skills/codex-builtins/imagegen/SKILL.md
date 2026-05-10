---
name: "imagegen"
description: "Codex .system built-in skill for generating or editing raster bitmap images such as photos, illustrations, textures, sprites, mockups, transparent cutouts, or project visual assets."
tags: ["codex-builtin", "codex-system", "image-generation", "bitmap", "visual-assets"]
---
# imagegen
## When to Use
Use when Codex should create or edit a bitmap image, derive variants from visual references, make a mockup, or produce project image assets.
## Do Not Use When
Do not use for deterministic SVG/vector/code-native assets or HTML/CSS/canvas diagrams that should be edited directly in source code.
## Required Inputs
Image goal, reference images if any, output asset location, dimensions or style constraints, and whether transparency is required.
## Workflow
1. Choose generate or edit mode.
2. Use the built-in image generation path by default.
3. Move project assets from the Codex output location into the workspace.
4. Verify the generated bitmap satisfies the request.
