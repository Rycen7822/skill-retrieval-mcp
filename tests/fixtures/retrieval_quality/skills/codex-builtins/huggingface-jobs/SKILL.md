---
name: huggingface-jobs
description: Codex curated Hugging Face skill for managing Hugging Face Jobs, scheduled runs, job logs, hardware choices, and remote execution.
tags: [codex-builtin, huggingface, jobs, remote-execution, logs]
---
# huggingface-jobs
## When to Use
Use for launching, monitoring, debugging, or summarizing Hugging Face Jobs and their remote logs or hardware settings.
## Do Not Use When
Do not use for local-only uv environment repair or GitHub PR comments.
## Required Inputs
Job command, Hub credentials, hardware target, repo or script path, and log inspection goal.
## Workflow
1. Prepare job command and resources.
2. Launch or inspect the job.
3. Fetch status and logs.
4. Summarize failures or completion evidence.
