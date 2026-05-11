use crate::models::{
    AVAILABLE_VIEWS, CachePayload, FileFingerprint, LoadRequest, SearchRequest, SkillRecord,
};
use crate::parser::{discover_files, parse_skill};
use crate::text::{
    estimate_tokens, important_phrases, jaccard_sets, token_counts, token_set, tokenize,
    trim_to_token_budget,
};
use anyhow::{Result, anyhow};
use chrono::{TimeZone, Utc};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const ACTION_TOKENS: &[&str] = &[
    "review",
    "inspect",
    "implement",
    "fix",
    "debug",
    "create",
    "open",
    "push",
    "merge",
    "commit",
    "delete",
    "remove",
    "deploy",
    "write",
    "edit",
    "modify",
    "generate",
    "download",
    "install",
    "run",
    "execute",
];
const SAFE_ID_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_.:@+-";
const CACHE_VERSION: u32 = 4;

#[derive(Debug, Clone)]
struct PreparedText {
    lower: String,
    counts: HashMap<String, usize>,
    tokens: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct PreparedRecord {
    description: PreparedText,
    workflow: PreparedText,
    raw: PreparedText,
    metadata: PreparedText,
    positive: PreparedText,
    aliases: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Clone)]
struct ScoredRecord {
    index: usize,
    score: f64,
    why_match: Vec<String>,
    why_maybe_not: Vec<String>,
    missing_requirements: Vec<String>,
    matched_fields: BTreeSet<String>,
    positive_conflicts: Vec<String>,
}

pub struct SkillRetrievalEngine {
    roots: Vec<PathBuf>,
    cache_path: PathBuf,
    cache_status: String,
    records: Vec<SkillRecord>,
    by_id: HashMap<String, usize>,
    prepared: Vec<PreparedRecord>,
    handles: Mutex<HashMap<String, String>>,
}

impl SkillRetrievalEngine {
    pub fn new() -> Result<Self> {
        Self::new_with_paths(default_roots(), default_cache_path())
    }

    pub fn new_with_paths(roots: Vec<PathBuf>, cache_path: PathBuf) -> Result<Self> {
        let roots: Vec<PathBuf> = roots
            .into_iter()
            .map(expand_tilde)
            .map(|p| p.canonicalize().unwrap_or(p))
            .collect();
        let cache_path = expand_tilde(cache_path);
        let files = discover_files(&roots);
        let fingerprints = fingerprints(&files);
        let root_paths = roots
            .iter()
            .map(|root| root.display().to_string())
            .collect::<Vec<_>>();

        let (records, cache_status) = if let Some(records) =
            load_cache(&cache_path, &root_paths, &fingerprints, CACHE_VERSION)
        {
            (records, "loaded".to_string())
        } else {
            let mut records = parse_records(&roots, &files);
            dedupe_records(&mut records);
            write_cache(&cache_path, &root_paths, &fingerprints, &records);
            (records, "rebuilt".to_string())
        };

        let by_id = records
            .iter()
            .enumerate()
            .map(|(idx, r)| (r.skill_id.clone(), idx))
            .collect();
        let prepared = records.iter().map(prepare_record).collect();
        Ok(Self {
            roots,
            cache_path,
            cache_status,
            records,
            by_id,
            prepared,
            handles: Mutex::new(HashMap::new()),
        })
    }

    pub fn records(&self) -> &[SkillRecord] {
        &self.records
    }
    pub fn cache_status(&self) -> &str {
        &self.cache_status
    }
    pub fn cache_path(&self) -> &Path {
        &self.cache_path
    }
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub fn search(&self, request: &SearchRequest) -> Result<Value> {
        let mut request = request.clone();
        request.validate()?;
        let query = PreparedQuery::new(&request);
        let mut scored = Vec::new();
        for (idx, record) in self.records.iter().enumerate() {
            if request.trusted_only
                && !matches!(record.trust_level.as_str(), "user-hermes" | "local-root")
            {
                continue;
            }
            if request
                .category
                .as_ref()
                .is_some_and(|category| &record.category != category)
            {
                continue;
            }
            scored.push(self.score_record(idx, record, &request, &query));
        }
        scored.sort_by(|a, b| {
            b.score.total_cmp(&a.score).then_with(|| {
                self.records[a.index]
                    .skill_id
                    .cmp(&self.records[b.index].skill_id)
            })
        });
        let mut selected = self.mmr_select(scored, request.k, request.mmr_lambda);
        selected.truncate(request.k);
        let query_id = query_id(&request);
        let top_gap = if selected.len() > 1 {
            selected[0].score - selected[1].score
        } else {
            selected.first().map(|s| s.score).unwrap_or(0.0)
        };
        let ambiguous = selected.len() > 1
            && (top_gap < 0.055
                || jaccard_sets(
                    &self.prepared[selected[0].index].positive.tokens,
                    &self.prepared[selected[1].index].positive.tokens,
                ) > 0.92);
        let overall = overall_confidence(
            selected.first().map(|s| s.score).unwrap_or(0.0),
            top_gap,
            ambiguous,
        );
        let mut results = Vec::new();
        for (rank0, item) in selected.iter().enumerate() {
            let rank = rank0 + 1;
            let record = &self.records[item.index];
            let handle = format!("search:{}:{}:{}", query_id, rank, record.skill_id);
            self.handles
                .lock()
                .unwrap()
                .insert(handle.clone(), record.skill_id.clone());
            let conf =
                candidate_confidence(item.score, if rank == 1 { top_gap } else { 0.0 }, ambiguous);
            let decision = load_decision(item, &conf, ambiguous);
            results.push(json!({
                "handle": handle,
                "skill_id": record.skill_id,
                "score": round4(item.score),
                "confidence": conf,
                "load_decision": decision,
                "recommended_view": if decision == "safe_to_load" { "runtime" } else { "preview" },
                "why_match": item.why_match.iter().take(5).cloned().collect::<Vec<_>>(),
                "why_maybe_not": item.why_maybe_not.iter().take(5).cloned().collect::<Vec<_>>(),
                "missing_requirements": item.missing_requirements,
                "matched_fields": item.matched_fields.iter().cloned().collect::<Vec<_>>(),
                "risk_flags": record.risk_flags,
                "trust_level": record.trust_level,
                "source_path": record.source_path,
                "source_sha256": record.source_sha256,
                "card": record.skill_card.chars().take(900).collect::<String>(),
            }));
        }
        let response = json!({
            "query_id": query_id,
            "confidence": overall,
            "results": results,
            "ambiguity": {"is_ambiguous": ambiguous, "reason": if ambiguous {"top candidates are close or near-duplicates; preview before runtime load"} else {"top candidate is clearly separated"}, "top_gap": round4(top_gap)},
            "cache_status": self.cache_status,
            "total_indexed": self.records.len(),
        });
        Ok(fit_search_budget(response, request.max_tokens))
    }

    pub fn load(&self, request: &LoadRequest) -> Result<Value> {
        let mut request = request.clone();
        request.validate()?;
        let skill_id = self.resolve_skill_id(&request.skill_id_or_handle)?;
        let idx = *self
            .by_id
            .get(&skill_id)
            .ok_or_else(|| anyhow!("Unknown skill handle/id: {}", skill_id))?;
        let record = &self.records[idx];
        let content = self.render_view(record, &request)?;
        let (content, truncated, tokens) = trim_to_token_budget(&content, request.max_tokens);
        let updated_at = Utc
            .timestamp_nanos(record.mtime_ns as i64)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        Ok(json!({
            "skill_id": record.skill_id,
            "view": request.view,
            "content": content,
            "tokens_estimate": tokens,
            "truncated": truncated,
            "available_views": AVAILABLE_VIEWS,
            "source_path": record.source_path,
            "source_sha256": record.source_sha256,
            "updated_at": updated_at,
            "trust_level": record.trust_level,
            "risk_flags": record.risk_flags,
        }))
    }

    fn resolve_skill_id(&self, raw: &str) -> Result<String> {
        if let Some(skill_id) = self.handles.lock().unwrap().get(raw).cloned() {
            return Ok(skill_id);
        }
        if raw.starts_with("search:") {
            return Err(anyhow!(
                "Unknown or expired skill search handle: {}. Use a handle returned by this skill_search session or pass a canonical skill_id.",
                raw
            ));
        }
        if !raw.chars().all(|c| SAFE_ID_CHARS.contains(c)) {
            return Err(anyhow!(
                "Unknown skill handle/id: {}. Use a skill_id or handle returned by skill_search.",
                raw
            ));
        }
        if self.by_id.contains_key(raw) {
            Ok(raw.to_string())
        } else {
            Err(anyhow!(
                "Unknown skill handle/id: {}. Use skill_search first or pass a canonical skill_id.",
                raw
            ))
        }
    }

    fn score_record(
        &self,
        idx: usize,
        record: &SkillRecord,
        request: &SearchRequest,
        query: &PreparedQuery,
    ) -> ScoredRecord {
        let prep = &self.prepared[idx];
        let (desc_score, desc_matches) = score_prepared(&query.description, &prep.description);
        let (workflow_score, workflow_matches) = score_prepared(&query.workflow, &prep.workflow);
        let (raw_score, raw_matches) = score_prepared(&query.raw, &prep.raw);
        let (metadata_score, meta_matches) = score_prepared(&query.metadata, &prep.metadata);
        let trust_score = if matches!(record.trust_level.as_str(), "user-hermes" | "local-root") {
            1.0
        } else {
            0.5
        };
        let mut score = 0.40 * desc_score
            + 0.30 * workflow_score
            + 0.15 * raw_score
            + 0.10 * metadata_score
            + 0.05 * trust_score;
        let (identity_bonus, identity_matches) = identity_match(prep, query);
        score += identity_bonus;
        let mut why_match = Vec::new();
        let mut matched_fields = BTreeSet::new();
        if !identity_matches.is_empty() {
            matched_fields.insert("identity".to_string());
            why_match.push(format!("identity matched: {}", identity_matches.join(", ")));
        }
        if !desc_matches.is_empty() {
            matched_fields.insert("description".to_string());
            why_match.push(format!(
                "description/card matched: {}",
                desc_matches.join(", ")
            ));
        }
        if !workflow_matches.is_empty() {
            matched_fields.insert("workflow_summary".to_string());
            why_match.push(format!("workflow matched: {}", workflow_matches.join(", ")));
        }
        if !raw_matches.is_empty() {
            matched_fields.insert("raw_user_request".to_string());
            why_match.push(format!(
                "raw request sanity matched: {}",
                raw_matches.join(", ")
            ));
        }
        if !meta_matches.is_empty() {
            matched_fields.insert("metadata".to_string());
            why_match.push(format!("metadata matched: {}", meta_matches.join(", ")));
        }
        let missing: Vec<String> = query
            .must_have
            .iter()
            .filter(|cue| !cue_present(cue, &prep.positive))
            .cloned()
            .collect();
        if !request.must_have.is_empty() {
            let matched_have: Vec<String> = query
                .must_have
                .iter()
                .filter(|cue| !missing.contains(cue))
                .cloned()
                .collect();
            if !matched_have.is_empty() {
                score += f64::min(0.16, 0.04 * matched_have.len() as f64);
                matched_fields.insert("must_have".to_string());
                why_match.push(format!("must_have satisfied: {}", matched_have.join(", ")));
            }
            if !missing.is_empty() {
                score -= f64::min(0.22, 0.055 * missing.len() as f64);
            }
        }
        let positive_conflicts: Vec<String> = query
            .must_not
            .iter()
            .filter(|cue| cue_present(cue, &prep.positive))
            .cloned()
            .collect();
        let mut why_maybe_not = Vec::new();
        if !positive_conflicts.is_empty() {
            score -= f64::min(0.35, 0.09 * positive_conflicts.len() as f64);
            why_maybe_not.push(format!(
                "must_not_positive_conflict: {}",
                positive_conflicts.join(", ")
            ));
        }
        if !query.raw.tokens.is_empty() && !record.do_not_use_when.is_empty() {
            let (neg_score, neg_matches) = negative_conflict(&query.raw, &record.do_not_use_when);
            if neg_score >= 0.20 {
                score -= f64::min(0.45, 0.18 + neg_score * 0.30);
                why_maybe_not.push(format!(
                    "do_not_use_when matched raw request: {}",
                    neg_matches.join(", ")
                ));
            }
        }
        if why_match.is_empty() {
            why_match.push("weak lexical fallback match".to_string());
        }
        ScoredRecord {
            index: idx,
            score: score.clamp(0.0, 1.0),
            why_match,
            why_maybe_not,
            missing_requirements: missing,
            matched_fields,
            positive_conflicts,
        }
    }

    fn mmr_select(&self, scored: Vec<ScoredRecord>, k: usize, lambda: f64) -> Vec<ScoredRecord> {
        if scored.is_empty() || k == 0 {
            return Vec::new();
        }
        let mut selected = vec![scored[0].clone()];
        let mut remaining: Vec<ScoredRecord> = scored.into_iter().skip(1).collect();
        while !remaining.is_empty() && selected.len() < k {
            let mut best_idx = 0;
            let mut best_value = f64::NEG_INFINITY;
            for (idx, item) in remaining.iter().enumerate() {
                let item_tokens = &self.prepared[item.index].positive.tokens;
                let similarity = selected
                    .iter()
                    .map(|chosen| {
                        jaccard_sets(item_tokens, &self.prepared[chosen.index].positive.tokens)
                    })
                    .fold(0.0, f64::max);
                let value = lambda * item.score - (1.0 - lambda) * similarity;
                if value > best_value {
                    best_value = value;
                    best_idx = idx;
                }
            }
            selected.push(remaining.remove(best_idx));
        }
        selected
    }

    fn render_view(&self, record: &SkillRecord, request: &LoadRequest) -> Result<String> {
        let applicability = applicability(record);
        match request.view.as_str() {
            "card" => Ok(record.skill_card.clone()),
            "preview" => Ok([
                applicability,
                format!("Workflow summary:\n{}", empty_doc(&record.workflow_summary)),
                format!(
                    "Risk flags: {}",
                    if record.risk_flags.is_empty() {
                        "none detected".to_string()
                    } else {
                        record.risk_flags.join(", ")
                    }
                ),
                format!("Source: {}", record.source_path),
            ]
            .join("\n\n")),
            "runtime" => {
                let mut parts = Vec::new();
                for id in [
                    "required-inputs",
                    "workflow",
                    "process",
                    "steps",
                    "verification",
                    "pitfalls",
                    "common-pitfalls",
                ] {
                    if let Some(sec) = record.sections.get(id) {
                        parts.push(sec.content.clone());
                    }
                }
                if parts.is_empty() {
                    parts.push(if record.workflow_summary.is_empty() {
                        record.content.clone()
                    } else {
                        record.workflow_summary.clone()
                    });
                }
                Ok(format!("{}\n\n{}", applicability, parts.join("\n\n")))
            }
            "risk" => {
                let mut parts = vec![
                    applicability,
                    format!(
                        "Risk flags: {}",
                        if record.risk_flags.is_empty() {
                            "none detected".to_string()
                        } else {
                            record.risk_flags.join(", ")
                        }
                    ),
                    format!("Trust level: {}", record.trust_level),
                    format!("Source SHA256: {}", record.source_sha256),
                ];
                for id in [
                    "risk",
                    "security",
                    "pitfalls",
                    "do-not-use-when",
                    "required-inputs",
                ] {
                    if let Some(sec) = record.sections.get(id) {
                        parts.push(sec.content.clone());
                    }
                }
                Ok(parts.join("\n\n"))
            }
            "sections" => {
                if request.section_ids.is_empty() {
                    return Err(anyhow!(
                        "section_ids required for sections view. Available sections: {}",
                        record
                            .sections
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                let mut parts = vec![applicability];
                for id in &request.section_ids {
                    let sec = record.sections.get(id).ok_or_else(|| {
                        anyhow!(
                            "Unknown section_id {:?}. Available sections: {}",
                            id,
                            record
                                .sections
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })?;
                    parts.push(sec.content.clone());
                }
                Ok(parts.join("\n\n"))
            }
            "full" => Ok(format!("{}\n\n{}", applicability, record.content)),
            _ => Err(anyhow!("Unsupported view: {}", request.view)),
        }
    }
}

fn default_roots() -> Vec<PathBuf> {
    if let Some(raw) = std::env::var("SRM_SKILL_ROOTS")
        .ok()
        .filter(|raw| !raw.trim().is_empty())
    {
        return raw
            .split(':')
            .filter(|p| !p.trim().is_empty())
            .map(PathBuf::from)
            .collect();
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    vec![home.join(".hermes").join("skills")]
}

fn default_cache_path() -> PathBuf {
    if let Some(raw) = std::env::var("SRM_CACHE_PATH")
        .ok()
        .filter(|raw| !raw.trim().is_empty())
    {
        return PathBuf::from(raw);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".cache")
        .join("skill-retrieval-mcp-rust")
        .join("index.json")
}

fn expand_tilde(path: PathBuf) -> PathBuf {
    let raw = path.to_string_lossy();
    match (
        raw == "~" || raw.starts_with("~/"),
        std::env::var_os("HOME"),
    ) {
        (true, Some(home)) => {
            let suffix = raw.strip_prefix("~/").unwrap_or("");
            PathBuf::from(home).join(suffix)
        }
        _ => path,
    }
}

fn fingerprints(files: &[PathBuf]) -> Vec<FileFingerprint> {
    files
        .iter()
        .filter_map(|path| {
            let metadata = std::fs::metadata(path).ok()?;
            let mtime_ns = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            Some(FileFingerprint {
                path: path.display().to_string(),
                mtime_ns,
                size: metadata.len(),
            })
        })
        .collect()
}

fn parse_records(roots: &[PathBuf], files: &[PathBuf]) -> Vec<SkillRecord> {
    let mut records = Vec::new();
    for path in files {
        let root = roots
            .iter()
            .find(|r| path.starts_with(r))
            .cloned()
            .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")).to_path_buf());
        if let Ok(record) = parse_skill(path, &root, roots) {
            records.push(record);
        }
    }
    records
}

fn load_cache(
    cache_path: &Path,
    root_paths: &[String],
    fingerprints: &[FileFingerprint],
    version: u32,
) -> Option<Vec<SkillRecord>> {
    let raw = std::fs::read_to_string(cache_path).ok()?;
    let payload = serde_json::from_str::<CachePayload>(&raw).ok()?;
    if payload.version == version
        && payload.root_paths == root_paths
        && payload.files == fingerprints
    {
        Some(payload.records)
    } else {
        None
    }
}

fn write_cache(
    cache_path: &Path,
    root_paths: &[String],
    fingerprints: &[FileFingerprint],
    records: &[SkillRecord],
) {
    let payload = CachePayload {
        version: CACHE_VERSION,
        root_paths: root_paths.to_vec(),
        files: fingerprints.to_vec(),
        records: records.to_vec(),
    };
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string(&payload) {
        let _ = std::fs::write(cache_path, raw);
    }
}

fn prepare_text(text: &str) -> PreparedText {
    PreparedText {
        lower: text.to_lowercase(),
        counts: token_counts(text),
        tokens: token_set(text),
    }
}

fn prepare_record(record: &SkillRecord) -> PreparedRecord {
    let desc = [
        record.description.as_str(),
        record.skill_card.as_str(),
        record.use_when.as_str(),
        &record.tags.join(" "),
    ]
    .join("\n");
    let workflow = [
        record.workflow_summary.as_str(),
        &record
            .sections
            .values()
            .map(|s| s.title.clone())
            .collect::<Vec<_>>()
            .join("\n"),
    ]
    .join("\n");
    let metadata = [
        record.category.as_str(),
        &record.tags.join(" "),
        &record.risk_flags.join(" "),
        record.required_inputs.as_str(),
    ]
    .join(" ");
    let positive = record.positive_text();
    let aliases = [record.skill_id.clone(), record.name.clone()]
        .into_iter()
        .flat_map(|value| {
            let phrase = tokenize(&value).join(" ");
            if !phrase.is_empty() && phrase != value.to_lowercase() {
                vec![value, phrase]
            } else {
                vec![value]
            }
        })
        .map(|a| (a.clone(), tokenize(&a)))
        .collect();
    PreparedRecord {
        description: prepare_text(&desc),
        workflow: prepare_text(&workflow),
        raw: prepare_text(&positive),
        metadata: prepare_text(&metadata),
        positive: prepare_text(&positive),
        aliases,
    }
}

#[derive(Debug, Clone)]
struct PreparedQuery {
    raw: QueryText,
    description: QueryText,
    workflow: QueryText,
    metadata: QueryText,
    identity_text: String,
    identity_tokens: Vec<String>,
    must_have: Vec<String>,
    must_not: Vec<String>,
}

#[derive(Debug, Clone)]
struct QueryText {
    raw: String,
    tokens: Vec<String>,
    phrases: Vec<String>,
}

impl QueryText {
    fn new(text: &str) -> Self {
        Self {
            raw: text.to_string(),
            tokens: tokenize(text),
            phrases: important_phrases(text),
        }
    }
}
impl PreparedQuery {
    fn new(req: &SearchRequest) -> Self {
        let raw = QueryText::new(&req.raw_user_request);
        let description = QueryText::new(&req.description_query);
        let workflow = QueryText::new(&req.workflow_query);
        Self {
            metadata: QueryText::new(
                &[req.environment.join(" "), req.nice_to_have.join(" ")].join(" "),
            ),
            identity_text: [
                req.raw_user_request.as_str(),
                req.description_query.as_str(),
                req.workflow_query.as_str(),
            ]
            .join("\n")
            .to_lowercase(),
            identity_tokens: [
                raw.tokens.clone(),
                description.tokens.clone(),
                workflow.tokens.clone(),
            ]
            .concat(),
            raw,
            description,
            workflow,
            must_have: req.must_have.clone(),
            must_not: req.must_not.clone(),
        }
    }
}

fn score_prepared(query: &QueryText, prepared: &PreparedText) -> (f64, Vec<String>) {
    if query.tokens.is_empty() || prepared.counts.is_empty() {
        return (0.0, Vec::new());
    }
    let mut matched = Vec::new();
    let mut weighted = 0.0;
    for token in &query.tokens {
        if let Some(count) = prepared.counts.get(token) {
            matched.push(token.clone());
            weighted += f64::min(2.0, 1.0 + ((*count as f64).ln_1p() / 3.0));
        }
    }
    let mut score = weighted / query.tokens.len().max(1) as f64;
    for phrase in &query.phrases {
        if prepared.lower.contains(phrase) {
            score += 0.08;
        }
    }
    matched.sort();
    matched.dedup();
    (score.min(1.0), matched.into_iter().take(8).collect())
}

fn identity_match(prep: &PreparedRecord, query: &PreparedQuery) -> (f64, Vec<String>) {
    let mut best = 0.0;
    let mut matches = Vec::new();
    for (alias, tokens) in &prep.aliases {
        if tokens.is_empty() {
            continue;
        }
        let exact = query.identity_text.contains(&alias.to_lowercase());
        let phrase = contains_token_phrase(&query.identity_tokens, tokens);
        if !(exact || phrase) {
            continue;
        }
        let bonus = if tokens.len() > 1 {
            if exact { 0.24 } else { 0.18 }
        } else if exact {
            0.08
        } else {
            0.04
        };
        if bonus > best {
            best = bonus;
        }
        matches.push(alias.clone());
    }
    matches.truncate(4);
    (best, matches)
}

fn contains_token_phrase(tokens: &[String], phrase: &[String]) -> bool {
    !tokens.is_empty()
        && !phrase.is_empty()
        && phrase.len() <= tokens.len()
        && tokens.windows(phrase.len()).any(|w| w == phrase)
}

fn cue_present(cue: &str, prepared: &PreparedText) -> bool {
    let toks = tokenize(cue);
    if toks.is_empty() {
        return false;
    }
    toks.iter().all(|t| prepared.tokens.contains(t)) || prepared.lower.contains(&toks.join(" "))
}

fn negative_conflict(raw_query: &QueryText, negative_text: &str) -> (f64, Vec<String>) {
    let raw_tokens: BTreeSet<String> = raw_query.tokens.iter().cloned().collect();
    let raw_lower = raw_query.raw.to_lowercase();
    let action_tokens: BTreeSet<String> = ACTION_TOKENS.iter().map(|s| s.to_string()).collect();
    let mut best = 0.0;
    let mut best_matches = Vec::new();
    for sentence in negative_text.split(['\n', '.', ';']) {
        let sent: BTreeSet<String> = tokenize(sentence).into_iter().collect();
        if sent.is_empty() {
            continue;
        }
        let actions: Vec<String> = raw_tokens
            .intersection(&sent)
            .filter(|t| action_tokens.contains(*t))
            .filter(|t| {
                let token = t.as_str();
                !raw_lower.contains(&format!("do not {}", token))
                    && !raw_lower.contains(&format!("don't {}", token))
                    && !raw_lower.contains(&format!("dont {}", token))
            })
            .cloned()
            .collect();
        if actions.is_empty() {
            continue;
        }
        let overlap: Vec<String> = raw_tokens.intersection(&sent).cloned().collect();
        let non_actions: Vec<String> = overlap
            .iter()
            .filter(|t| !action_tokens.contains(*t))
            .cloned()
            .collect();
        if non_actions.is_empty()
            && !["push", "merge", "delete", "remove", "deploy"].contains(&actions[0].as_str())
        {
            continue;
        }
        let score = f64::min(
            1.0,
            0.20 * actions.len() as f64 + 0.08 * non_actions.len() as f64,
        );
        if score > best {
            best = score;
            best_matches = overlap.into_iter().take(8).collect();
        }
    }
    (best, best_matches)
}

fn overall_confidence(top: f64, gap: f64, ambiguous: bool) -> &'static str {
    if ambiguous {
        "ambiguous"
    } else if top >= 0.42 && gap >= 0.055 {
        "high"
    } else if top >= 0.22 {
        "medium"
    } else {
        "low"
    }
}
fn candidate_confidence(score: f64, gap: f64, ambiguous: bool) -> String {
    if ambiguous {
        "ambiguous".to_string()
    } else if score >= 0.42 && (gap >= 0.055 || gap == 0.0) {
        "high".to_string()
    } else if score >= 0.22 {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}
fn load_decision(item: &ScoredRecord, confidence: &str, ambiguous: bool) -> &'static str {
    if !item.why_maybe_not.is_empty() || !item.positive_conflicts.is_empty() {
        "do_not_auto_load"
    } else if !item.missing_requirements.is_empty()
        || ambiguous
        || matches!(confidence, "medium" | "low" | "ambiguous")
    {
        "preview_first"
    } else {
        "safe_to_load"
    }
}
fn applicability(record: &SkillRecord) -> String {
    format!(
        "Applicability check:\nUse this skill only if:\n{}\nDo not use if:\n{}\nRequired context:\n{}\nIf this does not match the current task, stop and call skill_search again before applying the workflow.",
        empty_doc(&record.use_when),
        empty_doc(&record.do_not_use_when),
        empty_doc(&record.required_inputs)
    )
}
fn empty_doc(text: &str) -> String {
    if text.trim().is_empty() {
        "[not documented]".to_string()
    } else {
        text.trim().to_string()
    }
}
fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}
fn query_id(request: &SearchRequest) -> String {
    let raw = serde_json::to_string(request).unwrap_or_default();
    crate::text::sha256_hex(&raw).chars().take(16).collect()
}
fn fit_search_budget(mut response: Value, max_tokens: usize) -> Value {
    let original_count = response["results"].as_array().map(|a| a.len()).unwrap_or(0);
    response["omitted_results"] = json!(0);
    response["truncated"] = json!(false);
    if fits_budget(&mut response, max_tokens) {
        return response;
    }

    response["truncated"] = json!(true);
    for card_len in [320_usize, 220, 140, 80] {
        if let Some(results) = response["results"].as_array_mut() {
            for item in results.iter_mut() {
                if let Some(card) = item
                    .get("card")
                    .and_then(Value::as_str)
                    .filter(|card| card.chars().count() > card_len)
                {
                    let short = card
                        .chars()
                        .take(card_len.saturating_sub(3))
                        .collect::<String>();
                    item["card"] = json!(format!("{}...", short.trim_end()));
                }
            }
        }
        if fits_budget(&mut response, max_tokens) {
            return response;
        }
    }

    if let Some(results) = response["results"].as_array_mut() {
        for item in results.iter_mut() {
            item["why_match"] = json!(
                item["why_match"]
                    .as_array()
                    .map(|a| a.iter().take(2).cloned().collect::<Vec<_>>())
                    .unwrap_or_default()
            );
            item["why_maybe_not"] = json!(
                item["why_maybe_not"]
                    .as_array()
                    .map(|a| a.iter().take(2).cloned().collect::<Vec<_>>())
                    .unwrap_or_default()
            );
        }
    }
    if fits_budget(&mut response, max_tokens) {
        return response;
    }

    loop {
        let len = response["results"].as_array().map(|a| a.len()).unwrap_or(0);
        if len <= 1 || fits_budget(&mut response, max_tokens) {
            break;
        }
        if let Some(results) = response["results"].as_array_mut() {
            results.pop();
            response["omitted_results"] = json!(original_count.saturating_sub(results.len()));
        }
    }
    if fits_budget(&mut response, max_tokens) {
        return response;
    }

    if let Some(results) = response["results"].as_array_mut() {
        for item in results.iter_mut() {
            if let Some(obj) = item.as_object_mut() {
                obj.remove("card");
                obj.remove("source_path");
                obj.remove("risk_flags");
                obj.remove("why_match");
                obj.remove("why_maybe_not");
                obj.remove("matched_fields");
                if obj
                    .get("missing_requirements")
                    .and_then(Value::as_array)
                    .is_none_or(|items| items.is_empty())
                {
                    obj.remove("missing_requirements");
                }
                obj.insert("provenance_truncated".to_string(), json!(true));
            }
        }
    }
    if let Some(obj) = response["ambiguity"].as_object_mut() {
        obj.remove("reason");
    }
    if fits_budget(&mut response, max_tokens) {
        return response;
    }

    let compact_results = response["results"]
        .as_array()
        .map(|results| {
            results
                .iter()
                .take(1)
                .map(|item| {
                    let mut compact = serde_json::Map::new();
                    for key in [
                        "handle",
                        "skill_id",
                        "score",
                        "confidence",
                        "load_decision",
                        "recommended_view",
                        "source_sha256",
                    ] {
                        if let Some(value) = item.get(key).filter(|value| !value.is_null()) {
                            compact.insert(key.to_string(), value.clone());
                        }
                    }
                    if let Some(missing) = item
                        .get("missing_requirements")
                        .and_then(Value::as_array)
                        .filter(|missing| !missing.is_empty())
                    {
                        compact.insert(
                            "missing_requirements".to_string(),
                            Value::Array(missing.iter().take(2).cloned().collect()),
                        );
                    }
                    Value::Object(compact)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut compact = json!({
        "query_id": response.get("query_id").cloned().unwrap_or(Value::Null),
        "confidence": response.get("confidence").cloned().unwrap_or(Value::Null),
        "results": compact_results,
        "truncated": true,
        "omitted_results": original_count.saturating_sub(1),
        "total_indexed": response.get("total_indexed").cloned().unwrap_or(Value::Null),
    });
    if fits_budget(&mut compact, max_tokens) {
        return compact;
    }
    if let Some(results) = compact["results"].as_array_mut() {
        for item in results.iter_mut() {
            if let Some(obj) = item.as_object_mut() {
                obj.remove("handle");
            }
        }
    }
    fits_budget(&mut compact, max_tokens);
    compact
}

fn fits_budget(payload: &mut Value, max_tokens: usize) -> bool {
    stable_token_count(payload) <= max_tokens
}

fn stable_token_count(payload: &mut Value) -> usize {
    for _ in 0..8 {
        let count = estimate_tokens(&payload.to_string());
        if payload.get("tokens_estimate").and_then(Value::as_u64) == Some(count as u64) {
            return count;
        }
        payload["tokens_estimate"] = json!(count);
    }
    let count = estimate_tokens(&payload.to_string());
    payload["tokens_estimate"] = json!(count);
    count
}
fn dedupe_records(records: &mut [SkillRecord]) {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for record in records.iter_mut() {
        let count = seen.entry(record.skill_id.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            record.skill_id = format!("{}-{}-{}", record.category, record.skill_id, count);
        }
    }
}
