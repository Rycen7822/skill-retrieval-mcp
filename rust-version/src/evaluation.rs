use crate::SkillRetrievalEngine;
use crate::models::SearchRequest;
use crate::text::tokenize;
use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct RetrievalCase {
    pub case_id: String,
    pub intent: String,
    pub difficulty: String,
    pub language: String,
    pub relevant_skill_ids: Vec<String>,
    pub expected_top1: Option<String>,
    pub forbidden_skill_ids: Vec<String>,
    pub raw_user_request: String,
    pub description_query: String,
    pub workflow_query: String,
    pub must_have: Vec<String>,
    pub nice_to_have: Vec<String>,
    pub must_not: Vec<String>,
    pub environment: Vec<String>,
    pub category: Option<String>,
    pub k: usize,
    pub max_tokens: usize,
    pub mmr_lambda: f64,
    pub notes: String,
}

impl RetrievalCase {
    pub fn to_search_request(&self) -> SearchRequest {
        SearchRequest {
            raw_user_request: self.raw_user_request.clone(),
            description_query: self.description_query.clone(),
            workflow_query: self.workflow_query.clone(),
            must_have: self.must_have.clone(),
            nice_to_have: self.nice_to_have.clone(),
            must_not: self.must_not.clone(),
            environment: self.environment.clone(),
            category: self.category.clone(),
            k: self.k,
            max_tokens: self.max_tokens,
            mmr_lambda: self.mmr_lambda,
            ..SearchRequest::default()
        }
    }
}

pub fn load_retrieval_cases(path: impl AsRef<Path>) -> Result<Vec<RetrievalCase>> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut cases = Vec::new();
    let mut seen = BTreeSet::new();
    for (line_no, line) in content.lines().enumerate() {
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        let data: Value = serde_json::from_str(stripped)
            .with_context(|| format!("parse {}:{}", path.display(), line_no + 1))?;
        let case_id = required_str(&data, "case_id")?;
        if !seen.insert(case_id.clone()) {
            return Err(anyhow!(
                "duplicate case_id {:?} at {}:{}",
                case_id,
                path.display(),
                line_no + 1
            ));
        }
        let relevant = string_list(data.get("relevant_skill_ids"), "relevant_skill_ids")?;
        if relevant.is_empty() {
            return Err(anyhow!("{}: relevant_skill_ids must not be empty", case_id));
        }
        let expected_top1 = data
            .get("expected_top1")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);
        if expected_top1
            .as_ref()
            .is_some_and(|expected| !relevant.contains(expected))
        {
            return Err(anyhow!(
                "{}: expected_top1 must be in relevant_skill_ids",
                case_id
            ));
        }
        let forbidden = string_list(data.get("forbidden_skill_ids"), "forbidden_skill_ids")?;
        if relevant.iter().any(|id| forbidden.contains(id)) {
            return Err(anyhow!("{}: relevant and forbidden overlap", case_id));
        }
        cases.push(RetrievalCase {
            case_id,
            intent: required_str(&data, "intent")?,
            difficulty: required_str(&data, "difficulty")?,
            language: required_str(&data, "language")?,
            relevant_skill_ids: relevant,
            expected_top1,
            forbidden_skill_ids: forbidden,
            raw_user_request: opt_str(&data, "raw_user_request"),
            description_query: opt_str(&data, "description_query"),
            workflow_query: opt_str(&data, "workflow_query"),
            must_have: string_list(data.get("must_have"), "must_have")?,
            nice_to_have: string_list(data.get("nice_to_have"), "nice_to_have")?,
            must_not: string_list(data.get("must_not"), "must_not")?,
            environment: string_list(data.get("environment"), "environment")?,
            category: data
                .get("category")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned),
            k: data.get("k").and_then(Value::as_u64).unwrap_or(3) as usize,
            max_tokens: data
                .get("max_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(1200) as usize,
            mmr_lambda: data
                .get("mmr_lambda")
                .and_then(Value::as_f64)
                .unwrap_or(0.70),
            notes: opt_str(&data, "notes"),
        });
    }
    Ok(cases)
}

pub fn evaluate_retrieval_cases(
    engine: &SkillRetrievalEngine,
    cases: &[RetrievalCase],
) -> Result<Value> {
    let mut outcomes = Vec::new();
    let mut forbidden_topk_violations = Vec::new();
    let mut missing_expected_topk = Vec::new();
    let mut top1_misses = Vec::new();
    for case in cases {
        let response = engine.search(&case.to_search_request())?;
        let ranked_ids = response["results"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|item| {
                item.get("skill_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .collect::<Vec<_>>();
        let relevant = case
            .relevant_skill_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let forbidden = case
            .forbidden_skill_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let top1 = ranked_ids.first().cloned();
        let top1_hit = if let Some(expected) = &case.expected_top1 {
            top1.as_deref() == Some(expected.as_str())
        } else {
            top1.as_ref().is_some_and(|id| relevant.contains(id))
        };
        let topk = ranked_ids.iter().take(case.k).cloned().collect::<Vec<_>>();
        let hit_at_k = topk.iter().any(|id| relevant.contains(id));
        let average_precision = average_precision_at_k(&ranked_ids, &relevant, case.k);
        let judged_precision = judged_precision_at_k(&ranked_ids, &relevant, &forbidden, case.k);
        let forbidden_hits = topk
            .iter()
            .filter(|id| forbidden.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
        if !forbidden_hits.is_empty() {
            forbidden_topk_violations.push(json!({
                "case_id": case.case_id,
                "expected": case.relevant_skill_ids,
                "forbidden_hits": forbidden_hits,
                "ranked_ids": topk,
            }));
        }
        if !hit_at_k {
            missing_expected_topk.push(json!({
                "case_id": case.case_id,
                "expected": case.relevant_skill_ids,
                "ranked_ids": topk,
            }));
        }
        if !top1_hit {
            top1_misses.push(json!({
                "case_id": case.case_id,
                "expected_top1": case.expected_top1,
                "relevant": case.relevant_skill_ids,
                "top1": top1,
                "ranked_ids": ranked_ids.iter().take(case.k).cloned().collect::<Vec<_>>(),
            }));
        }
        outcomes.push(json!({
            "case_id": case.case_id,
            "intent": case.intent,
            "difficulty": case.difficulty,
            "language": case.language,
            "top1": top1,
            "ranked_ids": ranked_ids.iter().take(case.k).cloned().collect::<Vec<_>>(),
            "top1_hit": top1_hit,
            "hit_at_k": hit_at_k,
            "average_precision_at_k": round6(average_precision),
            "judged_precision_at_k": round6(judged_precision),
            "forbidden_hits": forbidden_hits,
            "tokens_estimate": response.get("tokens_estimate").cloned().unwrap_or(Value::Null),
            "confidence": response.get("confidence").cloned().unwrap_or(Value::Null),
        }));
    }
    let count = outcomes.len().max(1) as f64;
    Ok(json!({
        "case_count": outcomes.len(),
        "top1_accuracy": round4(outcomes.iter().filter(|row| row["top1_hit"].as_bool() == Some(true)).count() as f64 / count),
        "hit_rate_at_k": round4(outcomes.iter().filter(|row| row["hit_at_k"].as_bool() == Some(true)).count() as f64 / count),
        "mean_average_precision_at_k": round4(outcomes.iter().map(|row| row["average_precision_at_k"].as_f64().unwrap_or(0.0)).sum::<f64>() / count),
        "mean_judged_precision_at_k": round4(outcomes.iter().map(|row| row["judged_precision_at_k"].as_f64().unwrap_or(0.0)).sum::<f64>() / count),
        "forbidden_topk_violations": forbidden_topk_violations,
        "missing_expected_topk": missing_expected_topk,
        "top1_misses": top1_misses,
        "by_intent": bucket_rates(&outcomes, "intent"),
        "by_difficulty": bucket_rates(&outcomes, "difficulty"),
        "by_language": bucket_rates(&outcomes, "language"),
        "outcomes": outcomes,
    }))
}

pub fn evaluate_case_diversity(
    cases: &[RetrievalCase],
    near_duplicate_threshold: f64,
    mmr_lambda: f64,
    low_novelty_threshold: f64,
    top_n_pairs: usize,
) -> Value {
    let vectors = tfidf_vectors(cases);
    let mut pair_values = Vec::new();
    let mut pair_records = Vec::new();
    let mut similarities = HashMap::new();
    for i in 0..vectors.len() {
        similarities.insert((i, i), 1.0);
        for j in (i + 1)..vectors.len() {
            let cosine = cosine_sparse(&vectors[i], &vectors[j]);
            similarities.insert((i, j), cosine);
            pair_values.push(cosine);
            pair_records.push(json!({
                "case_id_a": cases[i].case_id,
                "case_id_b": cases[j].case_id,
                "cosine": round4(cosine),
                "intent_a": cases[i].intent,
                "intent_b": cases[j].intent,
                "language_a": cases[i].language,
                "language_b": cases[j].language,
                "gold_overlap": cases[i].relevant_skill_ids.iter().filter(|id| cases[j].relevant_skill_ids.contains(*id)).cloned().collect::<Vec<_>>(),
            }));
        }
    }
    pair_records.sort_by(|a, b| {
        b["cosine"]
            .as_f64()
            .unwrap_or(0.0)
            .total_cmp(&a["cosine"].as_f64().unwrap_or(0.0))
            .then_with(|| {
                a["case_id_a"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(b["case_id_a"].as_str().unwrap_or(""))
            })
            .then_with(|| {
                a["case_id_b"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(b["case_id_b"].as_str().unwrap_or(""))
            })
    });
    let near_duplicates = pair_records
        .iter()
        .filter(|row| row["cosine"].as_f64().unwrap_or(0.0) >= near_duplicate_threshold)
        .take(top_n_pairs)
        .cloned()
        .collect::<Vec<_>>();
    let (mmr_order, novelty_by_index) =
        mmr_novelty_order(cases, &vectors, &similarities, mmr_lambda);
    let low_novelty = mmr_order
        .iter()
        .skip(1)
        .filter_map(|idx| {
            let novelty = *novelty_by_index.get(idx)?;
            (novelty < low_novelty_threshold).then(|| {
                json!({
                    "case_id": cases[*idx].case_id,
                    "mmr_novelty": round4(novelty),
                    "max_prior_similarity": round4(1.0 - novelty),
                })
            })
        })
        .take(top_n_pairs)
        .collect::<Vec<_>>();
    let gold = cases
        .iter()
        .flat_map(|case| case.relevant_skill_ids.clone())
        .collect::<Vec<_>>();
    let categories = cases
        .iter()
        .map(|case| {
            case.category
                .clone()
                .unwrap_or_else(|| "uncategorized".to_string())
        })
        .collect::<Vec<_>>();
    json!({
        "case_count": cases.len(),
        "intent_entropy_norm": normalised_entropy(cases.iter().map(|case| case.intent.as_str())),
        "language_entropy_norm": normalised_entropy(cases.iter().map(|case| case.language.as_str())),
        "category_entropy_norm": normalised_entropy(categories.iter().map(String::as_str)),
        "gold_skill_entropy_norm": normalised_entropy(gold.iter().map(String::as_str)),
        "mean_pairwise_cosine": round4(if pair_values.is_empty() { 0.0 } else { pair_values.iter().sum::<f64>() / pair_values.len() as f64 }),
        "p95_pairwise_cosine": round4(percentile(&pair_values, 0.95)),
        "max_pairwise_cosine": round4(pair_values.iter().copied().fold(0.0, f64::max)),
        "near_duplicate_threshold": near_duplicate_threshold,
        "near_duplicate_pairs": near_duplicates,
        "top_similar_pairs": pair_records.into_iter().take(top_n_pairs).collect::<Vec<_>>(),
        "mmr_lambda": mmr_lambda,
        "mmr_mean_novelty": round4(if novelty_by_index.is_empty() { 0.0 } else { novelty_by_index.values().sum::<f64>() / novelty_by_index.len() as f64 }),
        "mmr_min_novelty": round4(novelty_by_index.values().copied().fold(1.0, f64::min)),
        "mmr_low_novelty_threshold": low_novelty_threshold,
        "mmr_low_novelty_cases": low_novelty,
        "mmr_selected_case_ids_head": mmr_order.iter().take(12).map(|idx| cases[*idx].case_id.clone()).collect::<Vec<_>>(),
    })
}

fn required_str(data: &Value, key: &str) -> Result<String> {
    data.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("{} must be a non-empty string", key))
}

fn opt_str(data: &Value, key: &str) -> String {
    data.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn string_list(value: Option<&Value>, field_name: &str) -> Result<Vec<String>> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => Ok(items
            .iter()
            .map(|item| item.as_str().unwrap_or("").trim().to_string())
            .filter(|item| !item.is_empty())
            .collect()),
        _ => Err(anyhow!("{} must be a list of strings", field_name)),
    }
}

fn average_precision_at_k(ranked: &[String], relevant: &BTreeSet<String>, k: usize) -> f64 {
    if relevant.is_empty() {
        return 0.0;
    }
    let mut hits = 0.0;
    let mut score = 0.0;
    for (idx, skill_id) in ranked.iter().take(k).enumerate() {
        if relevant.contains(skill_id) {
            hits += 1.0;
            score += hits / (idx + 1) as f64;
        }
    }
    score / relevant.len().min(k) as f64
}

fn judged_precision_at_k(
    ranked: &[String],
    relevant: &BTreeSet<String>,
    forbidden: &BTreeSet<String>,
    k: usize,
) -> f64 {
    let judged = ranked
        .iter()
        .take(k)
        .filter(|id| relevant.contains(*id) || forbidden.contains(*id))
        .collect::<Vec<_>>();
    if judged.is_empty() {
        1.0
    } else {
        judged
            .iter()
            .filter(|id| relevant.contains((*id).as_str()))
            .count() as f64
            / judged.len() as f64
    }
}

fn bucket_rates(outcomes: &[Value], field: &str) -> Value {
    let mut buckets: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for row in outcomes {
        let key = row[field].as_str().unwrap_or("").to_string();
        buckets.entry(key).or_default().push(row);
    }
    json!(buckets
        .into_iter()
        .map(|(key, rows)| {
            let len = rows.len().max(1) as f64;
            (key, json!({
                "case_count": rows.len(),
                "top1_accuracy": round4(rows.iter().filter(|row| row["top1_hit"].as_bool() == Some(true)).count() as f64 / len),
                "hit_rate_at_k": round4(rows.iter().filter(|row| row["hit_at_k"].as_bool() == Some(true)).count() as f64 / len),
                "mean_average_precision_at_k": round4(rows.iter().map(|row| row["average_precision_at_k"].as_f64().unwrap_or(0.0)).sum::<f64>() / len),
                "mean_judged_precision_at_k": round4(rows.iter().map(|row| row["judged_precision_at_k"].as_f64().unwrap_or(0.0)).sum::<f64>() / len),
            }))
        })
        .collect::<BTreeMap<_, _>>())
}

fn case_diversity_text(case: &RetrievalCase) -> String {
    [
        case.raw_user_request.as_str(),
        case.description_query.as_str(),
        case.workflow_query.as_str(),
        &case.must_have.join(" "),
        &case.nice_to_have.join(" "),
        &case.must_not.join(" "),
        &case.environment.join(" "),
        case.category.as_deref().unwrap_or(""),
        case.intent.as_str(),
        case.language.as_str(),
    ]
    .join("\n")
}

fn tfidf_vectors(cases: &[RetrievalCase]) -> Vec<HashMap<String, f64>> {
    let counters = cases
        .iter()
        .map(|case| token_counts(&case_diversity_text(case)))
        .collect::<Vec<_>>();
    let mut df: HashMap<String, usize> = HashMap::new();
    for counts in &counters {
        for token in counts.keys() {
            *df.entry(token.clone()).or_default() += 1;
        }
    }
    let total_docs = cases.len() as f64;
    counters
        .into_iter()
        .map(|counts| {
            let mut vector = HashMap::new();
            for (token, count) in counts {
                let tf = 1.0 + (count as f64).ln();
                let idf =
                    ((total_docs + 1.0) / (*df.get(&token).unwrap_or(&0) as f64 + 1.0)).ln() + 1.0;
                vector.insert(token, tf * idf);
            }
            let norm = vector.values().map(|v| v * v).sum::<f64>().sqrt();
            if norm > 0.0 {
                for value in vector.values_mut() {
                    *value /= norm;
                }
            }
            vector
        })
        .collect()
}

fn token_counts(text: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for token in tokenize(text) {
        *counts.entry(token).or_default() += 1;
    }
    counts
}

fn cosine_sparse(left: &HashMap<String, f64>, right: &HashMap<String, f64>) -> f64 {
    if left.len() <= right.len() {
        left.iter()
            .map(|(token, weight)| weight * right.get(token).unwrap_or(&0.0))
            .sum()
    } else {
        cosine_sparse(right, left)
    }
}

fn mmr_novelty_order(
    cases: &[RetrievalCase],
    vectors: &[HashMap<String, f64>],
    similarities: &HashMap<(usize, usize), f64>,
    mmr_lambda: f64,
) -> (Vec<usize>, HashMap<usize, f64>) {
    if cases.is_empty() {
        return (Vec::new(), HashMap::new());
    }
    let mut avg = Vec::new();
    for idx in 0..cases.len() {
        let sims = (0..cases.len())
            .filter(|other| *other != idx)
            .map(|other| {
                *similarities
                    .get(&(idx.min(other), idx.max(other)))
                    .unwrap_or(&0.0)
            })
            .collect::<Vec<_>>();
        avg.push(if sims.is_empty() {
            0.0
        } else {
            sims.iter().sum::<f64>() / sims.len() as f64
        });
    }
    let first = (0..cases.len())
        .min_by(|a, b| {
            avg[*a]
                .total_cmp(&avg[*b])
                .then_with(|| cases[*a].case_id.cmp(&cases[*b].case_id))
        })
        .unwrap_or(0);
    let mut selected = vec![first];
    let mut remaining = (0..cases.len())
        .filter(|idx| *idx != first)
        .collect::<BTreeSet<_>>();
    let mut novelty = HashMap::from([(first, 1.0)]);
    while !remaining.is_empty() {
        let mut best_idx = *remaining.iter().next().unwrap();
        let mut best_value = f64::NEG_INFINITY;
        let mut best_novelty = 0.0;
        for idx in &remaining {
            let max_similarity = selected
                .iter()
                .map(|chosen| {
                    *similarities
                        .get(&((*idx).min(*chosen), (*idx).max(*chosen)))
                        .unwrap_or(&0.0)
                })
                .fold(0.0, f64::max);
            let local_novelty = 1.0 - max_similarity;
            let lexical_breadth = f64::min(1.0, vectors[*idx].len() as f64 / 40.0);
            let value =
                mmr_lambda * (1.0 + 0.05 * lexical_breadth) - (1.0 - mmr_lambda) * max_similarity;
            if value > best_value {
                best_value = value;
                best_idx = *idx;
                best_novelty = local_novelty;
            }
        }
        selected.push(best_idx);
        novelty.insert(best_idx, best_novelty);
        remaining.remove(&best_idx);
    }
    (selected, novelty)
}

fn normalised_entropy<'a>(labels: impl Iterator<Item = &'a str>) -> f64 {
    let mut counts = HashMap::<String, usize>::new();
    for label in labels.filter(|label| !label.is_empty()) {
        *counts.entry(label.to_string()).or_default() += 1;
    }
    let total = counts.values().sum::<usize>() as f64;
    if total == 0.0 || counts.len() <= 1 {
        return 0.0;
    }
    let entropy = counts
        .values()
        .map(|count| {
            let p = *count as f64 / total;
            -p * p.ln()
        })
        .sum::<f64>();
    round4(entropy / (counts.len() as f64).ln())
}

fn percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    if ordered.len() == 1 {
        return ordered[0];
    }
    let rank = (ordered.len() - 1) as f64 * percentile;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        ordered[lower]
    } else {
        let weight = rank - lower as f64;
        ordered[lower] * (1.0 - weight) + ordered[upper] * weight
    }
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

fn round6(v: f64) -> f64 {
    (v * 1_000_000.0).round() / 1_000_000.0
}
