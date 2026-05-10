# Skill Retrieval MCP 大规模压力测试问题记录

- 生成时间：2026-05-10T23:19:27+08:00
- 项目目录：`/home/xu/project/tools/skill-retrieval-mcp`
- Skill roots：`/home/xu/.hermes/skills`
- 索引 skill 数：60
- Cache：first=rebuilt (77.446 ms), second=loaded (38.629 ms)
- 确定性 search 场景数：28871
- load 视图预算矩阵调用数：1800
- MCP stdio search/load 调用：10 / 10
- 并发压力时长：60.0 秒，workers=8
- 并发 pressure search 次数：9507

## 结论
本轮定义的场景矩阵未发现失败项；仍建议继续扩大真实任务评测集，因为自然语言输入空间不可数学穷尽。

## 延迟统计

| 操作 | count | avg ms | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|---:|
| build_first | 1 | 77.4457 | 77.4457 | 77.4457 | 77.4457 | 77.4457 |
| build_second | 1 | 38.6288 | 38.6288 | 38.6288 | 38.6288 | 38.6288 |
| load_matrix | 1800 | 0.0024 | 0.002 | 0.0043 | 0.0085 | 0.0386 |
| mcp_load | 10 | 1.4387 | 1.4092 | 1.6236 | 1.6715 | 1.6835 |
| mcp_search | 10 | 8.5943 | 3.6297 | 31.0788 | 48.9062 | 53.3631 |
| pressure_load | 90 | 0.0963 | 0.0379 | 0.0486 | 0.6245 | 5.2053 |
| pressure_search | 9507 | 50.2783 | 46.0171 | 115.2484 | 144.7837 | 208.5475 |
| search_matrix | 28871 | 5.8901 | 5.9856 | 12.2157 | 13.6385 | 16.0785 |

## 待改进问题

- 暂无自动检测到的问题。

## 本轮已修复的问题

- `SEARCH_BUDGET_EXCEEDED`：`_fit_search_budget()` 现在把 `tokens_estimate` 和 `truncated` 字段本身计入最终 JSON 预算；在常规裁剪仍超限时切换到 hard-budget fallback，并保留至少一个候选与完整 `source_sha256`。
- `SEARCH_HASH_TRUNCATED_UNDER_BUDGET`：ultra-compact 模式不再截断 `source_sha256`；压力测试脚本也改为把非 64 位 hash 视为 malformed。
- `SELF_RETRIEVAL_NOT_TOP1` / `SELF_RETRIEVAL_MISSING_TOPK`：新增 skill identity exact-match boost，针对 `skill_id` / `name` / token phrase 做有边界匹配，避免 `github` 这类短 router skill 抢占 `github-repo-management` 等具体 skill。
- `FORGED_SEARCH_HANDLE_ACCEPTED`：`search:*` handle 现在必须来自当前 engine 的 `_handles`；伪造 `search:...:<known_skill_id>` 会被拒绝，canonical `skill_id` 仍可直接加载。
- `README_PATH_STALE` / `RENAMED_VENV_STALE_SHEBANGS`：README 已更新为 `/home/xu/project/tools/skill-retrieval-mcp`，并补充目录重命名后刷新 uv venv/console scripts 的命令。
- `STRESS_CACHE_ARTIFACT_PRIVACY`：压力测试默认索引缓存改到 `.cache/stress-index.json`，并忽略 `reports/stress/*cache*.json`，避免把完整 skill 索引缓存提交。

## 覆盖范围

- 全部 indexed skill 的 exact_dual / description_only / workflow_only / raw_only_zh / tags_category / missing_must_have / must_not_destructive / prompt_injection_like 查询变体。
- k 边界：1、3、10；max_tokens 边界：200、240、320、1200、4000；MMR lambda：0.0、0.3、0.7、1.0。
- 全部 load views：card、preview、runtime、risk、sections、full；load max_tokens：80、160、400、1200、8000。
- SearchRequest / LoadRequest 的 Pydantic 非法输入边界。
- invalid id、路径穿越、unknown id、sections 空 section_ids 等安全/错误边界。
- 真实 MCP stdio server 的 list_tools、skill_search、skill_load 长循环。
- 共享 engine 的多线程并发 pressure search/load 热路径。

## 原始指标文件

- `/home/xu/project/tools/skill-retrieval-mcp/reports/stress/latest-stress-summary.json`
