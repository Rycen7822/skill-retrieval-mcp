from __future__ import annotations

from pathlib import Path

import pytest

from conftest import CODE_REVIEW_SKILL, write_skill


def test_load_preview_runtime_risk_sections_and_handles(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import LoadRequest, SearchRequest

    root = tmp_path / "skills"
    write_skill(root, "github/github-code-review", CODE_REVIEW_SKILL)
    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")

    search = engine.search(SearchRequest(
        raw_user_request="review this GitHub pull request",
        description_query="GitHub PR code review",
        workflow_query="inspect diff produce findings",
        k=1,
    ))
    handle = search["results"][0]["handle"]

    preview = engine.load(LoadRequest(skill_id_or_handle=handle, view="preview", max_tokens=400))
    assert preview["skill_id"] == "github-code-review"
    assert preview["view"] == "preview"
    assert "Applicability" in preview["content"]
    assert "Do not use" in preview["content"]
    assert preview["source_sha256"]
    assert preview["truncated"] is False

    runtime = engine.load(LoadRequest(skill_id_or_handle="github-code-review", view="runtime", max_tokens=700))
    assert "Workflow" in runtime["content"]
    assert "Verification" in runtime["content"]
    assert runtime["available_views"] == ["card", "preview", "runtime", "risk", "sections", "full"]

    risk = engine.load(LoadRequest(skill_id_or_handle="github-code-review", view="risk", max_tokens=300))
    assert "Risk flags" in risk["content"]
    assert "GIT" in risk["content"]

    section = engine.load(LoadRequest(skill_id_or_handle="github-code-review", view="sections", section_ids=["workflow"], max_tokens=300))
    assert "Inspect repository state" in section["content"]
    assert "Pitfalls" not in section["content"]


def test_load_rejects_unknown_or_path_traversal_handles(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import LoadRequest

    root = tmp_path / "skills"
    write_skill(root, "github/github-code-review", CODE_REVIEW_SKILL)
    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")

    with pytest.raises(KeyError):
        engine.load(LoadRequest(skill_id_or_handle="../secrets", view="full"))

    with pytest.raises(KeyError):
        engine.load(LoadRequest(skill_id_or_handle="/etc/passwd", view="full"))


def test_load_applies_token_budget_and_reports_truncation(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import LoadRequest

    root = tmp_path / "skills"
    long_skill = CODE_REVIEW_SKILL + "\n" + ("extra background text " * 1000)
    write_skill(root, "github/github-code-review", long_skill)
    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")

    loaded = engine.load(LoadRequest(skill_id_or_handle="github-code-review", view="full", max_tokens=80))
    assert loaded["truncated"] is True
    assert loaded["tokens_estimate"] <= 90
    assert loaded["content"].endswith("[TRUNCATED]")
