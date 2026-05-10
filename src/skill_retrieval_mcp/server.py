from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

from mcp.server.fastmcp import FastMCP

from .core import SkillRetrievalEngine
from .models import LoadRequest, SearchRequest

mcp = FastMCP("skill_retrieval_mcp")
_ENGINE: SkillRetrievalEngine | None = None
_ENGINE_KEY: tuple[str, str] | None = None


def get_engine() -> SkillRetrievalEngine:
    global _ENGINE, _ENGINE_KEY
    key = (os.environ.get("SRM_SKILL_ROOTS", ""), os.environ.get("SRM_CACHE_PATH", ""))
    if _ENGINE is None or _ENGINE_KEY != key:
        _ENGINE = SkillRetrievalEngine()
        _ENGINE_KEY = key
    return _ENGINE


def _json(data: dict[str, Any]) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2)


@mcp.tool(
    name="skill_search",
    annotations={
        "title": "Search local skills with dual-query retrieval",
        "readOnlyHint": True,
        "destructiveHint": False,
        "idempotentHint": True,
        "openWorldHint": False,
    },
)
async def skill_search(params: SearchRequest) -> str:
    """Search indexed skills using raw request, description query, workflow query, and constraints.

    Use this first when a reusable procedure may help. The tool returns short
    skill-level candidates only: handle, skill_id, score, confidence,
    load_decision, why_match, why_maybe_not, risk_flags, provenance, and a card.
    It does not return full skill content. If confidence is ambiguous or the
    candidate says preview_first, call skill_load with view='preview' before
    loading runtime.
    """
    return _json(get_engine().search(params))


@mcp.tool(
    name="skill_load",
    annotations={
        "title": "Load selected local skill view",
        "readOnlyHint": True,
        "destructiveHint": False,
        "idempotentHint": True,
        "openWorldHint": False,
    },
)
async def skill_load(params: LoadRequest) -> str:
    """Load a controlled view of a skill selected by skill_search.

    Accepts a canonical skill_id or handle returned by skill_search. Supported
    views are card, preview, runtime, risk, sections, and full. The tool never
    reads arbitrary paths from input; unknown ids/handles fail with actionable
    errors. Prefer preview for medium/ambiguous candidates and runtime only for
    high-confidence matches.
    """
    try:
        return _json(get_engine().load(params))
    except KeyError as exc:
        return _json({"error": str(exc), "suggestion": "Call skill_search and pass the returned handle or canonical skill_id."})


def main() -> None:
    mcp.run()


if __name__ == "__main__":
    main()
