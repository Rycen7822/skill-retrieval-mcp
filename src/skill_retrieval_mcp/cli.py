from __future__ import annotations

import argparse
import json
from pathlib import Path
from time import perf_counter

from .core import SkillRetrievalEngine, default_cache_path, default_roots
from .models import LoadRequest, SearchRequest


def _parse_roots(raw: str | None) -> list[Path] | None:
    if not raw:
        return None
    return [Path(part).expanduser().resolve() for part in raw.split(":") if part]


def cmd_search(args: argparse.Namespace) -> int:
    engine = SkillRetrievalEngine(roots=_parse_roots(args.roots), cache_path=args.cache)
    request = SearchRequest(
        raw_user_request=args.raw or "",
        description_query=args.description_query or "",
        workflow_query=args.workflow_query or "",
        must_have=args.must_have or [],
        must_not=args.must_not or [],
        nice_to_have=args.nice_to_have or [],
        environment=args.environment or [],
        k=args.k,
        max_tokens=args.max_tokens,
    )
    print(json.dumps(engine.search(request), ensure_ascii=False, indent=2))
    return 0


def cmd_load(args: argparse.Namespace) -> int:
    engine = SkillRetrievalEngine(roots=_parse_roots(args.roots), cache_path=args.cache)
    request = LoadRequest(skill_id_or_handle=args.skill, view=args.view, section_ids=args.section or [], max_tokens=args.max_tokens)
    print(json.dumps(engine.load(request), ensure_ascii=False, indent=2))
    return 0


def cmd_build_index(args: argparse.Namespace) -> int:
    engine = SkillRetrievalEngine(roots=_parse_roots(args.roots), cache_path=args.cache)
    print(json.dumps({
        "cache_status": engine.cache_status,
        "cache_path": str(engine.cache_path),
        "roots": [str(root) for root in engine.roots],
        "total_indexed": len(engine.records),
    }, ensure_ascii=False, indent=2))
    return 0


def cmd_bench(args: argparse.Namespace) -> int:
    build_start = perf_counter()
    engine = SkillRetrievalEngine(roots=_parse_roots(args.roots), cache_path=args.cache)
    build_ms = (perf_counter() - build_start) * 1000
    request = SearchRequest(
        raw_user_request=args.raw or "review code diff and choose the appropriate skill",
        description_query=args.description_query or "code review skill search",
        workflow_query=args.workflow_query or "inspect diff run verification report findings",
        k=args.k,
    )
    start = perf_counter()
    last = None
    for _ in range(args.iterations):
        last = engine.search(request)
    search_ms = ((perf_counter() - start) * 1000) / max(1, args.iterations)
    load_ms = None
    if last and last["results"]:
        load_req = LoadRequest(skill_id_or_handle=last["results"][0]["skill_id"], view="preview", max_tokens=400)
        start = perf_counter()
        for _ in range(args.iterations):
            engine.load(load_req)
        load_ms = ((perf_counter() - start) * 1000) / max(1, args.iterations)
    print(json.dumps({
        "total_indexed": len(engine.records),
        "cache_status": engine.cache_status,
        "build_ms": round(build_ms, 3),
        "avg_search_ms": round(search_ms, 3),
        "avg_load_ms": round(load_ms or 0.0, 3),
        "top_skill": last["results"][0]["skill_id"] if last and last["results"] else None,
    }, ensure_ascii=False, indent=2))
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Skill Retrieval MCP utility CLI")
    parser.add_argument("--roots", help="Skill roots separated by ':'; default SRM_SKILL_ROOTS or ~/.hermes/skills")
    parser.add_argument("--cache", help="Cache path; default SRM_CACHE_PATH or ~/.cache/skill-retrieval-mcp/index.json")
    sub = parser.add_subparsers(dest="cmd", required=True)

    search = sub.add_parser("search", help="Run skill_search outside MCP")
    search.add_argument("--raw")
    search.add_argument("--description-query")
    search.add_argument("--workflow-query")
    search.add_argument("--must-have", action="append")
    search.add_argument("--nice-to-have", action="append")
    search.add_argument("--must-not", action="append")
    search.add_argument("--environment", action="append")
    search.add_argument("-k", type=int, default=3)
    search.add_argument("--max-tokens", type=int, default=1200)
    search.set_defaults(func=cmd_search)

    load = sub.add_parser("load", help="Run skill_load outside MCP")
    load.add_argument("skill")
    load.add_argument("--view", choices=["card", "preview", "runtime", "risk", "sections", "full"], default="preview")
    load.add_argument("--section", action="append")
    load.add_argument("--max-tokens", type=int, default=1200)
    load.set_defaults(func=cmd_load)

    build = sub.add_parser("build-index", help="Build or refresh index cache")
    build.set_defaults(func=cmd_build_index)

    bench = sub.add_parser("bench", help="Run a simple local benchmark")
    bench.add_argument("--raw")
    bench.add_argument("--description-query")
    bench.add_argument("--workflow-query")
    bench.add_argument("-k", type=int, default=3)
    bench.add_argument("--iterations", type=int, default=100)
    bench.set_defaults(func=cmd_bench)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
