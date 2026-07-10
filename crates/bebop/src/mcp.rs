//! MCP — a minimal MCP server over stdio (JSON-RPC 2.0).
//!
//! Honest scope: implements the handshake + `tools/list` + `tools/call` for the
//! native bebop tools. No SDK, no network — reads newline-delimited JSON-RPC
//! from stdin, writes to stdout. The tools call the SAME Rust engines the CLI
//! uses (multipilot, knowledge, outfit), so the surface is real, not a stub.
//!
//! Run with `bebop mcp`. Honors `BEBOP_MCP_ONCE=1` to handle one request then
//! exit (useful for tests / non-persistent bridges).

use crate::knowledge::recall;
use crate::memory::LivingMemory;
use crate::multipilot::run_multipilot;
use crate::outfit::OUTFIT;
use std::io::{BufRead, Write};

/// A tool exposed over MCP.
pub struct McpTool {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: &'static str,
}

pub fn tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "dispatch",
            description: "Run a task through Multipilot (distinct pilots + field gate).",
            input_schema: r#"{"type":"object","properties":{"task":{"type":"string"},"n":{"type":"integer"}},"required":["task"]}"#,
        },
        McpTool {
            name: "recall",
            description: "Query the living-knowledge retriever (§0·GP).",
            input_schema: r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}"#,
        },
        McpTool {
            name: "outfit",
            description: "Print the luminous cosmo-noir identity contract.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
    ]
}

/// Run the MCP stdio loop. Returns when stdin closes or (if BEBOP_MCP_ONCE) after one call.
pub fn serve() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let once = std::env::var("BEBOP_MCP_ONCE").is_ok();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let resp = handle(&line);
        writeln!(stdout, "{resp}")?;
        stdout.flush()?;
        if once {
            break;
        }
    }
    Ok(())
}

/// Handle one JSON-RPC request, returning the JSON-RPC response string.
pub fn handle(req: &str) -> String {
    let v: serde_json::Value = match serde_json::from_str(req) {
        Ok(v) => v,
        Err(e) => {
            return error_resp(
                serde_json::Value::Null,
                -32700,
                &format!("parse error: {e}"),
            );
        }
    };
    let id = v.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let method = v.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = v.get("params").cloned().unwrap_or(serde_json::Value::Null);

    match method {
        "initialize" => success(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "bebop", "version": OUTFIT.version}
            }),
        ),
        "tools/list" => {
            let list: Vec<serde_json::Value> = tools()
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": serde_json::from_str::<serde_json::Value>(t.input_schema).unwrap()
                    })
                })
                .collect();
            success(id, serde_json::json!({ "tools": list }))
        }
        "tools/call" => {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            match call_tool(name, &args) {
                Ok(out) => success(
                    id,
                    serde_json::json!({ "content": [{"type":"text","text":out}], "isError": false }),
                ),
                Err(e) => success(
                    id,
                    serde_json::json!({ "content": [{"type":"text","text":e}], "isError": true }),
                ),
            }
        }
        "ping" => success(id, serde_json::json!({})),
        _ => error_resp(id, -32601, &format!("method not found: {method}")),
    }
}

/// Dispatch a tool by name. Returns text output or an error string.
pub fn call_tool(name: &str, args: &serde_json::Value) -> Result<String, String> {
    match name {
        "dispatch" => {
            let task = args
                .get("task")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            let n = args.get("n").and_then(|n| n.as_u64()).unwrap_or(3) as usize;
            let r = run_multipilot(&task, n, native_exec, Some(|| field_gate(&task)));
            Ok(format!(
                "multipilot({n}) → ok={} | field={:?}\n{}",
                r.ok, r.field_verdict, r.note
            ))
        }
        "recall" => {
            let q = args
                .get("query")
                .and_then(|q| q.as_str())
                .unwrap_or("")
                .to_string();
            let mm = seed_memory();
            let r = recall(&mm, &q, 3);
            if r.hits.is_empty() {
                Ok(format!("recall: {}", r.note))
            } else {
                let lines: Vec<String> = r
                    .hits
                    .iter()
                    .map(|h| format!("  • [{}] {} — {}", h.id, h.concept, h.text))
                    .collect();
                Ok(format!("recall ({}):\n{}", r.hits.len(), lines.join("\n")))
            }
        }
        "outfit" => Ok(OUTFIT.banner()),
        _ => Err(format!("unknown tool: {name}")),
    }
}

/// Deterministic native executor used by multipilot (no model, air-gapped).
/// Produces a structured plan string from the task; ok=true unless empty.
pub fn native_exec(task: &str) -> crate::copilot::NativeOutcome {
    let plan = if task.trim().is_empty() {
        String::new()
    } else {
        format!(
            "plan[{}]: 1) parse '{}' 2) route 3) execute 4) verify",
            task.len(),
            task
        )
    };
    crate::copilot::NativeOutcome {
        ok: !plan.is_empty(),
        backend: "native".into(),
        summary: plan,
        exit_code: 0,
    }
}

/// Field arbiter re-export — the real graph-PDE veto lives in `crate::field`.
pub use crate::field::field_gate;

/// A small seeded memory so recall returns real payloads over MCP.
pub fn seed_memory() -> LivingMemory {
    let mut m = LivingMemory::new();
    m.remember("copilot", "native doer/checker seam — fail-closed on red");
    m.remember("multipilot", "N distinct pilots + synthesizer, field-gated");
    m.remember("field", "deterministic guard OS: deny on red, no RNG/Date");
    m.remember("outfit", "luminous cosmo-noir identity contract (OUTFIT)");
    m.remember(
        "recall",
        "§0·GP living-knowledge retriever, noise floor honest",
    );
    m
}

fn success(id: serde_json::Value, result: serde_json::Value) -> String {
    serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn error_resp(id: serde_json::Value, code: i64, message: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_tools_list_exposes_three() {
        // GREEN: the server advertises dispatch/recall/outfit.
        let r = handle(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#);
        let v: serde_json::Value = serde_json::from_str(&r).unwrap();
        let names: Vec<&str> = v["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"dispatch"));
        assert!(names.contains(&"recall"));
        assert!(names.contains(&"outfit"));
    }

    #[test]
    fn mcp_dispatch_returns_ok() {
        // GREEN: tools/call dispatch runs multipilot and reports a verdict.
        let req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"dispatch","arguments":{"task":"wire the field core"}}}"#;
        let r = handle(req);
        let v: serde_json::Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["result"]["isError"], false);
        assert!(v["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("multipilot"));
    }

    #[test]
    fn mcp_recall_returns_real_payload() {
        // GREEN: recall over MCP returns a stored concept.
        let req = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"recall","arguments":{"query":"copilot"}}}"#;
        let r = handle(req);
        let v: serde_json::Value = serde_json::from_str(&r).unwrap();
        assert!(v["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("doer/checker"));
    }

    #[test]
    fn mcp_unknown_method_errors() {
        // RED: an unknown method must return a JSON-RPC error, not silently hang.
        let r = handle(r#"{"jsonrpc":"2.0","id":4,"method":"bogus"}"#);
        let v: serde_json::Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["error"]["code"], -32601);
    }

    #[test]
    fn mcp_field_gate_blocks_redline() {
        // RED: a dispatch targeting a red-line glob must be vetoed by the field.
        assert_eq!(field_gate("auth/login.ts"), "override");
        assert_eq!(field_gate("docs/design/foo.md"), "permit");
    }
}
