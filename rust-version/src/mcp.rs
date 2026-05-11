use crate::SkillRetrievalEngine;
use crate::models::{LoadRequest, SearchRequest};
use anyhow::Result;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

pub fn serve_stdio() -> Result<()> {
    serve_stdio_with_engine(SkillRetrievalEngine::new()?)
}

pub fn serve_stdio_with_engine(engine: SkillRetrievalEngine) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(err) => {
                write_response(
                    &mut stdout,
                    json!({"jsonrpc":"2.0", "id": null, "error":{"code": -32700, "message": err.to_string()}}),
                )?;
                continue;
            }
        };
        if request.get("id").is_none() {
            continue;
        }
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "skill_retrieval_mcp_rust", "version": env!("CARGO_PKG_VERSION")}
                }
            }),
            "tools/list" => json!({"jsonrpc":"2.0", "id": id, "result": {"tools": tools_schema()}}),
            "tools/call" => handle_tool_call(
                &engine,
                id,
                request.get("params").cloned().unwrap_or(Value::Null),
            ),
            _ => {
                json!({"jsonrpc":"2.0", "id": id, "error":{"code": -32601, "message": format!("unknown method: {}", method)}})
            }
        };
        write_response(&mut stdout, response)?;
    }
    Ok(())
}

fn write_response(out: &mut impl Write, response: Value) -> Result<()> {
    writeln!(out, "{}", serde_json::to_string(&response)?)?;
    out.flush()?;
    Ok(())
}

fn handle_tool_call(engine: &SkillRetrievalEngine, id: Value, params: Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    let inner = args.get("params").cloned().unwrap_or(args);
    let result = match name {
        "skill_search" => serde_json::from_value::<SearchRequest>(inner)
            .map_err(|e| anyhow::anyhow!(e))
            .and_then(|req| engine.search(&req)),
        "skill_load" => serde_json::from_value::<LoadRequest>(inner)
            .map_err(|e| anyhow::anyhow!(e))
            .and_then(|req| engine.load(&req)),
        _ => Err(anyhow::anyhow!("unknown tool: {}", name)),
    };
    match result {
        Ok(payload) => {
            json!({"jsonrpc":"2.0", "id": id, "result": {"content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}], "isError": false}})
        }
        Err(err) => {
            json!({"jsonrpc":"2.0", "id": id, "result": {"content": [{"type": "text", "text": json!({"error": err.to_string(), "suggestion":"Call skill_search and pass a returned handle or canonical skill_id."}).to_string()}], "isError": true}})
        }
    }
}

fn tools_schema() -> Value {
    json!([
        {
            "name": "skill_search",
            "description": "Search indexed local skills using raw request, description query, workflow query, and constraints. Returns short skill-level candidates only.",
            "annotations": {"title":"Search local skills with dual-query retrieval", "readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false},
            "inputSchema": {
                "type": "object",
                "properties": {"params": {"type":"object", "properties": {
                    "raw_user_request": {"type":"string"},
                    "description_query": {"type":"string"},
                    "workflow_query": {"type":"string"},
                    "must_have": {"type":"array", "items":{"type":"string"}},
                    "nice_to_have": {"type":"array", "items":{"type":"string"}},
                    "must_not": {"type":"array", "items":{"type":"string"}},
                    "environment": {"type":"array", "items":{"type":"string"}},
                    "category": {"type":"string"},
                    "k": {"type":"integer", "minimum":1, "maximum":10},
                    "max_tokens": {"type":"integer", "minimum":200, "maximum":4000},
                    "mmr_lambda": {"type":"number", "minimum":0, "maximum":1}
                }}}
            }
        },
        {
            "name": "skill_load",
            "description": "Load a controlled view of an indexed skill selected by skill_search.",
            "annotations": {"title":"Load selected local skill view", "readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false},
            "inputSchema": {
                "type":"object",
                "properties": {"params": {"type":"object", "properties": {
                    "skill_id_or_handle": {"type":"string"},
                    "view": {"type":"string", "enum":["card", "preview", "runtime", "risk", "sections", "full"]},
                    "section_ids": {"type":"array", "items":{"type":"string"}},
                    "max_tokens": {"type":"integer", "minimum":80, "maximum":8000}
                }, "required":["skill_id_or_handle"]}}
            }
        }
    ])
}
