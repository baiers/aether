//! Aether REST API Server
//! Exposes the Aether kernel over HTTP for framework-agnostic integration.
//! Works with LangChain, AutoGen, n8n, or any HTTP client.

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};

static LENS_HTML: &str = include_str!("../lens/index.html");
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

use aether_kernel::ast::SafetyLevel;
use aether_kernel::parser::parse_aether;
use aether_kernel::executor::{ExecutionConfig, ExecutionLog, execute_with_config};

// =============================================================================
// API Types
// =============================================================================

#[derive(Deserialize)]
struct ValidateRequest {
    code: String,
}

#[derive(Deserialize)]
struct ExecuteRequest {
    code: String,
    safety_level: Option<String>,
}

#[derive(Deserialize)]
struct InspectRequest {
    execution_id: String,
}

#[derive(Clone)]
struct AppState {
    logs: Arc<Mutex<HashMap<String, ExecutionLog>>>,
}

// =============================================================================
// Handlers
// =============================================================================

async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "aether-kernel",
        "version": "0.3.0",
        "tier": "community",
        "pro_available": true,
        "pro_url": "https://aether-lang.dev/pro"
    }))
}

async fn validate(Json(req): Json<ValidateRequest>) -> (StatusCode, Json<Value>) {
    match parse_aether(&req.code) {
        Ok(program) => {
            let root_count = program.roots.len();
            let node_count: usize = program.roots.iter()
                .flat_map(|r| r.blocks.iter())
                .filter(|b| matches!(b, aether_kernel::ast::Block::Action(_)))
                .count();

            (StatusCode::OK, Json(json!({
                "valid": true,
                "roots": root_count,
                "action_nodes": node_count
            })))
        }
        Err(e) => {
            (StatusCode::BAD_REQUEST, Json(json!({
                "valid": false,
                "error": e.to_string()
            })))
        }
    }
}

async fn execute(
    State(state): State<AppState>,
    Json(req): Json<ExecuteRequest>,
) -> (StatusCode, Json<Value>) {
    let safety_str = req.safety_level.as_deref().unwrap_or("l2");
    let safety_level = match SafetyLevel::from_str(safety_str) {
        Some(l) => l,
        None => return (StatusCode::BAD_REQUEST, Json(json!({
            "error": format!("Unknown safety level: {}", safety_str)
        }))),
    };

    let program = match parse_aether(&req.code) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({
            "error": format!("Parse error: {}", e)
        }))),
    };

    let config = ExecutionConfig { auto_approve_level: safety_level, ..ExecutionConfig::default() };

    match execute_with_config(program, config).await {
        Ok(log) => {
            let exec_id = log.sys.execution_id.clone();
            let result = serde_json::to_value(&log).unwrap_or(json!({"error": "serialization failed"}));

            // Store for inspection
            let mut logs = state.logs.lock().await;
            logs.insert(exec_id, log);

            (StatusCode::OK, Json(result))
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": format!("Execution error: {}", e)
            })))
        }
    }
}

async fn inspect(
    State(state): State<AppState>,
    Json(req): Json<InspectRequest>,
) -> (StatusCode, Json<Value>) {
    let logs = state.logs.lock().await;
    match logs.get(&req.execution_id) {
        Some(log) => {
            let result = serde_json::to_value(log).unwrap_or(json!({"error": "serialization failed"}));
            (StatusCode::OK, Json(result))
        }
        None => {
            (StatusCode::NOT_FOUND, Json(json!({
                "error": "Execution log not found",
                "execution_id": req.execution_id
            })))
        }
    }
}

async fn grammar() -> Json<Value> {
    // Return the Aether grammar spec for LLM consumption
    Json(json!({
        "version": "0.2",
        "syntax_summary": {
            "root": "§ROOT <hash> { <blocks> }",
            "action": "§ACT <hash> { ::META {} ::IN {} ::EXEC<LANG> {} ::OUT {} ::VALIDATE {} }",
            "parallel": "§PAR { <action_nodes> }",
            "request": "§REQ <hash> { ::SENDER: \"\" ::TARGET: \"\" ::CONTEXT {} ::INSTRUCTIONS {} }",
            "context": "::CTX { key: value }",
        },
        "languages": ["PYTHON", "JS", "RUST", "SQL", "SHELL", "TEXT", "WASM"],
        "safety_levels": {
            "l0": "Pure — no I/O, math/logic only",
            "l1": "Read-Only — GET requests, file reads",
            "l2": "State-Mod — file writes, DB inserts",
            "l3": "Net-Egress — POST requests, email, external APIs",
            "l4": "System-Root — shell access, package installs"
        },
        "types": ["Bool", "Int", "Float", "String", "JSON", "JSON_String", "JSON_Object", "Blob", "Tensor", "Ref", "Map", "List", "Table"]
    }))
}

async fn ui() -> Html<&'static str> {
    Html(LENS_HTML)
}

// =============================================================================
// Server
// =============================================================================

#[tokio::main]
async fn main() {
    let state = AppState {
        logs: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/grammar", get(grammar))
        .route("/validate", post(validate))
        .route("/execute", post(execute))
        .route("/inspect", post(inspect))
        .route("/ui", get(ui))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("AETHER_PORT").unwrap_or_else(|_| "3737".to_string());
    let addr = format!("0.0.0.0:{}", port);

    println!("=== Aether API Server v0.2 ===");
    println!("Listening on http://{}", addr);
    println!();
    println!("Endpoints:");
    println!("  GET  /health    — Server status");
    println!("  GET  /grammar   — Aether syntax reference (for LLMs)");
    println!("  POST /validate  — Validate .ae code");
    println!("  POST /execute   — Execute .ae code");
    println!("  POST /inspect   — Inspect execution log");
    println!("  GET  /ui        — Aether Lens DAG visualizer");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
