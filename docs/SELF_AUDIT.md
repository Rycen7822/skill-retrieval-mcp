# Self Audit / Confidence Loop

This document records the implementation confidence loop for the current Skill Retrieval MCP.

## Baseline position

I cannot honestly claim mathematical or permanent 100% confidence for any software implementation. The practical standard used here is factual confidence for the current stated requirements:

- design requirements have corresponding tests;
- tests fail before implementation or regression fix where applicable;
- all tests, MCP smoke, CLI smoke, py_compile, and benchmarks pass;
- discovered vulnerabilities are either fixed or recorded as explicit residual risks.

## Implemented safeguards

- Only two MCP tools are exposed: `skill_search` and `skill_load`.
- Search returns skill-level candidates, not full skill chunks.
- Workflow query reranks skills; it does not directly inject arbitrary matching chunks.
- `skill_load` accepts only indexed skill ids or handles returned by search.
- Path traversal attempts are rejected.
- `preview_first` is used for ambiguous candidates and missing hard requirements.
- `do_not_auto_load` is used for action-sensitive negative cue conflicts.
- Search responses enforce token budgets while preserving valid JSON.
- Cache invalidates on root/path/mtime/size change and parser cache version.
- Load responses include applicability checks, source hash, trust level, risk flags, and truncation metadata.

## Audit loop 1: package / environment

Finding:

- Initial `uv sync --extra dev` failed because hatch could not infer package files: no `src/skill_retrieval_mcp` directory.

Fix:

- Added `src/skill_retrieval_mcp/__init__.py`.
- Added `[tool.hatch.build.targets.wheel] packages = ["src/skill_retrieval_mcp"]`.

Verification:

- `uv sync --extra dev` passes.

## Audit loop 2: TDD implementation

Red:

- Added parser/search/load/cache/performance tests first.
- Initial test run failed with `ModuleNotFoundError: skill_retrieval_mcp.core`.

Fix:

- Implemented core engine, models, CLI, and MCP server.

Verification:

- Initial full suite reached 10 passing tests after fixing two discovered issues.

## Audit loop 3: negative-cue section extraction

Finding:

- `Do Not Use When` slug initially collapsed to `section` because slugification reused scoring tokenizer and removed stopwords.
- This prevented structured `do_not_use_when` extraction.

Fix:

- Made heading slugification preserve structural words while keeping scoring tokenizer stopword-aware.

Verification:

- Negative-cue test for implementation requests now blocks code-review auto-load.

## Audit loop 4: search latency

Finding:

- First implementation searched 361 synthetic skills at about 177 ms/query because each search repeatedly tokenized every field and recomputed MMR text similarity.

Fix:

- Added per-record prepared token/counter caches.
- Added prepared query objects per search.
- Changed MMR similarity to use precomputed token sets.

Verification:

- Synthetic 361-skill test passes with average search < 30 ms.
- Real 60-skill benchmark: about 0.94 ms/search.

## Audit loop 5: false negative cue conflict

Finding:

- A raw request like “review this pull request” could overlap with a negative sentence like “do not use when creating a pull request” on noun tokens only.
- This could incorrectly mark the right review skill as `do_not_auto_load`.

Fix:

- Added action-sensitive negative cue matching.
- Negative conflict now requires overlap on action tokens plus supporting context, so `review PR` does not trigger `create PR`; `implement feature` still triggers implementation-related do-not-use text.

Verification:

- Added regression test `test_missing_must_have_blocks_direct_runtime_load`.
- Existing negative-cue tests still pass.

## Audit loop 6: search budget enforcement

Finding:

- Search used valid JSON but did not strictly fit very small `max_tokens`; it shrank cards but did not re-check enough and did not expose `tokens_estimate`.

Fix:

- Added `_fit_search_budget` with multi-phase compaction:
  1. shrink cards;
  2. shrink reasons;
  3. reduce candidate count;
  4. ultra-compact provenance when necessary.
- Added `tokens_estimate` and `omitted_results`.

Verification:

- Added regression test for many long cards at `max_tokens=200`.
- Test passes while preserving valid JSON.

## Final verification commands run

```bash
uv run pytest -q
uv run python -m py_compile src/skill_retrieval_mcp/*.py
uv run srm build-index
uv run srm search --raw 'review this PR for correctness and security' --description-query 'GitHub pull request code review' --workflow-query 'inspect git diff write findings' -k 3 --max-tokens 1200
uv run srm load github-code-review --view preview --max-tokens 500
uv run srm bench --iterations 200 --raw 'review this PR for correctness and security' --description-query 'GitHub pull request code review' --workflow-query 'inspect git diff write findings' -k 3
```

Results:

- `pytest`: 13 passed.
- `py_compile`: passed.
- default skill index: 60 skills.
- real-skill benchmark: build/load smoke passed; average search about 0.94 ms; average load about 0.002 ms.
- MCP stdio integration test: lists exactly `skill_search` and `skill_load`; search/load tool calls work.

## Remaining non-blocking risks

- Lexical/hybrid retrieval is deterministic and fast, but not as semantically broad as an embedding backend. This is intentional for hot-path performance; vector support should be optional/offline.
- Risk flags are conservative heuristic labels and may over-report side effects. This is safer than under-reporting but can make risk summaries noisy.
- Existing skills may lack explicit `Do Not Use When` sections. MCP can only gate on available text plus query constraints.
- The FastMCP schema wraps each tool input as `{ "params": ... }`; this is correct for the current implementation but slightly more verbose than fully flattened parameters.
- “Absolute high performance” depends on hardware and skill library size. Current tests and benchmarks demonstrate high performance for hundreds of local skills, not an unbounded guarantee.

## Current confidence statement

There are no known unresolved blockers for the requested local Skill Retrieval MCP. For the current scope — local Hermes-style `SKILL.md` retrieval, two-tool MCP surface, dual-query skill-level search, lazy view loading, cache, safety gates, and hundreds-of-skills performance — the implementation is verified by tests, MCP smoke, CLI smoke, and benchmark. It is not mathematically 100% certain, but it has reached factual high confidence for the stated requirements.
