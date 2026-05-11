# Skill Retrieval MCP Rust Version

Pure Rust implementation of the local Skill Retrieval MCP. It mirrors the Python server's core tool and utility surface:

- `skill_search`: dual-query, skill-level retrieval with constraints, MMR, risk flags, provenance, and token-budget compaction.
- `skill_load`: controlled views for indexed skills: `card`, `preview`, `runtime`, `risk`, `sections`, and `full`.
- CLI utilities: `build-index`, `search`, `load`, `bench`, and `evaluate`.
- Persistent JSON cache with file fingerprints and automatic invalidation when indexed `SKILL.md` files change.

## Build and test

```bash
cd /home/xu/project/tools/skill-retrieval-mcp/rust-version
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## CLI examples

```bash
cargo run -- build-index
cargo run -- search \
  --raw "Need Obsidian backlink repair help" \
  --description-query "obsidian vault backlinks wikilinks notes" \
  --workflow-query "search vault edit links" \
  --category note-taking \
  -k 2
cargo run -- load obsidian --view preview
cargo run -- bench --iterations 100
cargo run -- evaluate \
  --cases ../tests/fixtures/retrieval_quality/cases.jsonl \
  --skill-root ../tests/fixtures/retrieval_quality/skills \
  --skip-diversity
```

Use `--roots` to override the skill library. By default it reads `$SRM_SKILL_ROOTS` or `~/.hermes/skills`. Use `--cache` or `$SRM_CACHE_PATH` to override the index cache path; otherwise Rust uses `~/.cache/skill-retrieval-mcp-rust/index.json`.

## MCP stdio

```bash
cargo run -- serve
```

Example client command:

```json
{
  "mcpServers": {
    "skill-retrieval-rust": {
      "command": "/home/xu/project/tools/skill-retrieval-mcp/rust-version/target/debug/skill-retrieval-mcp-rust",
      "args": ["serve"],
      "env": {
        "SRM_SKILL_ROOTS": "/home/xu/.hermes/skills"
      }
    }
  }
}
```

## Notes

This implementation is local and deterministic. It does not use Python, embeddings, or network calls in the hot path. The stdio server implements the MCP JSON-RPC messages needed by local MCP clients for `initialize`, `tools/list`, and `tools/call`.
