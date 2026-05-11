# Skill Retrieval MCP

High-performance local MCP server for token-efficient skill retrieval.

It exposes only two agent-facing tools:

- `skill_search`: dual-query skill-level retrieval with confidence, ambiguity detection, negative-cue gating, MMR diversity, and compact cards.
- `skill_load`: lazy loading of selected skill views: `card`, `preview`, `runtime`, `risk`, `sections`, `full`.

Full design: `docs/skill-retrieval-mcp-design.md`.
Self-audit and verification notes: `docs/SELF_AUDIT.md`.
Retrieval quality evaluation: `docs/RETRIEVAL_QUALITY_EVAL.md` (500 labelled cases / 50 gold skills with accuracy/precision gates plus entropy, TF-IDF cosine, and MMR novelty diversity gates across direct, paraphrase, noisy, typo, low-budget, environment, multilingual, category, multi-relevant, Hermes Agent, and Codex built-in skill scenarios).

## Why this exists

Instead of injecting every skill description or full `SKILL.md` into Codex context, this MCP keeps indexing, reranking, risk cues, and view selection inside the MCP process. Codex sees only two tools and loads content only when needed.

The intended flow is:

1. Codex sends `raw_user_request`, `description_query`, and `workflow_query` to `skill_search`.
2. MCP returns short skill-level candidates with `why_match`, `why_maybe_not`, `confidence`, and `load_decision`.
3. Codex loads `preview` for ambiguous candidates or `runtime` for clearly safe candidates using `skill_load`.
4. MCP never reads arbitrary paths from inputs; loading is restricted to indexed skill ids or search handles.

## Install / sync

```bash
cd /home/xu/project/tools/skill-retrieval-mcp
uv sync --extra dev
```

If this project directory is moved or renamed, refresh uv's virtualenv metadata and console-script shebangs before running MCP stdio tests:

```bash
uv venv --allow-existing --prompt skill-retrieval-mcp
uv sync --extra dev --reinstall
```

## CLI smoke checks

```bash
uv run srm build-index
uv run srm search \
  --raw 'review this PR for correctness and security' \
  --description-query 'GitHub pull request code review' \
  --workflow-query 'inspect git diff write findings' \
  -k 3
uv run srm load github-code-review --view preview --max-tokens 500
uv run srm bench --iterations 200
uv run pytest -q
uv run pytest tests/test_retrieval_quality.py -q
uv run pytest tests/test_builtin_skill_roots.py -q
uv run python scripts/evaluate_retrieval_quality.py --json-out reports/retrieval-quality/latest.json
```

## MCP server

Default skill root is `~/.hermes/skills`. Override with `SRM_SKILL_ROOTS` using `:`-separated paths.

Example local client config:

```json
{
  "mcpServers": {
    "skill-retrieval": {
      "command": "uv",
      "args": ["run", "--project", "/home/xu/project/tools/skill-retrieval-mcp", "skill-retrieval-mcp"],
      "env": {
        "SRM_SKILL_ROOTS": "/home/xu/.hermes/skills"
      }
    }
  }
}
```

## Tool schemas

### `skill_search`

Input is wrapped by FastMCP as `{ "params": SearchRequest }`.

Important fields:

- `raw_user_request`: original user request for sanity checks.
- `description_query`: intent/type query for skill-card retrieval.
- `workflow_query`: procedure query for workflow-summary reranking.
- `must_have`: hard requirement cues.
- `must_not`: positive skill requirements that must not be present.
- `environment`: runtime cues.
- `k`: candidate count, 1-10.
- `max_tokens`: approximate response budget.

### `skill_load`

Input is wrapped as `{ "params": LoadRequest }`.

Important fields:

- `skill_id_or_handle`: canonical `skill_id` or handle from `skill_search`.
- `view`: `card`, `preview`, `runtime`, `risk`, `sections`, or `full`.
- `section_ids`: only for `sections` view.
- `max_tokens`: approximate content budget.

## Current performance on this machine

Using `/home/xu/.hermes/skills` with 60 indexed skills:

- cache build/load smoke: OK
- average search latency over 200 iterations: about 0.94 ms
- average preview load latency over 200 iterations: about 0.002 ms

Synthetic regression test covers 361 skills and enforces:

- build < 2500 ms
- average search < 30 ms
- average preview load < 20 ms

## Limits

This implementation is intentionally local and deterministic. It does not download or run embedding models by default. That keeps latency stable and avoids network/model dependency in the hot path. A future optional vector backend can be added as an offline index enhancer, but should not replace skill-level gating.
