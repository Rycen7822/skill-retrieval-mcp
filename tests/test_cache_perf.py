from __future__ import annotations

from pathlib import Path

from conftest import CODE_REVIEW_SKILL, write_skill


def test_cache_is_reused_and_invalidated_when_skill_changes(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import SearchRequest

    root = tmp_path / "skills"
    path = write_skill(root, "github/github-code-review", CODE_REVIEW_SKILL)
    cache_path = tmp_path / "cache.json"

    first = SkillRetrievalEngine(roots=[root], cache_path=cache_path)
    assert cache_path.exists()
    assert first.cache_status in {"rebuilt", "loaded"}

    second = SkillRetrievalEngine(roots=[root], cache_path=cache_path)
    assert second.cache_status == "loaded"

    changed = CODE_REVIEW_SKILL.replace("security", "cryptographic security")
    path.write_text(changed, encoding="utf-8")

    third = SkillRetrievalEngine(roots=[root], cache_path=cache_path)
    assert third.cache_status == "rebuilt"
    result = third.search(SearchRequest(
        raw_user_request="review crypto security in a PR",
        description_query="cryptographic security code review",
        workflow_query="inspect diff",
        k=1,
    ))
    assert result["results"][0]["skill_id"] == "github-code-review"


def test_search_and_load_are_fast_for_hundreds_of_skills(tmp_path: Path) -> None:
    from time import perf_counter

    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import LoadRequest, SearchRequest

    root = tmp_path / "skills"
    for idx in range(360):
        body = CODE_REVIEW_SKILL.replace("github-code-review", f"synthetic-skill-{idx}")
        body = body.replace("Review GitHub", f"Synthetic workflow {idx} for GitHub")
        write_skill(root, f"category-{idx % 12}/synthetic-skill-{idx}", body)
    target = CODE_REVIEW_SKILL.replace("github-code-review", "special-security-review")
    target = target.replace("correctness, security", "cryptographic API misuse and authorization security")
    write_skill(root, "security/special-security-review", target)

    build_start = perf_counter()
    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")
    build_ms = (perf_counter() - build_start) * 1000
    assert build_ms < 2500

    request = SearchRequest(
        raw_user_request="review cryptographic API misuse in this pull request",
        description_query="cryptographic API misuse security review",
        workflow_query="inspect diff authorization crypto findings",
        k=5,
    )
    start = perf_counter()
    for _ in range(50):
        result = engine.search(request)
    search_ms = ((perf_counter() - start) * 1000) / 50
    assert result["results"][0]["skill_id"] == "special-security-review"
    assert search_ms < 30

    load_start = perf_counter()
    for _ in range(50):
        loaded = engine.load(LoadRequest(skill_id_or_handle="special-security-review", view="preview", max_tokens=300))
    load_ms = ((perf_counter() - load_start) * 1000) / 50
    assert loaded["skill_id"] == "special-security-review"
    assert load_ms < 20
