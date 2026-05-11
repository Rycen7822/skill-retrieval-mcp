use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const AVAILABLE_VIEWS: [&str; 6] = ["card", "preview", "runtime", "risk", "sections", "full"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    #[serde(default)]
    pub raw_user_request: String,
    #[serde(default)]
    pub description_query: String,
    #[serde(default)]
    pub workflow_query: String,
    #[serde(default)]
    pub must_have: Vec<String>,
    #[serde(default)]
    pub nice_to_have: Vec<String>,
    #[serde(default)]
    pub must_not: Vec<String>,
    #[serde(default)]
    pub environment: Vec<String>,
    #[serde(default = "default_k")]
    pub k: usize,
    #[serde(default = "default_search_tokens")]
    pub max_tokens: usize,
    #[serde(default = "default_mmr_lambda")]
    pub mmr_lambda: f64,
    #[serde(default)]
    pub trusted_only: bool,
    #[serde(default)]
    pub category: Option<String>,
}

fn default_k() -> usize {
    3
}
fn default_search_tokens() -> usize {
    1200
}
fn default_mmr_lambda() -> f64 {
    0.70
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            raw_user_request: String::new(),
            description_query: String::new(),
            workflow_query: String::new(),
            must_have: Vec::new(),
            nice_to_have: Vec::new(),
            must_not: Vec::new(),
            environment: Vec::new(),
            k: default_k(),
            max_tokens: default_search_tokens(),
            mmr_lambda: default_mmr_lambda(),
            trusted_only: false,
            category: None,
        }
    }
}

impl SearchRequest {
    pub fn validate(&mut self) -> anyhow::Result<()> {
        self.raw_user_request = self.raw_user_request.trim().to_string();
        self.description_query = self.description_query.trim().to_string();
        self.workflow_query = self.workflow_query.trim().to_string();
        self.must_have = trim_list(&self.must_have);
        self.nice_to_have = trim_list(&self.nice_to_have);
        self.must_not = trim_list(&self.must_not);
        self.environment = trim_list(&self.environment);
        if self.raw_user_request.chars().count() > 8000 {
            anyhow::bail!("raw_user_request exceeds 8000 characters");
        }
        if self.description_query.chars().count() > 4000 {
            anyhow::bail!("description_query exceeds 4000 characters");
        }
        if self.workflow_query.chars().count() > 4000 {
            anyhow::bail!("workflow_query exceeds 4000 characters");
        }
        for (name, values) in [
            ("must_have", &self.must_have),
            ("nice_to_have", &self.nice_to_have),
            ("must_not", &self.must_not),
            ("environment", &self.environment),
        ] {
            if values.len() > 20 {
                anyhow::bail!("{} accepts at most 20 entries", name);
            }
        }
        if !(1..=10).contains(&self.k) {
            anyhow::bail!("k must be between 1 and 10");
        }
        if !(200..=4000).contains(&self.max_tokens) {
            anyhow::bail!("max_tokens must be between 200 and 4000");
        }
        if !(0.0..=1.0).contains(&self.mmr_lambda) {
            anyhow::bail!("mmr_lambda must be between 0 and 1");
        }
        if self.raw_user_request.is_empty()
            && self.description_query.is_empty()
            && self.workflow_query.is_empty()
            && self.must_have.is_empty()
        {
            anyhow::bail!("At least one query field or must_have cue is required");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadRequest {
    pub skill_id_or_handle: String,
    #[serde(default = "default_view")]
    pub view: String,
    #[serde(default)]
    pub section_ids: Vec<String>,
    #[serde(default = "default_load_tokens")]
    pub max_tokens: usize,
}

fn default_view() -> String {
    "preview".to_string()
}
fn default_load_tokens() -> usize {
    1200
}

impl LoadRequest {
    pub fn validate(&mut self) -> anyhow::Result<()> {
        self.skill_id_or_handle = self.skill_id_or_handle.trim().to_string();
        self.view = self.view.trim().to_string();
        self.section_ids = trim_list(&self.section_ids);
        if self.skill_id_or_handle.is_empty() {
            anyhow::bail!("skill_id_or_handle is required");
        }
        if self.skill_id_or_handle.chars().count() > 300 {
            anyhow::bail!("skill_id_or_handle exceeds 300 characters");
        }
        if !AVAILABLE_VIEWS.contains(&self.view.as_str()) {
            anyhow::bail!("unsupported view: {}", self.view);
        }
        if self.section_ids.len() > 20 {
            anyhow::bail!("section_ids accepts at most 20 entries");
        }
        if !(80..=8000).contains(&self.max_tokens) {
            anyhow::bail!("max_tokens must be between 80 and 8000");
        }
        Ok(())
    }
}

fn trim_list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionRecord {
    pub section_id: String,
    pub title: String,
    pub level: usize,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRecord {
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub skill_card: String,
    pub workflow_summary: String,
    pub use_when: String,
    pub do_not_use_when: String,
    pub required_inputs: String,
    pub risk_flags: Vec<String>,
    pub sections: BTreeMap<String, SectionRecord>,
    pub source_path: String,
    pub source_sha256: String,
    pub mtime_ns: u128,
    pub size: u64,
    pub trust_level: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFingerprint {
    pub path: String,
    pub mtime_ns: u128,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePayload {
    pub version: u32,
    pub root_paths: Vec<String>,
    pub files: Vec<FileFingerprint>,
    pub records: Vec<SkillRecord>,
}

impl SkillRecord {
    pub fn positive_text(&self) -> String {
        [
            self.name.as_str(),
            self.description.as_str(),
            &self.tags.join(" "),
            self.category.as_str(),
            self.skill_card.as_str(),
            self.workflow_summary.as_str(),
            self.use_when.as_str(),
            self.required_inputs.as_str(),
        ]
        .join("\n")
    }
}
