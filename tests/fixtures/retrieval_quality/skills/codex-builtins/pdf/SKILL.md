---
name: pdf
description: Codex built-in skill for reading, creating, or reviewing PDF files where rendering, page layout, typography, and visual fidelity matter.
tags: [codex-builtin, pdf, rendering, layout]
---
# pdf
## When to Use
Use for PDF generation, PDF review, visual layout checks, rendering pages to images, or extracting content while preserving layout concerns.
## Do Not Use When
Do not use for ordinary Markdown editing or text-only notes where PDF rendering is irrelevant.
## Required Inputs
Input PDF or desired output path, rendering tools such as Poppler, and layout quality criteria.
## Workflow
1. Render pages to images for visual checks.
2. Use reportlab, pdfplumber, or pypdf as appropriate.
3. Regenerate or inspect output.
4. Verify spacing, clipping, fonts, and page transitions.
