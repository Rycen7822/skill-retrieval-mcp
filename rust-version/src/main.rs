use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use skill_retrieval_mcp_rust::{LoadRequest, SearchRequest, SkillRetrievalEngine, evaluation, mcp};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "skill-retrieval-mcp-rust")]
#[command(about = "Pure Rust Skill Retrieval MCP utility and stdio server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(long, global = true)]
    roots: Option<String>,
    #[arg(long, global = true)]
    cache: Option<PathBuf>,
}

#[derive(Clone, Subcommand)]
enum Commands {
    Serve,
    BuildIndex,
    Search {
        #[arg(long)]
        raw: Option<String>,
        #[arg(long = "description-query")]
        description_query: Option<String>,
        #[arg(long = "workflow-query")]
        workflow_query: Option<String>,
        #[arg(long = "must-have")]
        must_have: Vec<String>,
        #[arg(long = "nice-to-have")]
        nice_to_have: Vec<String>,
        #[arg(long = "must-not")]
        must_not: Vec<String>,
        #[arg(long)]
        environment: Vec<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(short = 'k', default_value_t = 3)]
        k: usize,
        #[arg(long = "max-tokens", default_value_t = 1200)]
        max_tokens: usize,
    },
    Load {
        skill: String,
        #[arg(long, default_value = "preview")]
        view: String,
        #[arg(long = "section")]
        section_ids: Vec<String>,
        #[arg(long = "max-tokens", default_value_t = 1200)]
        max_tokens: usize,
    },
    Bench {
        #[arg(long)]
        raw: Option<String>,
        #[arg(long = "description-query")]
        description_query: Option<String>,
        #[arg(long = "workflow-query")]
        workflow_query: Option<String>,
        #[arg(short = 'k', default_value_t = 3)]
        k: usize,
        #[arg(long, default_value_t = 100)]
        iterations: usize,
    },
    Evaluate {
        #[arg(long)]
        cases: PathBuf,
        #[arg(long = "skill-root")]
        skill_roots: Vec<PathBuf>,
        #[arg(long = "skip-diversity", default_value_t = false)]
        skip_diversity: bool,
        #[arg(long = "top1-threshold", default_value_t = 0.55)]
        top1_threshold: f64,
        #[arg(long = "hit-rate-threshold", default_value_t = 0.75)]
        hit_rate_threshold: f64,
        #[arg(long = "judged-precision-threshold", default_value_t = 0.80)]
        judged_precision_threshold: f64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.clone().unwrap_or(Commands::Serve) {
        Commands::Serve => mcp::serve_stdio_with_engine(engine_from_cli(&cli)?),
        Commands::BuildIndex => {
            let engine = engine_from_cli(&cli)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "cache_status": engine.cache_status(),
                    "cache_path": engine.cache_path(),
                    "roots": engine.roots(),
                    "total_indexed": engine.records().len(),
                }))?
            );
            Ok(())
        }
        Commands::Search {
            raw,
            description_query,
            workflow_query,
            must_have,
            nice_to_have,
            must_not,
            environment,
            category,
            k,
            max_tokens,
        } => {
            let engine = engine_from_cli(&cli)?;
            let req = SearchRequest {
                raw_user_request: raw.unwrap_or_default(),
                description_query: description_query.unwrap_or_default(),
                workflow_query: workflow_query.unwrap_or_default(),
                must_have,
                nice_to_have,
                must_not,
                environment,
                category,
                k,
                max_tokens,
                ..SearchRequest::default()
            };
            println!("{}", serde_json::to_string_pretty(&engine.search(&req)?)?);
            Ok(())
        }
        Commands::Load {
            skill,
            view,
            section_ids,
            max_tokens,
        } => {
            let engine = engine_from_cli(&cli)?;
            let req = LoadRequest {
                skill_id_or_handle: skill,
                view,
                section_ids,
                max_tokens,
            };
            println!("{}", serde_json::to_string_pretty(&engine.load(&req)?)?);
            Ok(())
        }
        Commands::Bench {
            raw,
            description_query,
            workflow_query,
            k,
            iterations,
        } => {
            let started = Instant::now();
            let engine = engine_from_cli(&cli)?;
            let build_ms = started.elapsed().as_secs_f64() * 1000.0;
            let fallback_name = engine
                .records()
                .first()
                .map(|record| record.name.clone())
                .unwrap_or_else(|| "skill search".to_string());
            let search_request = SearchRequest {
                raw_user_request: raw.unwrap_or_else(|| {
                    "review code diff and choose the appropriate skill".to_string()
                }),
                description_query: description_query
                    .unwrap_or_else(|| "code review skill search".to_string()),
                workflow_query: workflow_query.unwrap_or_else(|| {
                    format!("inspect diff run verification report findings {fallback_name}")
                }),
                k,
                max_tokens: 1200,
                ..SearchRequest::default()
            };
            let iterations = iterations.max(1);
            let search_started = Instant::now();
            let mut last = None;
            for _ in 0..iterations {
                last = Some(engine.search(&search_request)?);
            }
            let total_search_ms = search_started.elapsed().as_secs_f64() * 1000.0;
            let top_skill = last
                .as_ref()
                .and_then(|value| value["results"].as_array())
                .and_then(|results| results.first())
                .and_then(|item| item["skill_id"].as_str())
                .map(ToOwned::to_owned);
            let load_ms = if let Some(skill_id) = top_skill.as_ref() {
                let request = LoadRequest {
                    skill_id_or_handle: skill_id.clone(),
                    view: "preview".to_string(),
                    section_ids: Vec::new(),
                    max_tokens: 400,
                };
                let load_started = Instant::now();
                for _ in 0..iterations {
                    let _ = engine.load(&request)?;
                }
                load_started.elapsed().as_secs_f64() * 1000.0 / iterations as f64
            } else {
                0.0
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "cache_status": engine.cache_status(),
                    "cache_path": engine.cache_path(),
                    "roots": engine.roots(),
                    "total_indexed": engine.records().len(),
                    "iterations": iterations,
                    "build_ms": round3(build_ms),
                    "avg_search_ms": round3(total_search_ms / iterations as f64),
                    "avg_load_ms": round3(load_ms),
                    "top_skill": top_skill,
                }))?
            );
            Ok(())
        }
        Commands::Evaluate {
            cases,
            skill_roots,
            skip_diversity,
            top1_threshold,
            hit_rate_threshold,
            judged_precision_threshold,
        } => {
            let cases_data = evaluation::load_retrieval_cases(&cases)?;
            let engine = if skill_roots.is_empty() {
                engine_from_cli(&cli)?
            } else {
                let cache = cache_from_cli(&cli);
                SkillRetrievalEngine::new_with_paths(skill_roots, cache)?
            };
            let mut report = evaluation::evaluate_retrieval_cases(&engine, &cases_data)?;
            let mut threshold_failures = Vec::new();
            add_threshold_failure(
                &mut threshold_failures,
                "top1_accuracy",
                report["top1_accuracy"].as_f64().unwrap_or(0.0),
                top1_threshold,
            );
            add_threshold_failure(
                &mut threshold_failures,
                "hit_rate_at_k",
                report["hit_rate_at_k"].as_f64().unwrap_or(0.0),
                hit_rate_threshold,
            );
            add_threshold_failure(
                &mut threshold_failures,
                "mean_judged_precision_at_k",
                report["mean_judged_precision_at_k"].as_f64().unwrap_or(0.0),
                judged_precision_threshold,
            );
            report["thresholds"] = json!({
                "top1_accuracy": top1_threshold,
                "hit_rate_at_k": hit_rate_threshold,
                "mean_judged_precision_at_k": judged_precision_threshold,
            });
            report["threshold_failures"] = json!(threshold_failures);
            report["cache_status"] = json!(engine.cache_status());
            report["total_indexed"] = json!(engine.records().len());
            if !skip_diversity {
                report["diversity"] =
                    evaluation::evaluate_case_diversity(&cases_data, 0.86, 0.65, 0.28, 20);
            }
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
    }
}

fn engine_from_cli(cli: &Cli) -> Result<SkillRetrievalEngine> {
    if cli.roots.is_none() && cli.cache.is_none() {
        return SkillRetrievalEngine::new();
    }
    let roots = cli
        .roots
        .as_ref()
        .map(|raw| {
            raw.split(':')
                .filter(|s| !s.trim().is_empty())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_else(|| {
            std::env::var("SRM_SKILL_ROOTS")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(|raw| raw.split(':').map(PathBuf::from).collect())
                .unwrap_or_else(|| {
                    vec![
                        PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                            .join(".hermes/skills"),
                    ]
                })
        });
    let cache = cache_from_cli(cli);
    SkillRetrievalEngine::new_with_paths(roots, cache)
}

fn cache_from_cli(cli: &Cli) -> PathBuf {
    cli.cache
        .clone()
        .or_else(|| std::env::var_os("SRM_CACHE_PATH").map(PathBuf::from))
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join(".cache/skill-retrieval-mcp-rust/index.json")
        })
}

fn add_threshold_failure(
    failures: &mut Vec<serde_json::Value>,
    metric: &str,
    actual: f64,
    threshold: f64,
) {
    if actual < threshold {
        failures.push(json!({
            "metric": metric,
            "actual": round3(actual),
            "threshold": threshold,
        }));
    }
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}
