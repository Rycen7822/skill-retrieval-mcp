from __future__ import annotations

import json
from pathlib import Path

import pytest

from conftest import CODE_REVIEW_SKILL, write_skill


GITHUB_ISSUES_SKILL = """
---
name: github-issues
description: Create, manage, triage, and close GitHub issues. Search existing issues, update labels, and report issue status.
tags: [github, issues, triage, repository]
---

# GitHub Issues

## When to Use

Use when the user asks to create, search, label, triage, close, or update GitHub issues in a repository.

## Do Not Use When

Do not use when the user asks to clone, create, fork, configure, release, or manage whole GitHub repositories.

## Workflow

1. Inspect repository issue settings.
2. Search for existing issues.
3. Create or update issue metadata.
4. Report issue URLs and status.
"""


GITHUB_REPO_MANAGEMENT_SKILL = """
---
name: github-repo-management
description: Clone, create, fork, configure, and manage GitHub repositories. Manage remotes, secrets, releases, and workflows.
tags: [GitHub, Repositories, Git, Releases, Secrets, Configuration]
---

# GitHub Repository Management

## When to Use

Use when the user asks to clone repositories, create repositories, fork projects, configure repository settings, manage remotes, or create releases.

## Do Not Use When

Do not use when the user only asks to triage GitHub issues or review a pull request diff.

## Workflow

1. Inspect local git remote and GitHub account context.
2. Clone, create, fork, configure, or update repository settings.
3. Verify remotes, default branch, visibility, and GitHub API state.
4. Report repository URL and relevant settings.
"""


GENERIC_GITHUB_SKILL = """
---
name: github
description: GitHub utility router for issues, pull requests, code review, repository management, and CI workflows.
tags: [github, router, repository, pull-request, issues]
---

# GitHub Router

## When to Use

Use when the user asks a broad GitHub question and the exact GitHub workflow skill is unclear.

## Workflow

1. Inspect the user intent.
2. Route to issues, pull requests, code review, repository management, or CI workflow.
"""


def test_search_budget_200_is_hard_limit_and_keeps_full_source_hash(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine, estimate_tokens
    from skill_retrieval_mcp.models import SearchRequest

    root = tmp_path / "skills"
    for idx in range(12):
        body = CODE_REVIEW_SKILL.replace("github-code-review", f"long-skill-{idx}")
        body = body.replace("Review GitHub pull requests", "Review GitHub pull requests " + ("long contextual detail " * 120))
        write_skill(root, f"long/long-skill-{idx}", body)

    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")
    result = engine.search(SearchRequest(
        raw_user_request="review GitHub pull request",
        description_query="GitHub pull request review",
        workflow_query="inspect diff findings",
        k=10,
        max_tokens=200,
    ))

    compact_json = json.dumps(result, ensure_ascii=False, separators=(",", ":"))
    assert result["truncated"] is True
    assert result["tokens_estimate"] <= 200
    assert estimate_tokens(compact_json) <= 200
    assert result["results"], "budget compaction must preserve at least one candidate when candidates exist"
    assert len(result["results"][0]["source_sha256"]) == 64
    assert "source_sha256_prefix" not in result["results"][0]


def test_exact_skill_id_query_prefers_self_over_neighboring_github_skills(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import SearchRequest

    root = tmp_path / "skills"
    write_skill(root, "github/github-issues", GITHUB_ISSUES_SKILL)
    write_skill(root, "github/github-repo-management", GITHUB_REPO_MANAGEMENT_SKILL)
    write_skill(root, "github/github", GENERIC_GITHUB_SKILL)

    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")
    result = engine.search(SearchRequest(
        raw_user_request="I need to use the right skill for: github-repo-management. Clone, create, fork, configure, and manage GitHub repositories.",
        description_query="github-repo-management Clone, create, fork, configure, and manage GitHub repositories GitHub Repositories Git Releases Secrets Configuration github",
        workflow_query="Inspect local git remote and GitHub account context. Clone, create, fork, configure, or update repository settings. Verify remotes, default branch, visibility, and GitHub API state.",
        category="github",
        k=3,
        max_tokens=1200,
    ))

    ids = [item["skill_id"] for item in result["results"]]
    assert ids[0] == "github-repo-management"
    assert "github-repo-management" in ids
    assert any("identity" in reason for reason in result["results"][0]["why_match"])


def test_forged_search_handle_is_rejected_but_real_handle_and_canonical_id_work(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import LoadRequest, SearchRequest

    root = tmp_path / "skills"
    write_skill(root, "github/github-code-review", CODE_REVIEW_SKILL)
    engine = SkillRetrievalEngine(roots=[root], cache_path=tmp_path / "cache.json")

    canonical = engine.load(LoadRequest(skill_id_or_handle="github-code-review", view="preview", max_tokens=400))
    assert canonical["skill_id"] == "github-code-review"

    with pytest.raises(KeyError):
        engine.load(LoadRequest(skill_id_or_handle="search:forged:99:github-code-review", view="preview", max_tokens=400))

    search = engine.search(SearchRequest(
        raw_user_request="review this pull request",
        description_query="github code review",
        workflow_query="inspect diff report findings",
        k=1,
    ))
    real_handle = search["results"][0]["handle"]
    loaded = engine.load(LoadRequest(skill_id_or_handle=real_handle, view="preview", max_tokens=400))
    assert loaded["skill_id"] == "github-code-review"


def test_readme_uses_current_project_path_and_documents_venv_refresh() -> None:
    readme = Path("README.md").read_text(encoding="utf-8")
    stale_path = "/home/xu/project/tools/" + "SRM"
    assert stale_path not in readme
    assert "/home/xu/project/tools/skill-retrieval-mcp" in readme
    assert "uv venv --allow-existing --prompt skill-retrieval-mcp" in readme
    assert "uv sync --extra dev --reinstall" in readme


def test_stress_script_keeps_index_cache_out_of_commit_artifacts() -> None:
    script = Path("scripts/stress_skill_retrieval.py").read_text(encoding="utf-8")
    gitignore = Path(".gitignore").read_text(encoding="utf-8")
    assert 'default=".cache/stress-index.json"' in script
    assert "reports/stress/*cache*.json" in gitignore
