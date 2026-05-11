use crate::models::{SectionRecord, SkillRecord};
use crate::text::{heading_re, sha256_hex, slugify};
use anyhow::{Context, Result};
use serde_yaml::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

const RISK_PATTERNS: &[(&str, &[&str])] = &[
    (
        "NETWORK",
        &[
            "http", "https", "api", "github", "download", "web", "network", "curl", "wget", "gh ",
        ],
    ),
    (
        "GIT",
        &[
            "git",
            "github",
            "pull request",
            "pr",
            "branch",
            "commit",
            "diff",
        ],
    ),
    (
        "PROCESS",
        &[
            "run ",
            "execute",
            "command",
            "terminal",
            "pytest",
            "npm",
            "uv ",
            "python -m",
            "build",
            "test",
        ],
    ),
    (
        "LOCAL_FS_READ",
        &[
            "read",
            "inspect",
            "file",
            "diff",
            "repository",
            "repo",
            "open",
        ],
    ),
    (
        "LOCAL_FS_WRITE",
        &[
            "write", "edit", "patch", "create", "update", "commit", "save", "delete", "remove",
        ],
    ),
    (
        "DESTRUCTIVE_POSSIBLE",
        &[
            "delete",
            "remove",
            "drop",
            "overwrite",
            "push",
            "merge",
            "deploy",
        ],
    ),
];

pub fn discover_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if entry.file_type().is_file()
                && path.file_name().and_then(|s| s.to_str()) == Some("SKILL.md")
            {
                files.extend(path.canonicalize());
            }
        }
    }
    files.sort();
    files
}

pub fn parse_skill(path: &Path, root: &Path, roots: &[PathBuf]) -> Result<SkillRecord> {
    let content = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let (front, body) = load_frontmatter(&content);
    let sections = extract_sections(&body);
    let name = yaml_string(&front, "name").unwrap_or_else(|| {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("skill")
            .to_string()
    });
    let description = yaml_string(&front, "description").unwrap_or_default();
    let mut tags = yaml_string_list(&front, "tags");
    if tags.is_empty() {
        tags = yaml_metadata_hermes_tags(&front);
    }
    let use_when = extract_by_aliases(&sections, &["when-to-use", "use-when", "overview"]);
    let do_not = extract_by_aliases(
        &sections,
        &["do-not-use-when", "when-not-to-use", "dont-use-when"],
    );
    let required = extract_by_aliases(
        &sections,
        &[
            "required-inputs",
            "requirements",
            "setup",
            "required-environment-variables",
        ],
    );
    let workflow = workflow_summary(&sections, &description);
    let card = make_card(&name, &description, &tags, &use_when, &required);
    let metadata = fs::metadata(path)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok());
    let mtime_ns = modified.map(|d| d.as_nanos()).unwrap_or(0);
    Ok(SkillRecord {
        skill_id: slugify(&name),
        name,
        description,
        category: category_for(root, path),
        tags,
        skill_card: card,
        workflow_summary: workflow,
        use_when,
        do_not_use_when: do_not,
        required_inputs: required,
        risk_flags: risk_flags(&content),
        sections,
        source_path: path.display().to_string(),
        source_sha256: sha256_hex(&content),
        mtime_ns,
        size: metadata.len(),
        trust_level: trust_level(path, roots),
        content,
    })
}

fn load_frontmatter(text: &str) -> (BTreeMap<String, Value>, String) {
    if !text.starts_with("---\n") {
        return (BTreeMap::new(), text.to_string());
    }
    let rest = &text[4..];
    if let Some(pos) = rest.find("---\n") {
        let front = &rest[..pos];
        let body = &rest[pos + 4..];
        let data = serde_yaml::from_str::<BTreeMap<String, Value>>(front).unwrap_or_default();
        (data, body.to_string())
    } else {
        (BTreeMap::new(), text.to_string())
    }
}

fn yaml_string(map: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn yaml_string_list(map: &BTreeMap<String, Value>, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(Value::String(s)) => vec![s.trim().to_string()],
        Some(Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn yaml_metadata_hermes_tags(map: &BTreeMap<String, Value>) -> Vec<String> {
    let Some(Value::Mapping(meta)) = map.get("metadata") else {
        return Vec::new();
    };
    let Some(Value::Mapping(hermes)) = meta.get(Value::String("hermes".to_string())) else {
        return Vec::new();
    };
    match hermes.get(Value::String("tags".to_string())) {
        Some(Value::String(s)) => vec![s.trim().to_string()],
        Some(Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn extract_sections(body: &str) -> BTreeMap<String, SectionRecord> {
    let lines: Vec<&str> = body.lines().collect();
    let mut headings: Vec<(usize, usize, String)> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if let Some(cap) = heading_re().captures(line) {
            headings.push((idx, cap[1].len(), cap[2].trim().to_string()));
        }
    }
    let mut sections = BTreeMap::new();
    for (pos, (start, level, title)) in headings.iter().enumerate() {
        let mut end = lines.len();
        for (next_start, next_level, _) in headings.iter().skip(pos + 1) {
            if next_level <= level {
                end = *next_start;
                break;
            }
        }
        let base = slugify(title);
        let id = if sections.contains_key(&base) {
            format!("{}-{}", base, pos + 1)
        } else {
            base
        };
        let content = lines[*start..end].join("\n").trim().to_string();
        sections.insert(
            id.clone(),
            SectionRecord {
                section_id: id,
                title: title.clone(),
                level: *level,
                content,
                start_line: start + 1,
                end_line: end,
            },
        );
    }
    sections
}

fn section_text(sections: &BTreeMap<String, SectionRecord>, id: &str) -> Option<String> {
    sections
        .get(id)
        .map(|s| {
            let mut lines: Vec<&str> = s.content.lines().collect();
            if lines
                .first()
                .is_some_and(|line| heading_re().is_match(line))
            {
                lines.remove(0);
            }
            lines.join("\n").trim().to_string()
        })
        .filter(|s| !s.is_empty())
}

fn extract_by_aliases(sections: &BTreeMap<String, SectionRecord>, aliases: &[&str]) -> String {
    aliases
        .iter()
        .find_map(|id| section_text(sections, id))
        .unwrap_or_default()
}

fn workflow_summary(sections: &BTreeMap<String, SectionRecord>, description: &str) -> String {
    let text = extract_by_aliases(
        sections,
        &[
            "workflow",
            "process",
            "steps",
            "procedure",
            "core-principle",
        ],
    );
    let source = if text.is_empty() { description } else { &text };
    let mut lines = Vec::new();
    for line in source.lines() {
        let stripped = line.trim();
        if stripped.is_empty() || heading_re().is_match(stripped) {
            continue;
        }
        lines.push(stripped.to_string());
        if lines.join(" ").len() > 1000 {
            break;
        }
    }
    lines.join("\n")
}

fn make_card(
    name: &str,
    description: &str,
    tags: &[String],
    use_when: &str,
    required: &str,
) -> String {
    let mut parts = vec![format!("{}: {}", name, description).trim().to_string()];
    if !tags.is_empty() {
        parts.push(format!(
            "Tags: {}",
            tags.iter().take(8).cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    if !use_when.is_empty() {
        parts.push(format!("Use when: {}", compact(use_when, 500)));
    }
    if !required.is_empty() {
        parts.push(format!("Required: {}", compact(required, 300)));
    }
    parts
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn compact(text: &str, max_chars: usize) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    one_line.chars().take(max_chars).collect()
}

fn category_for(root: &Path, path: &Path) -> String {
    let parent = path.parent().unwrap_or(path);
    parent
        .strip_prefix(root)
        .ok()
        .and_then(|rel| {
            if rel.components().count() > 1 {
                rel.components()
                    .next()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "uncategorized".to_string())
}

fn risk_flags(text: &str) -> Vec<String> {
    let low = text.to_lowercase();
    let mut flags = Vec::new();
    for (flag, patterns) in RISK_PATTERNS {
        if patterns.iter().any(|p| low.contains(p)) {
            flags.push((*flag).to_string());
        }
    }
    flags.sort();
    flags.dedup();
    flags
}

fn trust_level(path: &Path, roots: &[PathBuf]) -> String {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"));
    let hermes = home.join(".hermes").join("skills");
    if path.starts_with(&hermes) {
        return "user-hermes".to_string();
    }
    if roots.iter().any(|root| path.starts_with(root)) {
        return "local-root".to_string();
    }
    "unknown".to_string()
}
