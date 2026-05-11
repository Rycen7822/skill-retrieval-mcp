use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn stdio_server_lists_and_calls_skill_tools() {
    let fixture_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("retrieval_quality")
        .join("skills");
    let cache = tempfile::tempdir().unwrap().path().join("stdio-cache.json");

    let mut child = Command::new(env!("CARGO_BIN_EXE_skill-retrieval-mcp-rust"))
        .arg("--roots")
        .arg(&fixture_root)
        .arg("--cache")
        .arg(&cache)
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    writeln!(stdin, "{}", serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}
    })).unwrap();
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let init: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(init["id"], 1);
    assert_eq!(
        init["result"]["serverInfo"]["name"],
        "skill_retrieval_mcp_rust"
    );

    writeln!(
        stdin,
        "{}",
        serde_json::json!({"jsonrpc":"2.0", "method":"notifications/initialized"})
    )
    .unwrap();
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"jsonrpc":"2.0", "id": 2, "method":"tools/list"})
    )
    .unwrap();
    line.clear();
    reader.read_line(&mut line).unwrap();
    let listed: serde_json::Value = serde_json::from_str(&line).unwrap();
    let tools = listed["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|tool| tool["name"] == "skill_search"));
    assert!(tools.iter().any(|tool| tool["name"] == "skill_load"));

    writeln!(
        stdin,
        "{}",
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "skill_search",
                "arguments": {"params": {
                    "raw_user_request": "Need Obsidian backlink repair help",
                    "description_query": "obsidian vault backlinks wikilinks notes",
                    "workflow_query": "search vault edit links",
                    "category": "note-taking",
                    "k": 2,
                    "max_tokens": 800
                }}
            }
        })
    )
    .unwrap();
    line.clear();
    reader.read_line(&mut line).unwrap();
    let called: serde_json::Value = serde_json::from_str(&line).unwrap();
    let text = called["result"]["content"][0]["text"].as_str().unwrap();
    let payload: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(payload["results"][0]["skill_id"], "obsidian");

    drop(stdin);
    let _ = child.wait();
}
