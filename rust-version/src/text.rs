use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

static TOKEN_RE: OnceLock<Regex> = OnceLock::new();
static HEADING_RE: OnceLock<Regex> = OnceLock::new();

pub fn token_re() -> &'static Regex {
    TOKEN_RE.get_or_init(|| Regex::new(r"[a-zA-Z0-9_]+|[\u4e00-\u9fff]+").unwrap())
}

pub fn heading_re() -> &'static Regex {
    HEADING_RE.get_or_init(|| Regex::new(r"^(#{1,6})\s+(.+?)\s*$").unwrap())
}

pub fn tokenize(text: &str) -> Vec<String> {
    token_re()
        .find_iter(&text.to_lowercase())
        .filter_map(|m| {
            let mut tok = m.as_str().trim_matches('_').to_string();
            if tok.is_empty() || stopwords().contains(tok.as_str()) {
                return None;
            }
            if tok.len() > 3 && tok.ends_with("ing") {
                tok.truncate(tok.len() - 3);
            } else if tok.len() > 3 && tok.ends_with("ed") {
                tok.truncate(tok.len() - 2);
            } else if tok.len() > 3 && tok.ends_with('s') {
                tok.truncate(tok.len() - 1);
            }
            if tok.is_empty() { None } else { Some(tok) }
        })
        .collect()
}

fn stopwords() -> &'static BTreeSet<&'static str> {
    static STOPWORDS: OnceLock<BTreeSet<&'static str>> = OnceLock::new();
    STOPWORDS.get_or_init(|| {
        [
            "the", "a", "an", "and", "or", "to", "of", "in", "on", "for", "with", "when", "this",
            "that", "use", "using", "user", "asks", "ask", "please", "need", "needs", "be", "is",
            "are", "by", "as", "then", "after", "before", "into", "from", "it", "its", "only",
            "do", "not", "you", "your",
        ]
        .into_iter()
        .collect()
    })
}

pub fn slugify(text: &str) -> String {
    let parts: Vec<String> = token_re()
        .find_iter(&text.to_lowercase())
        .map(|m| m.as_str().trim_matches('_').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        "section".to_string()
    } else {
        parts.join("-")
    }
}

pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4).max(1)
}

pub fn trim_to_token_budget(text: &str, max_tokens: usize) -> (String, bool, usize) {
    if estimate_tokens(text) <= max_tokens {
        return (text.to_string(), false, estimate_tokens(text));
    }
    let suffix = "\n[TRUNCATED]";
    let max_chars = max_tokens.saturating_mul(4).saturating_sub(suffix.len());
    let mut out: String = text.chars().take(max_chars).collect();
    out = out.trim_end().to_string();
    out.push_str(suffix);
    let tokens = estimate_tokens(&out);
    (out, true, tokens)
}

pub fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn token_counts(text: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for token in tokenize(text) {
        *counts.entry(token).or_insert(0) += 1;
    }
    counts
}

pub fn token_set(text: &str) -> BTreeSet<String> {
    tokenize(text).into_iter().collect()
}

pub fn important_phrases(text: &str) -> Vec<String> {
    let toks = tokenize(text);
    let mut phrases = Vec::new();
    for n in 2..=usize::min(5, toks.len()) {
        for idx in 0..=toks.len() - n {
            let phrase = toks[idx..idx + n].join(" ");
            if phrase.len() > 4 {
                phrases.push(phrase);
            }
            if phrases.len() >= 80 {
                return phrases;
            }
        }
    }
    phrases
}

pub fn jaccard_sets(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    intersection / union
}
