from __future__ import annotations

from pathlib import Path

import pytest


EXPECTED_HERMES_AUTONOMOUS_SKILL_IDS = {
    "hermes-agent",
    "codex",
}

EXPECTED_CODEX_SKILL_IDS = {
    "ast-grep",
    "pdf",
    "nvmini",
    "imagegen",
    "openai-docs",
    "plugin-creator",
    "skill-creator",
    "skill-installer",
    "structured-artifact-inspection",
    "rg-budget-search",
    "huggingface-datasets",
    "huggingface-gradio",
    "huggingface-community-evals",
    "huggingface-trackio",
    "huggingface-vision-trainer",
    "huggingface-jobs",
    "hf-cli",
    "huggingface-llm-trainer",
    "huggingface-papers",
    "transformers-js",
    "huggingface-paper-publisher",
    "gh-fix-ci",
    "gh-address-comments",
    "github",
    "yeet",
}

IDENTITY_QUERIES = {
    "hermes-agent": "hermes-agent Hermes Agent config skills gateway providers MCP",
    "codex": "codex Codex CLI autonomous coding agent delegation",
    "ast-grep": "ast-grep structural AST code search patterns",
    "pdf": "pdf render pages layout typography clipping",
    "nvmini": "nvmini NVIDIA GPU CUDA diagnostics nvidia-smi memory",
    "imagegen": "imagegen generate bitmap raster image visual asset mockup",
    "openai-docs": "openai-docs OpenAI API documentation model prompt migration citations",
    "plugin-creator": "plugin-creator scaffold Codex plugin .codex-plugin plugin.json",
    "skill-creator": "skill-creator create update Codex skill frontmatter description workflow",
    "skill-installer": "skill-installer install Codex skills curated GitHub CODEX_HOME",
    "structured-artifact-inspection": "structured-artifact-inspection artifact viewer provenance JSON report",
    "rg-budget-search": "rg-budget-search budgeted ripgrep repository search bounded hits",
    "huggingface-datasets": "huggingface-datasets Dataset Viewer splits rows parquet",
    "huggingface-gradio": "huggingface-gradio Gradio Space demo callbacks queue",
    "huggingface-community-evals": "huggingface-community-evals benchmark evaluation submission metrics",
    "huggingface-trackio": "huggingface-trackio experiment tracking metrics dashboard runs",
    "huggingface-vision-trainer": "huggingface-vision-trainer vision model image dataset transforms checkpoint",
    "huggingface-jobs": "huggingface-jobs remote job hardware logs Hugging Face",
    "hf-cli": "hf-cli Hugging Face Hub CLI upload repo auth",
    "huggingface-llm-trainer": "huggingface-llm-trainer LLM fine-tuning tokenizer trainer LoRA",
    "huggingface-papers": "huggingface-papers paper page linked models datasets Spaces",
    "transformers-js": "transformers-js JavaScript browser inference pipeline model loading",
    "huggingface-paper-publisher": "huggingface-paper-publisher publish paper artifacts model dataset cards",
    "gh-fix-ci": "gh-fix-ci failing GitHub Actions checks PR logs",
    "gh-address-comments": "gh-address-comments address PR review comments unresolved threads",
    "github": "github Codex curated GitHub connector repo issue PR metadata",
    "yeet": "yeet publish local changes commit push draft PR",
}


def _latest_curated_codex_skill_root(family: str) -> Path | None:
    base = Path.home() / ".codex" / "plugins" / "cache" / "openai-curated" / family
    if not base.exists():
        return None
    candidates = sorted((path / "skills" for path in base.iterdir() if (path / "skills").is_dir()), key=lambda p: str(p))
    return candidates[-1] if candidates else None


def _installed_builtin_roots() -> tuple[list[Path], set[str]]:
    roots: list[Path] = []
    expected: set[str] = set()

    hermes_autonomous = Path.home() / ".hermes" / "skills" / "autonomous-ai-agents"
    if hermes_autonomous.exists():
        roots.append(hermes_autonomous)
        expected.update(EXPECTED_HERMES_AUTONOMOUS_SKILL_IDS)

    codex_user_skills = Path.home() / ".codex" / "skills"
    if codex_user_skills.exists():
        roots.append(codex_user_skills)
        expected.update({"ast-grep", "pdf", "nvmini", "imagegen", "openai-docs", "plugin-creator", "skill-creator", "skill-installer"})

    structured_artifacts = Path.home() / ".codex" / "plugins" / "structured-artifact-viewer" / "skills"
    if structured_artifacts.exists():
        roots.append(structured_artifacts)
        expected.add("structured-artifact-inspection")

    rg_guard = Path.home() / ".codex" / "plugins" / "codex-rg-guard" / "skills"
    if rg_guard.exists():
        roots.append(rg_guard)
        expected.add("rg-budget-search")

    hf_curated = _latest_curated_codex_skill_root("hugging-face")
    if hf_curated:
        roots.append(hf_curated)
        expected.update({
            "huggingface-datasets",
            "huggingface-gradio",
            "huggingface-community-evals",
            "huggingface-trackio",
            "huggingface-vision-trainer",
            "huggingface-jobs",
            "hf-cli",
            "huggingface-llm-trainer",
            "huggingface-papers",
            "transformers-js",
            "huggingface-paper-publisher",
        })

    github_curated = _latest_curated_codex_skill_root("github")
    if github_curated:
        roots.append(github_curated)
        expected.update({"gh-fix-ci", "gh-address-comments", "github", "yeet"})

    return roots, expected


def test_installed_hermes_and_codex_builtin_skill_roots_are_indexable(tmp_path: Path) -> None:
    from skill_retrieval_mcp.core import SkillRetrievalEngine
    from skill_retrieval_mcp.models import SearchRequest

    roots, expected = _installed_builtin_roots()
    if not roots:
        pytest.skip("No local Hermes/Codex built-in skill roots found")

    engine = SkillRetrievalEngine(roots=roots, cache_path=tmp_path / "builtin-skill-roots-cache.json")
    indexed = {record.skill_id for record in engine.records}

    assert expected.issubset(indexed)

    for skill_id in sorted(expected):
        query = IDENTITY_QUERIES[skill_id]
        result = engine.search(SearchRequest(
            raw_user_request=query,
            description_query=query,
            workflow_query=query,
            k=3,
            max_tokens=1200,
        ))
        ranked_ids = [item["skill_id"] for item in result["results"]]
        assert ranked_ids and ranked_ids[0] == skill_id
