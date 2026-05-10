---
name: systematic-debugging
description: Investigate bugs, failed tests, crashes, regressions, and unexpected behavior by reproducing, tracing root cause, and then fixing with evidence.
tags: [debugging, root-cause, regression, failure, troubleshooting]
---
# Systematic Debugging
## When to Use
Use when tests fail, a program crashes, behavior is unexpected, a regression appears, or the user asks for root cause analysis before changing code.
## Do Not Use When
Do not use for planned feature implementation with no bug, general PR workflow, or research paper verification.
## Required Inputs
Error message, failing command, logs, reproduction steps, or suspected component.
## Workflow
1. Reproduce the failure consistently.
2. Read the full error and inspect recent changes.
3. Trace data flow to isolate the root cause.
4. Add a regression test and implement one evidence-backed fix.
