from __future__ import annotations

from pathlib import Path

from conftest import CODE_REVIEW_SKILL, IMPLEMENT_SKILL, PR_WORKFLOW_SKILL, write_skill


def test_build_index_extracts_frontmatter_sections_risk_and_stable_ids(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine

    root = tmp_path / "skills"
    path = write_skill(root, "github/github-code-review", CODE_REVIEW_SKILL)

    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")
    records = engine.records

    assert len(records) == 1
    record = records[0]
    assert record.skill_id == "github-code-review"
    assert record.name == "github-code-review"
    assert "security" in record.tags
    assert "workflow" in record.sections
    assert record.sections["workflow"].title == "Workflow"
    assert "LOCAL_FS_READ" in record.risk_flags
    assert "GIT" in record.risk_flags
    assert record.source_path == str(path)
    assert len(record.source_sha256) == 64
    assert record.skill_card


def test_dual_query_ranks_skill_level_candidates_and_returns_load_gate(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import SearchRequest

    root = tmp_path / "skills"
    write_skill(root, "github/github-code-review", CODE_REVIEW_SKILL)
    write_skill(root, "github/github-pr-workflow", PR_WORKFLOW_SKILL)
    write_skill(root, "software/test-driven-development", IMPLEMENT_SKILL)

    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")
    result = engine.search(SearchRequest(
        raw_user_request="Please review this GitHub PR and check security issues in the diff.",
        description_query="review GitHub pull request for correctness and security",
        workflow_query="inspect git diff identify bugs security issues write prioritized review findings",
        must_have=["git diff", "code review"],
        must_not=["implement feature", "push changes"],
        k=3,
        max_tokens=1200,
    ))

    assert result["results"][0]["skill_id"] == "github-code-review"
    assert result["results"][0]["load_decision"] == "safe_to_load"
    assert result["results"][0]["recommended_view"] == "runtime"
    assert "description" in result["results"][0]["matched_fields"]
    assert "workflow_summary" in result["results"][0]["matched_fields"]
    assert result["results"][0]["why_match"]
    assert all("content" not in item for item in result["results"]), "search must not return full skill content"


def test_raw_request_negative_cue_prevents_wrong_auto_load(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import SearchRequest

    root = tmp_path / "skills"
    write_skill(root, "github/github-code-review", CODE_REVIEW_SKILL)
    write_skill(root, "software/test-driven-development", IMPLEMENT_SKILL)

    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")
    result = engine.search(SearchRequest(
        raw_user_request="Implement the feature and push code after tests pass.",
        description_query="implement feature in code",
        workflow_query="write failing test then production code run tests",
        must_have=["implement feature"],
        k=2,
    ))

    assert result["results"][0]["skill_id"] == "test-driven-development"
    code_review = next(item for item in result["results"] if item["skill_id"] == "github-code-review")
    assert code_review["load_decision"] == "do_not_auto_load"
    assert any("do_not_use_when" in reason for reason in code_review["why_maybe_not"])


def test_ambiguous_candidates_require_preview_first(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import SearchRequest

    root = tmp_path / "skills"
    write_skill(root, "a/review-a", CODE_REVIEW_SKILL.replace("github-code-review", "review-a"))
    write_skill(root, "b/review-b", CODE_REVIEW_SKILL.replace("github-code-review", "review-b"))

    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")
    result = engine.search(SearchRequest(
        raw_user_request="review this diff",
        description_query="review diff",
        workflow_query="inspect diff and report findings",
        k=2,
    ))

    assert result["confidence"] == "ambiguous"
    assert result["ambiguity"]["is_ambiguous"] is True
    assert all(item["load_decision"] == "preview_first" for item in result["results"])


def test_mmr_keeps_diverse_skill_candidates(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import SearchRequest

    root = tmp_path / "skills"
    for idx in range(6):
        write_skill(root, f"reviews/review-{idx}", CODE_REVIEW_SKILL.replace("github-code-review", f"review-{idx}"))
    write_skill(root, "implementation/test-driven-development", IMPLEMENT_SKILL)

    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")
    result = engine.search(SearchRequest(
        raw_user_request="I need to review a diff and maybe understand whether feature implementation tests exist.",
        description_query="review code diff and tests",
        workflow_query="inspect diff findings tests implementation",
        k=4,
        mmr_lambda=0.55,
    ))

    ids = [item["skill_id"] for item in result["results"]]
    assert "test-driven-development" in ids, "MMR should keep a procedurally different candidate instead of only near-duplicate review skills"
