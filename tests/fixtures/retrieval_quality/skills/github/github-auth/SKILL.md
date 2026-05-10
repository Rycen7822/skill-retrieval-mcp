---
name: github-auth
description: Set up, repair, and verify GitHub authentication with gh auth login, git credential helpers, SSH keys, and token scopes.
tags: [github, authentication, gh, credentials, ssh, token]
---
# GitHub Auth
## When to Use
Use when the user needs GitHub authentication, gh CLI login, credential helper repair, SSH key setup, token scope checks, or push permission diagnostics.
## Do Not Use When
Do not use when the user asks to review a pull request, create a repository, triage issues, or open a PR from code changes.
## Required Inputs
GitHub host, desired auth method, repository owner, permission scope, and whether network login is allowed.
## Workflow
1. Inspect current gh auth status and git credential configuration.
2. Choose HTTPS token or SSH key authentication.
3. Verify repository access, push permissions, and token scopes.
4. Report safe remediation steps without exposing secrets.
