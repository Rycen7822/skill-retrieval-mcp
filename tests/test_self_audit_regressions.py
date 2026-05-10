from __future__ import annotations

import json
from pathlib import Path

from conftest import CODE_REVIEW_SKILL, write_skill


def test_missing_must_have_blocks_direct_runtime_load(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import SearchRequest

    root = tmp_path / "skills"
    write_skill(root, "github/github-code-review", CODE_REVIEW_SKILL)
    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")

    result = engine.search(SearchRequest(
        raw_user_request="review this pull request",
        description_query="code review pull request security",
        workflow_query="inspect diff and report findings",
        must_have=["kubernetes cluster"],
        k=1,
    ))

    candidate = result["results"][0]
    assert candidate["missing_requirements"] == ["kubernetes cluster"]
    assert candidate["load_decision"] == "preview_first"
    assert candidate["recommended_view"] == "preview"


def test_search_response_respects_token_budget_with_many_long_cards(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine, estimate_tokens
    from skill_retrieval_mcp.models import SearchRequest

    root = tmp_path / "skills"
    for idx in range(12):
        body = CODE_REVIEW_SKILL.replace("github-code-review", f"long-skill-{idx}")
        body = body.replace("Review GitHub pull requests", "Review GitHub pull requests " + ("long contextual detail " * 80))
        write_skill(root, f"long/long-skill-{idx}", body)

    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")
    result = engine.search(SearchRequest(
        raw_user_request="review GitHub pull request",
        description_query="GitHub pull request review",
        workflow_query="inspect diff findings",
        k=10,
        max_tokens=200,
    ))

    encoded = json.dumps(result, ensure_ascii=False)
    assert result["truncated"] is True
    assert result["tokens_estimate"] <= 230
    assert estimate_tokens(encoded) <= 250
