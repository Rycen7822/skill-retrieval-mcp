---
name: blogwatcher
description: Monitor blogs, RSS feeds, Atom feeds, newsletters, and release-note pages for new posts and summarize changes.
tags: [rss, atom, blog, monitoring, feed, newsletter]
---
# Blogwatcher
## When to Use
Use when the user wants recurring or one-shot monitoring of blogs, RSS/Atom feeds, newsletters, changelogs, or release-note pages.
## Do Not Use When
Do not use when the user asks to search arXiv papers, verify a paper citation, or clip one article into a note.
## Required Inputs
Feed URLs, polling cadence, interesting keywords, delivery target, and deduplication horizon.
## Workflow
1. Validate feed URLs or discover RSS/Atom endpoints.
2. Fetch entries, deduplicate against prior runs, and rank interesting updates.
3. Summarize changed posts with source links.
4. Schedule or document the monitoring run.
