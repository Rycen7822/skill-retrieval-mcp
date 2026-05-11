<div align="center">

# Skill Retrieval MCP

**Low-context local skill retrieval MCP for agents that avoids loading every `SKILL.md` into context.**

Search short skill cards first, then lazily load only the selected runtime view.

</div>

<br/>

<p align="center">
  <a href="README.md"><img src="https://img.shields.io/badge/Docs-README-f5c542?style=for-the-badge" alt="Docs"></a>
  <a href=".mcp.example.json"><img src="https://img.shields.io/badge/MCP-two%20tools-2ea44f?style=for-the-badge" alt="MCP: two tools"></a>
  <a href="rust-version/README.md"><img src="https://img.shields.io/badge/Runtime-Python%20%7C%20Rust-blue?style=for-the-badge" alt="Runtime: Python and Rust"></a>
  <a href="https://github.com/Rycen7822/skill-retrieval-mcp/releases"><img src="https://img.shields.io/badge/Rust%20Prebuilt-planned-0969da?style=for-the-badge" alt="Rust prebuilt package planned"></a>
</p>

> Skill Retrieval MCP is meant for agent environments with many local skills.
> For tiny skill libraries or one-off manual inspection, directly reading a known
> `SKILL.md` may be simpler.

Full design: `docs/skill-retrieval-mcp-design.md`.
Self-audit and verification notes: `docs/SELF_AUDIT.md`.
Retrieval quality evaluation: `docs/RETRIEVAL_QUALITY_EVAL.md`.

## Why

Instead of injecting every skill description or full `SKILL.md` into Codex or
another coding agent's context, this MCP keeps indexing, reranking, risk cues,
and view selection inside the MCP process. The agent sees only two tools and
loads content only when needed.

The intended flow is:

1. The agent sends `raw_user_request`, `description_query`, and `workflow_query`
   to `skill_search`.
2. MCP returns compact skill-level candidates with `why_match`,
   `why_maybe_not`, `confidence`, and `load_decision`.
3. The agent loads `preview` for ambiguous candidates or `runtime` for clearly
   safe candidates using `skill_load`.
4. MCP never reads arbitrary paths from inputs; loading is restricted to indexed
   skill ids or search handles.

## Features

- **Two MCP tools**: `skill_search` and `skill_load` keep the agent tool surface small.
- **Dual-query retrieval**: combines raw request, description intent, workflow cues, requirements, environment, category, and negative constraints.
- **Lazy controlled loading**: `card`, `preview`, `runtime`, `risk`, `sections`, and `full` views.
- **Safety and provenance**: canonical skill ids, search handles, source paths, SHA-256 hashes, risk flags, and path traversal rejection.
- **Budgeted output**: compact cards and response trimming for low-token retrieval.
- **Evaluation tooling**: labelled retrieval cases, accuracy/precision gates, entropy, TF-IDF cosine, and MMR novelty checks.
- **Two runtimes**: Python implementation for editable source use; Rust implementation for native binaries and lower runtime overhead.

## Install

| Variant | Best for | Runtime requirements | Status |
| --- | --- | --- | --- |
| Rust prebuilt package | Normal use and distribution on supported hosts | Linux x86_64 package once released | Planned; no package is published yet |
| Python version | Editing, debugging, or using the original implementation | Python 3.11+, `uv` | Available |
| Rust from source | Native local use, development, and platform-specific builds | Rust toolchain | Available |

Default skill root is `~/.hermes/skills`. Override with `SRM_SKILL_ROOTS` using
`:`-separated paths. Override the cache with `SRM_CACHE_PATH` or CLI `--cache`.

### Agent Self-Install Prompts

Copy the matching prompt into a Codex or other coding agent when you want it to
install the MCP for itself.

Rust prebuilt package, recommended once release assets exist:

```text
Install Skill Retrieval MCP for yourself globally from repo `https://github.com/Rycen7822/skill-retrieval-mcp`. Prefer the Rust prebuilt package if a release asset named like `skill-retrieval-mcp-rust-<version>-<target>.tar.gz` exists; install it under `~/.codex/plugins/skill-retrieval-mcp`, register MCP `skill-retrieval` to `~/.codex/plugins/skill-retrieval-mcp/bin/skill-retrieval-mcp-rust serve`, set `SRM_SKILL_ROOTS` to the local skill library, and verify with `skill-retrieval-mcp-rust --help`, `codex mcp list`, `tools/list`, `skill_search`, and `skill_load`. If no Rust prebuilt asset exists yet, stop and use the Rust-from-source prompt instead.
```

Python version, for editing or debugging the source implementation:

```text
Install the Python/source version of Skill Retrieval MCP for yourself from repo `https://github.com/Rycen7822/skill-retrieval-mcp`, not the Rust prebuilt package. Put the source tree under `~/.codex/plugins/skill-retrieval-mcp`, run `uv sync --extra dev`, register MCP `skill-retrieval` with `uv run --project ~/.codex/plugins/skill-retrieval-mcp skill-retrieval-mcp`, set `SRM_SKILL_ROOTS` to the local skill library, and verify with `uv run --project ~/.codex/plugins/skill-retrieval-mcp srm build-index`, `codex mcp list`, `tools/list`, `skill_search`, and `skill_load`.
```

Rust from source, for current native installs and platform-specific builds:

```text
Build Skill Retrieval MCP from source for your platform by cloning `https://github.com/Rycen7822/skill-retrieval-mcp`, then running `cd rust-version && cargo build --release --bins && cargo test`. Install or copy `rust-version/target/release/skill-retrieval-mcp-rust` under `~/.codex/plugins/skill-retrieval-mcp/bin`, register MCP `skill-retrieval` to that binary with argument `serve`, set `SRM_SKILL_ROOTS` to the local skill library, and verify with `skill-retrieval-mcp-rust --help`, `codex mcp list`, `tools/list`, `skill_search`, and `skill_load`.
```

### Rust Prebuilt Package

No Rust prebuilt package is published yet. This section documents the expected
install shape so future release assets stay consistent with the other tools in
this workspace.

Expected future release asset:

```text
https://github.com/Rycen7822/skill-retrieval-mcp/releases/download/v<version>/skill-retrieval-mcp-rust-<version>-<target>.tar.gz
```

Expected future install flow:

```bash
mkdir -p ~/.codex/plugins/skill-retrieval-mcp
curl -L -o skill-retrieval-mcp-rust-<version>-<target>.tar.gz \
  https://github.com/Rycen7822/skill-retrieval-mcp/releases/download/v<version>/skill-retrieval-mcp-rust-<version>-<target>.tar.gz
tar -xzf skill-retrieval-mcp-rust-<version>-<target>.tar.gz -C ~/.codex/plugins/skill-retrieval-mcp --strip-components=1
~/.codex/plugins/skill-retrieval-mcp/bin/skill-retrieval-mcp-rust --help
```

Fallback direct MCP registration, once the binary exists:

```bash
codex mcp add skill-retrieval -- \
  ~/.codex/plugins/skill-retrieval-mcp/bin/skill-retrieval-mcp-rust serve
```

Until a prebuilt package is released, use **Rust From Source** below.

### Python Version

Use this path when you want the editable Python implementation or want to debug
FastMCP / Pydantic behavior directly.

```bash
git clone https://github.com/Rycen7822/skill-retrieval-mcp.git
cd skill-retrieval-mcp
uv sync --extra dev
uv run srm build-index
uv run srm search \
  --raw 'review this PR for correctness and security' \
  --description-query 'GitHub pull request code review' \
  --workflow-query 'inspect git diff write findings' \
  -k 3
uv run srm load github-code-review --view preview --max-tokens 500
```

If this project directory is moved or renamed, refresh uv's virtualenv metadata
and console-script shebangs before running MCP stdio tests:

```bash
uv venv --allow-existing --prompt skill-retrieval-mcp
uv sync --extra dev --reinstall
```

Fallback direct MCP registration:

```bash
codex mcp add skill-retrieval -- \
  uv run --project /absolute/path/to/skill-retrieval-mcp skill-retrieval-mcp
```

Example local MCP config:

```json
{
  "mcpServers": {
    "skill-retrieval": {
      "command": "uv",
      "args": ["run", "--project", "/absolute/path/to/skill-retrieval-mcp", "skill-retrieval-mcp"],
      "env": {
        "SRM_SKILL_ROOTS": "/home/xu/.hermes/skills"
      }
    }
  }
}
```

### Rust From Source

Build the Rust binary locally:

```bash
git clone https://github.com/Rycen7822/skill-retrieval-mcp.git
cd skill-retrieval-mcp/rust-version
cargo build --release --bins
cargo test
target/release/skill-retrieval-mcp-rust --help
```

Run Rust CLI smoke checks:

```bash
target/release/skill-retrieval-mcp-rust build-index
target/release/skill-retrieval-mcp-rust search \
  --raw 'Need Obsidian backlink repair help' \
  --description-query 'obsidian vault backlinks wikilinks notes' \
  --workflow-query 'search vault edit links' \
  --category note-taking \
  -k 2
target/release/skill-retrieval-mcp-rust load obsidian --view preview
target/release/skill-retrieval-mcp-rust bench --iterations 100
```

Fallback direct MCP registration:

```bash
codex mcp add skill-retrieval -- \
  /absolute/path/to/skill-retrieval-mcp/rust-version/target/release/skill-retrieval-mcp-rust serve
```

Example local MCP config:

```json
{
  "mcpServers": {
    "skill-retrieval-rust": {
      "command": "/absolute/path/to/skill-retrieval-mcp/rust-version/target/release/skill-retrieval-mcp-rust",
      "args": ["serve"],
      "env": {
        "SRM_SKILL_ROOTS": "/home/xu/.hermes/skills"
      }
    }
  }
}
```

See `rust-version/README.md` for Rust-specific notes.

## How It Works

Use skill retrieval as a staged workflow:

1. Search with task intent and workflow cues, not with raw full skill files.
2. Inspect compact candidates and ambiguity / risk signals.
3. Load `preview` when choosing among candidates.
4. Load `runtime` or targeted `sections` only after a skill is selected.
5. Fall back to `full` only when the full skill body is required.

The goal is to spend context on the selected procedure, not the entire skill
library.

## MCP Operations

The MCP exposes two tools:

| Tool | Purpose |
| --- | --- |
| `skill_search` | Search indexed local skills using raw request, description query, workflow query, and constraints. |
| `skill_load` | Load a controlled view of an indexed skill selected by canonical id or a search handle. |

### `skill_search`

Input is wrapped by FastMCP / MCP clients as `{ "params": SearchRequest }`.
Important fields:

| Field | Purpose |
| --- | --- |
| `raw_user_request` | Original user request for sanity checks and lexical cues. |
| `description_query` | Intent/type query for skill-card retrieval. |
| `workflow_query` | Procedure query for workflow-summary reranking. |
| `must_have` | Hard requirement cues. |
| `nice_to_have` | Soft preference cues. |
| `must_not` | Positive skill requirements that must not be present. |
| `environment` | Runtime or ecosystem cues. |
| `category` | Optional skill category filter. |
| `k` | Candidate count, 1-10. |
| `max_tokens` | Approximate response budget. |

### `skill_load`

Input is wrapped as `{ "params": LoadRequest }`.
Important fields:

| Field | Purpose |
| --- | --- |
| `skill_id_or_handle` | Canonical `skill_id` or handle returned by `skill_search`. |
| `view` | `card`, `preview`, `runtime`, `risk`, `sections`, or `full`. |
| `section_ids` | Section ids to load when `view` is `sections`. |
| `max_tokens` | Approximate content budget. |

## CLI

Python CLI:

```bash
uv run srm build-index
uv run srm search \
  --raw 'review this PR for correctness and security' \
  --description-query 'GitHub pull request code review' \
  --workflow-query 'inspect git diff write findings' \
  -k 3
uv run srm load github-code-review --view preview --max-tokens 500
uv run srm bench --iterations 200
uv run python scripts/evaluate_retrieval_quality.py --json-out reports/retrieval-quality/latest.json
```

Rust CLI:

```bash
cd rust-version
cargo run -- build-index
cargo run -- search \
  --raw 'Need Obsidian backlink repair help' \
  --description-query 'obsidian vault backlinks wikilinks notes' \
  --workflow-query 'search vault edit links' \
  --category note-taking \
  -k 2
cargo run -- load obsidian --view preview
cargo run -- bench --iterations 100
cargo run -- evaluate \
  --cases ../tests/fixtures/retrieval_quality/cases.jsonl \
  --skill-root ../tests/fixtures/retrieval_quality/skills \
  --skip-diversity
```

## Project Layout

| Path | Purpose |
| --- | --- |
| `.mcp.example.json` | Example Python MCP registration. |
| `src/skill_retrieval_mcp/` | Python core, models, CLI, and FastMCP server. |
| `scripts/evaluate_retrieval_quality.py` | Python labelled retrieval evaluation runner. |
| `scripts/stress_skill_retrieval.py` | Python stress / matrix test runner. |
| `rust-version/` | Pure Rust CLI / MCP implementation and tests. |
| `tests/` | Python unit, integration, cache, and retrieval quality tests. |
| `tests/fixtures/retrieval_quality/` | Labelled fixture cases and synthetic skill library. |
| `docs/` | Design, audit, and evaluation notes. |
| `reports/` | Generated evaluation / stress report outputs. |

## Development

Python checks:

```bash
uv run --extra dev pytest -q
uv run pytest tests/test_retrieval_quality.py -q
uv run pytest tests/test_builtin_skill_roots.py -q
uv run python scripts/evaluate_retrieval_quality.py --json-out reports/retrieval-quality/latest.json
```

Rust checks:

```bash
cd rust-version
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Current Performance on This Machine

Using `/home/xu/.hermes/skills` with 60 indexed skills:

- cache build/load smoke: OK
- average search latency over 200 iterations: about 0.94 ms
- average preview load latency over 200 iterations: about 0.002 ms

Synthetic regression tests cover 361 skills and enforce:

- build < 2500 ms
- average search < 30 ms
- average preview load < 20 ms

## Notes

- The implementation is local and deterministic by default.
- It does not download or run embedding models in the hot path.
- Optional vector backends can be added later as offline index enhancers, but should not replace skill-level gating.
- Rust prebuilt packaging is intentionally documented before release so future archives keep a stable install shape.
