use std::fs;

use skill_retrieval_mcp_rust::{LoadRequest, SearchRequest, SkillRetrievalEngine};
use tempfile::tempdir;

fn write_skill(root: &std::path::Path, category: &str, dir: &str, text: &str) {
    let path = root.join(category).join(dir);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("SKILL.md"), text).unwrap();
}

#[test]
fn parses_frontmatter_sections_risk_and_load_views() {
    let tmp = tempdir().unwrap();
    write_skill(
        tmp.path(),
        "software-development",
        "code-review",
        r#"---
name: code-review
description: Review git diffs for correctness and security.
tags: [git, review, security]
---

# Code Review

## When to Use
Use when reviewing a pull request or git diff.

## Do Not Use When
Do not use when implementing a new feature or pushing changes.

## Required Inputs
A git diff or PR URL.

## Workflow
Inspect the diff, identify bugs, run tests, and report prioritized findings.

## Verification
Run pytest or cargo test when relevant.
"#,
    );

    let engine = SkillRetrievalEngine::new_with_paths(
        vec![tmp.path().to_path_buf()],
        tmp.path().join("cache.json"),
    )
    .unwrap();
    assert_eq!(engine.records().len(), 1);
    let record = &engine.records()[0];
    assert_eq!(record.skill_id, "code-review");
    assert_eq!(record.category, "software-development");
    assert!(record.sections.contains_key("when-to-use"));
    assert!(record.risk_flags.iter().any(|flag| flag == "GIT"));
    assert_eq!(record.source_sha256.len(), 64);

    let loaded = engine
        .load(&LoadRequest {
            skill_id_or_handle: "code-review".to_string(),
            view: "preview".to_string(),
            section_ids: vec![],
            max_tokens: 600,
        })
        .unwrap();
    assert_eq!(loaded["skill_id"], "code-review");
    assert!(
        loaded["content"]
            .as_str()
            .unwrap()
            .contains("Applicability check:")
    );
    assert_eq!(loaded["truncated"], false);
}

#[test]
fn search_ranks_expected_skill_and_enforces_load_safety() {
    let tmp = tempdir().unwrap();
    write_skill(
        tmp.path(),
        "github",
        "github-code-review",
        r#"---
name: github-code-review
description: Review GitHub pull requests by inspecting diffs and writing findings.
tags: [github, pull-request, review]
---

# GitHub Code Review

## When to Use
Use for reviewing PR diffs for bugs, regressions, and security concerns.

## Do Not Use When
Do not use when creating a pull request or pushing commits.

## Workflow
Fetch the PR diff, inspect changed files, run targeted tests, and summarize findings.
"#,
    );
    write_skill(
        tmp.path(),
        "github",
        "github-pr-workflow",
        r#"---
name: github-pr-workflow
description: Create branches, commit changes, push, open pull requests, and merge.
tags: [github, pr, push]
---

# GitHub PR Workflow

## When to Use
Use when implementing changes and opening or merging a pull request.

## Workflow
Create a branch, commit files, push to GitHub, and open a PR.
"#,
    );

    let engine = SkillRetrievalEngine::new_with_paths(
        vec![tmp.path().to_path_buf()],
        tmp.path().join("cache.json"),
    )
    .unwrap();
    let response = engine
        .search(&SearchRequest {
            raw_user_request: "Please review this GitHub pull request, do not push changes."
                .to_string(),
            description_query: "github pull request code review security".to_string(),
            workflow_query: "inspect diff run tests write findings".to_string(),
            must_have: vec!["git diff".to_string(), "review".to_string()],
            must_not: vec!["push changes".to_string()],
            environment: vec!["GitHub".to_string()],
            k: 2,
            max_tokens: 1200,
            ..SearchRequest::default()
        })
        .unwrap();

    let results = response["results"].as_array().unwrap();
    assert_eq!(results[0]["skill_id"], "github-code-review");
    assert_ne!(results[0]["load_decision"], "do_not_auto_load");
    assert!(
        results
            .iter()
            .any(|item| item["skill_id"] == "github-pr-workflow")
    );

    let handle = results[0]["handle"].as_str().unwrap().to_string();
    let loaded_by_handle = engine
        .load(&LoadRequest {
            skill_id_or_handle: handle,
            view: "runtime".to_string(),
            section_ids: vec![],
            max_tokens: 800,
        })
        .unwrap();
    assert!(
        loaded_by_handle["content"]
            .as_str()
            .unwrap()
            .contains("Workflow")
    );

    let forged = engine.load(&LoadRequest {
        skill_id_or_handle: "search:forged:1:github-code-review".to_string(),
        view: "preview".to_string(),
        section_ids: vec![],
        max_tokens: 800,
    });
    assert!(forged.is_err());
}
