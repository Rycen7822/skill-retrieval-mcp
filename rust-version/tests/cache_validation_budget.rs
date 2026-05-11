use std::fs;

use skill_retrieval_mcp_rust::{LoadRequest, SearchRequest, SkillRetrievalEngine};
use tempfile::tempdir;

const CODE_REVIEW_SKILL: &str = r#"---
name: github-code-review
description: Review GitHub pull requests or git diffs for correctness, security, and maintainability findings.
tags: [github, code-review, security, git]
---

# GitHub Code Review

## When to Use
Use when the user asks to review an existing pull request, patch, or git diff and produce prioritized findings.

## Do Not Use When
Do not use when the user asks you to implement a feature, push changes, create a pull request, or merge code.

## Required Inputs
A git repository and a diff or pull request reference.

## Workflow
1. Inspect repository state and identify the base diff.
2. Read relevant files around changed lines.
3. Identify correctness, security, maintainability, and test coverage issues.
4. Run targeted tests or static checks when available.
5. Return prioritized review findings with file and line evidence.

## Verification
Confirm every finding is backed by code evidence and avoid speculative issues.

## Pitfalls
Do not rewrite the user code during review unless explicitly asked.
"#;

fn write_skill(root: &std::path::Path, rel: &str, text: &str) -> std::path::PathBuf {
    let dir = root.join(rel);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("SKILL.md");
    fs::write(&path, format!("{}\n", text.trim())).unwrap();
    path
}

#[test]
fn cache_is_reused_and_invalidated_when_skill_changes() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("skills");
    let path = write_skill(&root, "github/github-code-review", CODE_REVIEW_SKILL);
    let cache = tmp.path().join("cache.json");

    let first = SkillRetrievalEngine::new_with_paths(vec![root.clone()], cache.clone()).unwrap();
    assert!(cache.exists());
    assert_eq!(first.cache_status(), "rebuilt");

    let second = SkillRetrievalEngine::new_with_paths(vec![root.clone()], cache.clone()).unwrap();
    assert_eq!(second.cache_status(), "loaded");
    assert_eq!(second.records().len(), first.records().len());

    let changed = CODE_REVIEW_SKILL.replace("security", "cryptographic security");
    fs::write(path, changed).unwrap();

    let third = SkillRetrievalEngine::new_with_paths(vec![root], cache).unwrap();
    assert_eq!(third.cache_status(), "rebuilt");
    let result = third
        .search(&SearchRequest {
            raw_user_request: "review crypto security in a PR".to_string(),
            description_query: "cryptographic security code review".to_string(),
            workflow_query: "inspect diff".to_string(),
            k: 1,
            ..SearchRequest::default()
        })
        .unwrap();
    assert_eq!(result["results"][0]["skill_id"], "github-code-review");
}

#[test]
fn request_validation_rejects_python_boundary_inputs_instead_of_clamping() {
    let mut empty = SearchRequest::default();
    assert!(empty.validate().is_err());

    let mut k_zero = SearchRequest {
        raw_user_request: "x".to_string(),
        k: 0,
        ..SearchRequest::default()
    };
    assert!(k_zero.validate().is_err());

    let mut too_many = SearchRequest {
        raw_user_request: "x".to_string(),
        k: 11,
        ..SearchRequest::default()
    };
    assert!(too_many.validate().is_err());

    let mut too_low_budget = SearchRequest {
        raw_user_request: "x".to_string(),
        max_tokens: 199,
        ..SearchRequest::default()
    };
    assert!(too_low_budget.validate().is_err());

    let mut raw_too_long = SearchRequest {
        raw_user_request: "x".repeat(8001),
        ..SearchRequest::default()
    };
    assert!(raw_too_long.validate().is_err());

    let mut bad_view = LoadRequest {
        skill_id_or_handle: "x".to_string(),
        view: "not-a-view".to_string(),
        section_ids: vec![],
        max_tokens: 1200,
    };
    assert!(bad_view.validate().is_err());

    let mut too_many_sections = LoadRequest {
        skill_id_or_handle: "x".to_string(),
        view: "sections".to_string(),
        section_ids: (0..21).map(|i| i.to_string()).collect(),
        max_tokens: 1200,
    };
    assert!(too_many_sections.validate().is_err());
}

#[test]
fn search_budget_200_is_hard_limit_and_keeps_full_source_hash() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("skills");
    for idx in 0..12 {
        let body = CODE_REVIEW_SKILL
            .replace("github-code-review", &format!("long-skill-{idx}"))
            .replace(
                "Review GitHub pull requests",
                &format!(
                    "Review GitHub pull requests {}",
                    "long contextual detail ".repeat(120)
                ),
            );
        write_skill(&root, &format!("long/long-skill-{idx}"), &body);
    }
    let engine =
        SkillRetrievalEngine::new_with_paths(vec![root], tmp.path().join("cache.json")).unwrap();
    let result = engine
        .search(&SearchRequest {
            raw_user_request: "review GitHub pull request".to_string(),
            description_query: "GitHub pull request review".to_string(),
            workflow_query: "inspect diff findings".to_string(),
            k: 10,
            max_tokens: 200,
            ..SearchRequest::default()
        })
        .unwrap();
    let compact = serde_json::to_string(&result).unwrap();
    assert_eq!(result["truncated"], true);
    assert!(
        result["tokens_estimate"].as_u64().unwrap() <= 200,
        "{compact}"
    );
    assert!(
        skill_retrieval_mcp_rust::text::estimate_tokens(&compact) <= 200,
        "{compact}"
    );
    assert!(!result["results"].as_array().unwrap().is_empty());
    assert_eq!(
        result["results"][0]["source_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
}
