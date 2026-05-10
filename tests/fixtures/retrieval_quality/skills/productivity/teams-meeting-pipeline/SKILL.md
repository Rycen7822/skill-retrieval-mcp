---
name: teams-meeting-pipeline
description: Process Microsoft Teams meeting transcripts, recordings, speaker turns, summaries, action items, and follow-up notes through a local pipeline.
tags: [teams, meeting, transcript, summary, action-items]
---
# Teams Meeting Pipeline
## When to Use
Use when the user asks to summarize a Teams meeting, process transcript files, extract action items, identify speakers, or generate follow-up meeting notes.
## Do Not Use When
Do not use for academic paper writing, GitHub workflows, or llm-wiki clipping.
## Required Inputs
Teams transcript, recording export, chat log, or meeting folder.
## Workflow
1. Locate transcript and meeting metadata.
2. Segment speaker turns and timestamps.
3. Summarize decisions, action items, owners, and deadlines.
4. Export follow-up notes.
