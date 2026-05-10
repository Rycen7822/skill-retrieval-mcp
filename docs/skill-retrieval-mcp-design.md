# Skill Retrieval MCP 设计文档

## 1. 目标

本项目实现一个面向 Codex / Hermes 等本地 agent 的 Skill Retrieval MCP。核心目标不是把所有 skill description 或完整 SKILL.md 默认注入上下文，而是把 skill 发现、检索、rerank、门控、摘要压缩与加载粒度控制放在 MCP 服务内部，只向模型暴露极少工具描述。

默认暴露两个工具：

1. `skill_search`：根据原始请求、description query、workflow query、硬约束和反约束检索候选 skill。
2. `skill_load`：只加载已经选中的 skill 的受控视图，而不是默认加载全文。

## 2. 设计原则

- 先选 skill，再选内容：全库检索的单位必须先是 skill-level card，不直接对全库 chunk 做最终上下文选择。
- 双 query 只参与 rerank：description query 用于意图/任务类型匹配，workflow query 用于流程/步骤匹配；二者都用于 skill-level 聚合分数，不直接把命中的正文 chunk 注入上下文。
- 低 token 工具面：MCP 只暴露 search/load 两个工具；复杂索引、MMR、风险检测、section 解析都在服务内部完成。
- preview-first：中低置信度或 top 候选接近时，优先返回 preview，避免直接加载 1500+ token 的错误 runtime。
- 反触发优先：`do_not_use_when` / `negative cues` / `must_not` 命中时强降权或禁止自动 runtime load。
- provenance 必须可见：每个返回都带 source path、sha256、更新时间、truncated 状态和 trust level。
- 安全边界清晰：不执行 skill 内容，不暴露任意文件读取；`skill_load` 只能加载索引内的 canonical skill id 或 search handle。

## 3. 检索输入

`skill_search` 接受结构化双 query：

```json
{
  "raw_user_request": "用户原始请求",
  "description_query": "review GitHub pull request for correctness and security",
  "workflow_query": "inspect git diff, identify bugs/security issues, verify with tests, write prioritized findings",
  "must_have": ["git diff", "code review"],
  "nice_to_have": ["security review", "GitHub PR"],
  "must_not": ["implement feature", "push changes"],
  "environment": ["git repository", "gh optional"],
  "k": 3,
  "max_tokens": 1200
}
```

其中：

- `raw_user_request` 用作一致性校验，避免 workflow query 过度扩展用户意图。
- `description_query` 搜索 skill 描述、tags、trigger、category。
- `workflow_query` 搜索 workflow summary、步骤标题、验证流程摘要。
- `must_have` 是硬约束，缺失会降权并写入 `missing_requirements`。
- `must_not` 是反约束，命中会强降权并阻止自动 runtime load。

## 4. 索引结构

每个 skill 在内存索引中对应一个 SkillRecord：

- `skill_id`：稳定 canonical id，例如 `github-code-review` 或 `research/llm-wiki`。
- `name`、`description`、`category`、`tags`。
- `skill_card`：200-400 token 内的短卡。
- `workflow_summary`：只用于 workflow query rerank 的短流程摘要。
- `use_when` / `do_not_use_when`。
- `risk_flags`：例如 `LOCAL_FS_READ`、`LOCAL_FS_WRITE`、`NETWORK`、`PROCESS`、`GIT`、`DESTRUCTIVE_POSSIBLE`。
- `sections`：按 Markdown heading 解析出的 section map。
- `source_path`、`source_sha256`、`mtime_ns`、`size`、`trust_level`。

索引默认扫描：

- `$SRM_SKILL_ROOTS`，以 `:` 分隔；若为空，扫描 `~/.hermes/skills`。
- 每个 skill 必须有 `SKILL.md`。
- cache 路径默认 `$SRM_CACHE_PATH`，若为空则使用 `~/.cache/skill-retrieval-mcp/index.json`。

## 5. 评分与 MMR

总分采用 hybrid 融合：

```text
score_total =
  0.40 * description_score
+ 0.30 * workflow_score
+ 0.15 * raw_request_score
+ 0.10 * metadata_score
+ 0.05 * trust_score
+ bonuses
- penalties
```

- description_score：description/card/tags/use_when 的 token overlap 和 phrase match。
- workflow_score：workflow summary / section headings / steps 的 token overlap。
- raw_request_score：原始用户请求与 card/workflow 的 sanity check。
- metadata_score：category、tags、environment、risk/dependency cue。
- penalties：must_not、negative cue、missing must_have、环境不匹配。

MMR 只在 skill-level 候选上执行，用于减少相似 skill 挤占 top-k。不得直接在全库 chunk 上执行最终上下文选择。

## 6. Search 返回

`skill_search` 返回短候选，而不是正文：

```json
{
  "query_id": "...",
  "confidence": "high|medium|low|ambiguous",
  "results": [
    {
      "handle": "...",
      "skill_id": "github-code-review",
      "score": 0.86,
      "confidence": "high",
      "load_decision": "safe_to_load|preview_first|do_not_auto_load",
      "recommended_view": "runtime|preview|risk|card",
      "why_match": ["description matches PR/code review", "workflow matches inspect diff -> findings"],
      "why_maybe_not": ["does not implement fixes unless explicitly requested"],
      "missing_requirements": [],
      "matched_fields": ["description", "workflow_summary", "tags"],
      "risk_flags": ["LOCAL_FS_READ", "GIT"],
      "card": "..."
    }
  ],
  "ambiguity": {
    "is_ambiguous": false,
    "reason": "top candidate is clearly separated"
  }
}
```

自动加载策略：

- high confidence 且 top1-top2 差距足够：`safe_to_load` + `runtime`。
- medium 或 top1/top2 接近：`preview_first`。
- low 或反约束冲突：`do_not_auto_load`。

## 7. Load 视图

`skill_load` 支持：

- `card`：最短 skill card。
- `preview`：Applicability、Use/Do-not-use、required inputs、workflow one-liner、risk flags。
- `runtime`：执行步骤、pitfalls、verification，默认不含冗长背景。
- `risk`：风险、外部依赖、权限、网络/写文件/命令执行提示。
- `sections`：只加载指定 section ids。
- `full`：完整 SKILL.md，仅显式要求时使用。

所有 view 都必须：

- 前置 applicability check。
- 遵守 `max_tokens`。
- 返回 `truncated` 与 `available_views`。
- 带 `source_path`、`source_sha256`、`updated_at`。

## 8. 误加载风险与修复策略

| 风险 | 修复策略 |
| --- | --- |
| workflow query 命中错误正文 chunk | workflow query 仅 rerank skill；不直接加载 chunk |
| top1/top2 过近导致误选 | 返回 ambiguous + preview_first |
| skill description 泛化过强 | 引入 workflow、must_have、negative cues |
| skill 里有 prompt injection | 返回 provenance/trust/risk；不执行内容；只加载受控视图 |
| cache 过期 | 使用 path + mtime_ns + size + sha256 校验，stale 自动重建 |
| path traversal | load 只接受索引内 skill_id/handle，不接受任意路径 |
| 返回过长污染上下文 | 所有工具有 max_tokens 和裁剪标记 |
| 纯向量漏掉命令/专名 | hybrid lexical + metadata + phrase scoring；可后续接 embedding |

## 9. 性能目标

本地 skill 库常见规模为几十到数百个 skill。目标：

- 冷启动索引构建：数百 skill 下 < 1s 级别。
- 热启动 cache 加载：< 100ms 级别。
- 单次 search：p95 < 30ms（数百 skill）。
- 单次 load：p95 < 20ms（从内存索引加载受控 view）。
- 无网络依赖、无大型 embedding 模型默认下载，保证稳定低延迟。

“高性能”在此定义为本地低延迟、可预测、无远程模型依赖。若未来接向量库，应作为可选离线索引增强，而不是 search 热路径强依赖。

## 10. 验证策略

必须覆盖：

- parser：frontmatter、heading、section id、风险 flags。
- search：双 query 融合、must_have、must_not、ambiguity、MMR。
- load：preview/runtime/risk/sections/full、max_tokens、truncation、handle。
- safety：path traversal、未知 skill、反触发阻止 auto runtime。
- cache：stale 检测、热加载。
- performance：合成 300+ skills 的 search/load 基准。
- MCP：工具 list 与 tool call smoke。

## 11. 信心边界

不能诚实宣称任何软件实现“绝对 100% 无漏洞”。本项目采用事实化信心标准：

- 所有设计需求有对应测试。
- 单测、集成测试、MCP 烟测、性能基准全部通过。
- 自审发现的漏洞有回归测试和修复。
- 剩余风险明确记录，不伪装成数学意义上的 100%。

当测试与审计覆盖当前需求，并且没有已知未修复 blocker 时，才报告“对当前实现达到可验证的高信心”。
