from __future__ import annotations

import json
import os
import sys
from pathlib import Path

import pytest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from conftest import CODE_REVIEW_SKILL, write_skill


@pytest.mark.anyio
async def test_mcp_stdio_exposes_only_search_and_load_and_calls_work(tmp_path: Path) -> None:
    root = tmp_path / "skills"
    write_skill(root, "github/github-code-review", CODE_REVIEW_SKILL)
    env = os.environ.copy()
    env["SRM_SKILL_ROOTS"] = str(root)
    env["SRM_CACHE_PATH"] = str(tmp_path / "cache.json")
    params = StdioServerParameters(command=sys.executable, args=["-m", "skill_retrieval_mcp.server"], env=env)

    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            tools = await session.list_tools()
            names = sorted(tool.name for tool in tools.tools)
            assert names == ["skill_load", "skill_search"]

            search = await session.call_tool("skill_search", {
                "params": {
                    "raw_user_request": "review this GitHub PR",
                    "description_query": "GitHub pull request code review",
                    "workflow_query": "inspect diff report findings",
                    "k": 1,
                }
            })
            search_payload = json.loads(search.content[0].text)
            assert search_payload["results"][0]["skill_id"] == "github-code-review"
            assert "content" not in search_payload["results"][0]

            load = await session.call_tool("skill_load", {
                "params": {
                    "skill_id_or_handle": search_payload["results"][0]["handle"],
                    "view": "preview",
                    "max_tokens": 400,
                }
            })
            load_payload = json.loads(load.content[0].text)
            assert load_payload["skill_id"] == "github-code-review"
            assert "Applicability check" in load_payload["content"]
