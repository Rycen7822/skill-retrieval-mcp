use std::fs;
use std::process::Command;

use skill_retrieval_mcp_rust::SkillRetrievalEngine;
use skill_retrieval_mcp_rust::evaluation::{
    evaluate_case_diversity, evaluate_retrieval_cases, load_retrieval_cases,
};
use tempfile::tempdir;

const CODE_REVIEW_SKILL: &str = r#"---
name: github-code-review
description: Review GitHub pull requests by inspecting diffs and writing findings.
tags: [github, pull-request, review, security]
---

# GitHub Code Review

## When to Use
Use for reviewing PR diffs for bugs, regressions, and security concerns.

## Do Not Use When
Do not use when creating a pull request or pushing commits.

## Workflow
Fetch the PR diff, inspect changed files, run targeted tests, and summarize findings.
"#;

const PR_WORKFLOW_SKILL: &str = r#"---
name: github-pr-workflow
description: Create branches, commit changes, push, open pull requests, and merge.
tags: [github, pr, push]
---

# GitHub PR Workflow

## When to Use
Use when implementing changes and opening or merging a pull request.

## Workflow
Create a branch, commit files, push to GitHub, and open a PR.
"#;

fn write_skill(root: &std::path::Path, rel: &str, text: &str) {
    let dir = root.join(rel);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), format!("{}\n", text.trim())).unwrap();
}

fn write_cases(path: &std::path::Path) {
    fs::write(
        path,
        r#"{"case_id":"case-review","intent":"direct","difficulty":"easy","language":"en","relevant_skill_ids":["github-code-review"],"expected_top1":"github-code-review","forbidden_skill_ids":["github-pr-workflow"],"raw_user_request":"review this GitHub PR","description_query":"GitHub pull request code review security","workflow_query":"inspect diff report findings","category":"github","k":1,"max_tokens":900}
{"case_id":"case-pr","intent":"paraphrase","difficulty":"medium","language":"en","relevant_skill_ids":["github-pr-workflow"],"expected_top1":"github-pr-workflow","forbidden_skill_ids":["github-code-review"],"raw_user_request":"open a pull request after committing my changes","description_query":"create branch commit push open PR","workflow_query":"commit files push to GitHub open pull request","category":"github","k":1,"max_tokens":900}
{"case_id":"case-review-zh","intent":"zh","difficulty":"medium","language":"zh","relevant_skill_ids":["github-code-review"],"expected_top1":"github-code-review","forbidden_skill_ids":["github-pr-workflow"],"raw_user_request":"请审查这个 GitHub PR 的安全问题","description_query":"GitHub pull request code review security","workflow_query":"inspect diff report findings","category":"github","k":1,"max_tokens":900}
"#,
    )
    .unwrap();
}

#[test]
fn evaluation_api_loads_cases_and_reports_retrieval_metrics() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("skills");
    write_skill(&root, "github/github-code-review", CODE_REVIEW_SKILL);
    write_skill(&root, "github/github-pr-workflow", PR_WORKFLOW_SKILL);
    let cases_path = tmp.path().join("cases.jsonl");
    write_cases(&cases_path);

    let cases = load_retrieval_cases(&cases_path).unwrap();
    assert_eq!(cases.len(), 3);

    let engine =
        SkillRetrievalEngine::new_with_paths(vec![root], tmp.path().join("cache.json")).unwrap();
    let report = evaluate_retrieval_cases(&engine, &cases).unwrap();
    assert_eq!(report["case_count"], 3);
    assert_eq!(report["top1_accuracy"], 1.0);
    assert_eq!(
        report["forbidden_topk_violations"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    let diversity = evaluate_case_diversity(&cases, 0.86, 0.65, 0.28, 5);
    assert_eq!(diversity["case_count"], 3);
    assert!(diversity["language_entropy_norm"].as_f64().unwrap() > 0.0);
}

#[test]
fn cli_exposes_bench_and_evaluate_without_python_runtime() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("skills");
    write_skill(&root, "github/github-code-review", CODE_REVIEW_SKILL);
    write_skill(&root, "github/github-pr-workflow", PR_WORKFLOW_SKILL);
    let cases_path = tmp.path().join("cases.jsonl");
    write_cases(&cases_path);

    let bench = Command::new(env!("CARGO_BIN_EXE_skill-retrieval-mcp-rust"))
        .arg("--roots")
        .arg(&root)
        .arg("--cache")
        .arg(tmp.path().join("bench-cache.json"))
        .arg("bench")
        .arg("--iterations")
        .arg("3")
        .output()
        .unwrap();
    assert!(
        bench.status.success(),
        "{}",
        String::from_utf8_lossy(&bench.stderr)
    );
    let bench_json: serde_json::Value = serde_json::from_slice(&bench.stdout).unwrap();
    assert_eq!(bench_json["total_indexed"], 2);
    assert!(bench_json["avg_search_ms"].as_f64().unwrap() >= 0.0);

    let eval = Command::new(env!("CARGO_BIN_EXE_skill-retrieval-mcp-rust"))
        .arg("evaluate")
        .arg("--cases")
        .arg(&cases_path)
        .arg("--skill-root")
        .arg(&root)
        .arg("--cache")
        .arg(tmp.path().join("eval-cache.json"))
        .arg("--skip-diversity")
        .output()
        .unwrap();
    assert!(
        eval.status.success(),
        "{}",
        String::from_utf8_lossy(&eval.stderr)
    );
    let eval_json: serde_json::Value = serde_json::from_slice(&eval.stdout).unwrap();
    assert_eq!(eval_json["case_count"], 3);
    assert_eq!(eval_json["threshold_failures"].as_array().unwrap().len(), 0);
}
