//! Aether MCP Server — Model Context Protocol interface
//! Exposes the Aether kernel as tools for any MCP-compatible LLM client.
//! Protocol: JSON-RPC 2.0 over stdio

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

use aether_kernel::ast::SafetyLevel;
use aether_kernel::audit;
use aether_kernel::executor::{execute_with_config, ExecutionConfig, ExecutionLog};
use aether_kernel::parser::parse_aether;

// =============================================================================
// JSON-RPC Types
// =============================================================================

#[derive(Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

// =============================================================================
// MCP Protocol Handlers
// =============================================================================

struct McpState {
    execution_logs: HashMap<String, ExecutionLog>,
}

fn handle_initialize(_params: Option<Value>) -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "aether-kernel",
            "version": "0.3.0"
        }
    })
}

fn handle_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "aether_validate",
                "description": "Validate Aether (.ae) code without executing it. Returns parse results and identified issues.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "The Aether code to validate"
                        }
                    },
                    "required": ["code"]
                }
            },
            {
                "name": "aether_execute",
                "description": "Execute Aether (.ae) code. Parses, validates safety levels, runs guest language code in sandboxes, and returns a structured execution log with the state ledger.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "The Aether code to execute"
                        },
                        "safety_level": {
                            "type": "string",
                            "description": "Maximum auto-approved safety level: l0, l1, l2 (default), l3, l4",
                            "default": "l2"
                        }
                    },
                    "required": ["code"]
                }
            },
            {
                "name": "aether_audit",
                "description": "Audit an Aether execution log with Claude. Returns a structured natural-language report: what ran, what succeeded, what was self-healed, and what was blocked by safety gates. Use after aether_execute to close the AI-to-AI loop.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "execution_log": {
                            "type": "string",
                            "description": "The execution log JSON string (output of aether_execute, serialized)"
                        }
                    },
                    "required": ["execution_log"]
                }
            },
            {
                "name": "aether_inspect",
                "description": "Inspect a previous execution log by its ID. Returns the full trace, ledger state, and telemetry.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "execution_id": {
                            "type": "string",
                            "description": "The execution ID from a previous aether_execute call"
                        }
                    },
                    "required": ["execution_id"]
                }
            }
        ]
    })
}

async fn handle_tool_call(
    name: &str,
    arguments: &Value,
    state: &Arc<Mutex<McpState>>,
) -> Result<Value, String> {
    match name {
        "aether_validate" => {
            let code = arguments
                .get("code")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'code' argument")?;

            match parse_aether(code) {
                Ok(program) => {
                    let root_count = program.roots.len();
                    let node_count: usize = program
                        .roots
                        .iter()
                        .flat_map(|r| r.blocks.iter())
                        .filter(|b| matches!(b, aether_kernel::ast::Block::Action(_)))
                        .count();

                    Ok(json!({
                        "valid": true,
                        "roots": root_count,
                        "action_nodes": node_count,
                        "program": program
                    }))
                }
                Err(e) => Ok(json!({
                    "valid": false,
                    "error": e.to_string()
                })),
            }
        }

        "aether_execute" => {
            let code = arguments
                .get("code")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'code' argument")?;

            let safety_str = arguments
                .get("safety_level")
                .and_then(|v| v.as_str())
                .unwrap_or("l2");

            let safety_level = SafetyLevel::from_str(safety_str)
                .ok_or_else(|| format!("Unknown safety level: {}", safety_str))?;

            let program = parse_aether(code).map_err(|e| format!("Parse error: {}", e))?;

            let config = ExecutionConfig {
                auto_approve_level: safety_level,
                ..ExecutionConfig::default()
            };

            let log = execute_with_config(program, config)
                .await
                .map_err(|e| format!("Execution error: {}", e))?;

            let exec_id = log.sys.execution_id.clone();
            let result =
                serde_json::to_value(&log).map_err(|e| format!("Serialization error: {}", e))?;

            // Store for later inspection
            let mut s = state.lock().await;
            s.execution_logs.insert(exec_id, log);

            Ok(result)
        }

        "aether_audit" => {
            let log_json = arguments
                .get("execution_log")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'execution_log' argument")?;

            let report = audit::audit(log_json)
                .await
                .map_err(|e| format!("Audit failed: {}", e))?;

            Ok(json!({ "report": report }))
        }

        "aether_inspect" => {
            let exec_id = arguments
                .get("execution_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'execution_id' argument")?;

            let s = state.lock().await;
            match s.execution_logs.get(exec_id) {
                Some(log) => {
                    serde_json::to_value(log).map_err(|e| format!("Serialization error: {}", e))
                }
                None => Ok(json!({
                    "error": "Execution log not found",
                    "execution_id": exec_id
                })),
            }
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}

// =============================================================================
// Main Loop
// =============================================================================

#[tokio::main]
async fn main() {
    eprintln!("Aether MCP Server v0.2 starting...");

    let state = Arc::new(Mutex::new(McpState {
        execution_logs: HashMap::new(),
    }));

    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let err_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
                let _ = writeln!(
                    stdout.lock(),
                    "{}",
                    serde_json::to_string(&err_response).unwrap()
                );
                continue;
            }
        };

        let id = request.id.clone().unwrap_or(Value::Null);

        let response = match request.method.as_str() {
            "initialize" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(handle_initialize(request.params)),
                error: None,
            },

            "notifications/initialized" => continue, // No response needed

            "tools/list" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(handle_tools_list()),
                error: None,
            },

            "tools/call" => {
                let params = request.params.unwrap_or(json!({}));
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

                match handle_tool_call(tool_name, &arguments, &state).await {
                    Ok(result) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                            }]
                        })),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Error: {}", e)
                            }],
                            "isError": true
                        })),
                        error: None,
                    },
                }
            }

            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        };

        let response_json = serde_json::to_string(&response).unwrap();
        let _ = writeln!(stdout.lock(), "{}", response_json);
        let _ = stdout.lock().flush();
    }
}
