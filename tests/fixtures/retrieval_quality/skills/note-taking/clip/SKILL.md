---
name: clip
description: Clip URLs, PDFs, arXiv pages, WeChat articles, and technical blog posts into llm-wiki raw notes with localized images and source metadata.
tags: [clipping, url, pdf, arxiv, llm-wiki, raw-notes]
---
# Clip Into LLM Wiki
## When to Use
Use when the user gives a URL, PDF, arXiv link, web article, or blocked tech blog and wants it saved as a raw llm-wiki note.
## Do Not Use When
Do not use for a broad wiki maintenance pass or thesis writing after the source has already been captured.
## Required Inputs
A source URL or local PDF plus the llm-wiki raw note destination.
## Workflow
1. Fetch or render the source page.
2. Extract article text, metadata, abstract, and images.
3. Save a canonical raw markdown note and localized assets.
4. Verify source link, title, images, and index references.
