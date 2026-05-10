---
name: arxiv
description: Search arXiv, retrieve arXiv paper metadata, resolve arXiv IDs, download PDFs, and inspect abstracts or categories.
tags: [arxiv, paper-search, pdf, metadata, abstract]
---
# arXiv Search
## When to Use
Use when the user asks to search arXiv, fetch an arXiv abstract, resolve an arXiv identifier, download an arXiv PDF, or inspect arXiv categories.
## Do Not Use When
Do not use to write a full manuscript, repair DOCX references, or verify non-arXiv venue reliability by itself.
## Required Inputs
Search query, arXiv id, author, title phrase, category, or date range.
## Workflow
1. Query arXiv metadata.
2. Rank candidate papers by title, author, and abstract match.
3. Retrieve abstract, PDF link, and categories.
4. Report canonical arXiv id and source URL.
