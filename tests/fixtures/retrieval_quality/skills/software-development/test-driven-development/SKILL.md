---
name: test-driven-development
description: Implement features and bug fixes by writing failing tests first, then minimal production code, then refactoring with the suite green.
tags: [testing, tdd, implementation, feature, bugfix]
---
# Test Driven Development
## When to Use
Use when the user asks to implement code, add a feature, fix a bug, change behavior, or build production functionality in a repository.
## Do Not Use When
Do not use when the user only asks for diagnosis without changing code, an independent code review, or a paper literature search.
## Required Inputs
A code repository, desired behavior, and a runnable test command.
## Workflow
1. Write a focused failing test that describes the desired behavior.
2. Run the test and confirm it fails for the expected reason.
3. Implement the smallest code change to pass.
4. Run targeted and full tests, then refactor safely.
