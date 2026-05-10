---
name: nvmini
description: Codex built-in skill for lightweight NVIDIA GPU diagnostics and local CUDA visibility checks before running GPU-heavy experiments or benchmarks.
tags: [codex-builtin, nvidia, gpu, cuda, diagnostics]
---
# nvmini
## When to Use
Use when the task needs quick GPU inventory, CUDA availability, memory pressure, driver visibility, or nvidia-smi style diagnostics.
## Do Not Use When
Do not use for unrelated CPU-only Python debugging or broad system administration without GPU context.
## Required Inputs
Local machine access and permission to run read-only GPU diagnostic commands.
## Workflow
1. Check nvidia-smi or equivalent availability.
2. Record GPU model, memory, driver, CUDA visibility, and active processes.
3. Summarize whether the environment is ready for the requested workload.
