use crate::ast::*;
use crate::registry::AslRegistry;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use uuid::Uuid;

// =============================================================================
// Execution Log Types
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionLog {
    pub sys: SysInfo,
    pub ledger: HashMap<String, serde_json::Value>,
    pub trace: Vec<NodeTrace>,
    pub telemetry: Telemetry,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SysInfo {
    pub execution_id: String,
    pub host_agent: String,
    pub timestamp_start: String,
    pub timestamp_end: String,
    pub global_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeTrace {
    pub node: String,
    pub intent: String,
    pub safety: String,
    pub status: String,
    pub duration_ms: u64,
    pub output: serde_json::Value,
    pub validation_results: Vec<ValidationResult>,
    /// Node IDs this node depends on (for DAG visualization)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub depends_on: Vec<String>,
    /// Matched ASL registry entry ID, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asl_match: Option<String>,
    /// Warnings from ASL registry (safety mismatch, unknown std.* intent)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub asl_warnings: Vec<String>,
    /// Log of self-healing attempts (populated on RETRY)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub heal_log: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub assertion: String,
    pub passed: bool,
    pub action: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Telemetry {
    pub total_duration_ms: u64,
    pub nodes_executed: usize,
    pub nodes_failed: usize,
    pub nodes_skipped: usize,
}

// =============================================================================
// State Ledger — typed, immutable-per-write key-value store
// =============================================================================

#[derive(Debug, Clone)]
pub struct StateLedger {
    data: HashMap<String, serde_json::Value>,
}

impl Default for StateLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl StateLedger {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Write a value to the ledger, optionally validating against a declared type
    pub fn write(
        &mut self,
        address: &str,
        value: serde_json::Value,
        declared_type: Option<&AetherType>,
    ) -> Result<(), String> {
        if let Some(atype) = declared_type {
            if !atype.validate(&value) {
                return Err(format!(
                    "Type mismatch for {}: expected {:?}, got {}",
                    address, atype, value
                ));
            }
        }
        self.data.insert(address.to_string(), value);
        Ok(())
    }

    pub fn read(&self, address: &str) -> Option<&serde_json::Value> {
        self.data.get(address)
    }

    pub fn export(&self) -> HashMap<String, serde_json::Value> {
        self.data.clone()
    }
}

// =============================================================================
// Safety Gate
// =============================================================================

fn check_safety(node: &ActionNode, auto_approve_level: &SafetyLevel) -> Result<(), String> {
    let level = node
        .meta
        .as_ref()
        .and_then(|m| m.safety.as_ref())
        .cloned()
        .unwrap_or(SafetyLevel::L0Pure);

    if level > *auto_approve_level {
        return Err(format!(
            "Node {} requires {} but auto-approve is set to {}. Execution blocked.",
            node.id,
            level.label(),
            auto_approve_level.label()
        ));
    }
    Ok(())
}

// =============================================================================
// Guest Language Runners
// =============================================================================

async fn run_python(code: &str, inputs: &serde_json::Value) -> Result<serde_json::Value, String> {
    let dedented = dedent(code);

    let mut processed_code = dedented;
    let mut python_vars = Vec::new();

    if let Some(obj) = inputs.as_object() {
        for (addr, val) in obj {
            let py_name = sanitize_address(addr);
            let val_json = serde_json::to_string(val).unwrap_or_else(|_| "None".to_string());
            python_vars.push(format!("{} = json.loads(r'''{}''')", py_name, val_json));
            processed_code = processed_code.replace(addr.as_str(), &py_name);
        }
    }

    let vars_block = python_vars.join("\n");

    let base_indent = processed_code
        .lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);

    let top_level_code = processed_code
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            let indent_level = line.len() - trimmed.len();
            if trimmed.starts_with("return ") && indent_level <= base_indent {
                let indent_str = &line[..indent_level];
                format!("{}_ae_result = {}", indent_str, &trimmed["return ".len()..])
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let wrapper = format!(
        r#"import json, sys

# Inject inputs from Aether ledger
{}

# User code
_ae_result = None
{}

# Output result as JSON
if _ae_result is not None:
    print(json.dumps(_ae_result))
else:
    print("null")
"#,
        vars_block, top_level_code,
    );

    let output = Command::new("python")
        .args(["-c", &wrapper])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to spawn Python: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Python execution failed:\n{}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() || stdout == "null" {
        Ok(serde_json::Value::Null)
    } else {
        serde_json::from_str(&stdout).map_err(|e| {
            format!(
                "Python output is not valid JSON: {} (output was: {})",
                e, stdout
            )
        })
    }
}

async fn run_js(code: &str, inputs: &serde_json::Value) -> Result<serde_json::Value, String> {
    let runtime = if which_exists("bun") { "bun" } else { "node" };

    let mut processed_code = code.to_string();
    let mut js_vars = Vec::new();

    if let Some(obj) = inputs.as_object() {
        for (addr, val) in obj {
            let js_name = sanitize_address(addr);
            let val_json = serde_json::to_string(val).unwrap_or_else(|_| "null".to_string());
            js_vars.push(format!("const {} = JSON.parse(`{}`);", js_name, val_json));
            processed_code = processed_code.replace(addr.as_str(), &js_name);
        }
    }

    let vars_block = js_vars.join("\n");

    let wrapper = format!(
        r#"
{}

const _ae_exec = () => {{
    {}
}};

const _ae_result = _ae_exec();
if (_ae_result !== undefined && _ae_result !== null) {{
    console.log(JSON.stringify(_ae_result));
}} else {{
    console.log("null");
}}
"#,
        vars_block, processed_code,
    );

    let output = Command::new(runtime)
        .args(["-e", &wrapper])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to spawn {}: {}", runtime, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{} execution failed:\n{}", runtime, stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() || stdout == "null" {
        Ok(serde_json::Value::Null)
    } else {
        serde_json::from_str(&stdout).map_err(|e| {
            format!(
                "JS output is not valid JSON: {} (output was: {})",
                e, stdout
            )
        })
    }
}

async fn run_shell(code: &str, _inputs: &serde_json::Value) -> Result<serde_json::Value, String> {
    let output = Command::new("bash")
        .args(["-c", code])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to spawn shell: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        return Err(format!("Shell execution failed:\n{}", stderr));
    }

    Ok(serde_json::json!({
        "stdout": stdout,
        "stderr": stderr,
        "exit_code": output.status.code().unwrap_or(-1)
    }))
}

async fn run_text(code: &str, _inputs: &serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::String(code.to_string()))
}

async fn execute_guest(
    lang: &GuestLang,
    code: &str,
    inputs: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    match lang {
        GuestLang::Python => run_python(code, inputs).await,
        GuestLang::JS => run_js(code, inputs).await,
        GuestLang::Shell => run_shell(code, inputs).await,
        GuestLang::Text | GuestLang::TextGen => run_text(code, inputs).await,
        other => Err(format!("Runner for {:?} is not yet implemented", other)),
    }
}

// =============================================================================
// Self-Healing — LLM-driven RETRY loop
// =============================================================================

/// Call Claude Haiku to generate a fixed version of failing code.
/// Requires ANTHROPIC_API_KEY env var — silently fails if absent.
async fn heal_node_code(
    intent: &str,
    lang: &GuestLang,
    failing_code: &str,
    failed_assertion: &str,
    actual_output: &serde_json::Value,
) -> Result<String, String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY not set — self-healing disabled".to_string())?;

    let lang_name = match lang {
        GuestLang::Python => "Python",
        GuestLang::JS => "JavaScript",
        GuestLang::Shell => "Shell",
        _ => "Python",
    };

    let prompt = format!(
        "You are fixing a failing Aether action node.\n\n\
        Node intent: {intent}\n\
        Language: {lang_name}\n\n\
        Code that failed:\n{failing_code}\n\n\
        Failed validation assertion: {failed_assertion}\n\
        Actual output was: {actual}\n\n\
        Return ONLY the corrected {lang_name} code. \
        No explanation, no markdown fences, no comments about what changed. \
        The code must end with a return statement that produces the correct output.",
        intent = intent,
        lang_name = lang_name,
        failing_code = failing_code,
        failed_assertion = failed_assertion,
        actual = serde_json::to_string_pretty(actual_output).unwrap_or_default(),
    );

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 1024,
            "messages": [{ "role": "user", "content": prompt }]
        }))
        .send()
        .await
        .map_err(|e| format!("Healing API call failed: {}", e))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Healing response parse failed: {}", e))?;

    body["content"][0]["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| format!("Unexpected healing API response shape: {}", body))
}

// =============================================================================
// Expression Evaluator — for VALIDATE and CONDITION blocks
// =============================================================================

pub fn eval_expr(expr: &Expr, ledger: &StateLedger) -> Result<serde_json::Value, String> {
    match expr {
        Expr::Null => Ok(serde_json::Value::Null),
        Expr::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Expr::Int(i) => Ok(serde_json::json!(*i)),
        Expr::Float(f) => Ok(serde_json::json!(*f)),
        Expr::Str(s) => Ok(serde_json::Value::String(s.clone())),

        Expr::Address(addr) => ledger
            .read(addr)
            .cloned()
            .ok_or_else(|| format!("Address {} not found in ledger", addr)),

        Expr::Identifier(name) => {
            if let Some(v) = ledger.read(name) {
                Ok(v.clone())
            } else {
                Ok(serde_json::Value::String(name.clone()))
            }
        }

        Expr::BinOp { left, op, right } => {
            let lv = eval_expr(left, ledger)?;
            let rv = eval_expr(right, ledger)?;
            eval_binop(&lv, op, &rv)
        }

        Expr::UnaryOp { op, expr } => {
            let v = eval_expr(expr, ledger)?;
            match op {
                UnaryOperator::Not => Ok(serde_json::Value::Bool(!is_truthy(&v))),
                UnaryOperator::Neg => {
                    if let Some(n) = v.as_f64() {
                        Ok(serde_json::json!(-n))
                    } else {
                        Err(format!("Cannot negate: {}", v))
                    }
                }
            }
        }

        Expr::Index { object, key } => {
            let obj = eval_expr(object, ledger)?;
            let k = eval_expr(key, ledger)?;
            if let Some(arr) = obj.as_array() {
                if let Some(idx) = k.as_u64() {
                    arr.get(idx as usize)
                        .cloned()
                        .ok_or_else(|| format!("Index {} out of bounds", idx))
                } else {
                    Err(format!("Array index must be integer, got: {}", k))
                }
            } else if let Some(map) = obj.as_object() {
                if let Some(key_str) = k.as_str() {
                    map.get(key_str)
                        .cloned()
                        .ok_or_else(|| format!("Key '{}' not found in object", key_str))
                } else {
                    Err(format!("Object key must be string, got: {}", k))
                }
            } else {
                Err(format!("Cannot index into: {}", obj))
            }
        }

        Expr::DotAccess { object, field } => {
            let obj = eval_expr(object, ledger)?;
            if let Some(map) = obj.as_object() {
                map.get(field)
                    .cloned()
                    .ok_or_else(|| format!("Field '{}' not found in object", field))
            } else {
                Err(format!("Cannot access field '{}' on: {}", field, obj))
            }
        }

        Expr::FuncCall { name, args } => {
            let evaluated_args: Result<Vec<_>, _> =
                args.iter().map(|a| eval_expr(a, ledger)).collect();
            eval_builtin_func(name, &evaluated_args?)
        }
    }
}

fn eval_binop(
    left: &serde_json::Value,
    op: &BinOperator,
    right: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    match op {
        BinOperator::Eq => Ok(serde_json::Value::Bool(left == right)),
        BinOperator::Ne => Ok(serde_json::Value::Bool(left != right)),
        BinOperator::And => Ok(serde_json::Value::Bool(is_truthy(left) && is_truthy(right))),
        BinOperator::Or => Ok(serde_json::Value::Bool(is_truthy(left) || is_truthy(right))),
        BinOperator::Gt | BinOperator::Lt | BinOperator::Ge | BinOperator::Le => {
            let ln = left
                .as_f64()
                .ok_or_else(|| format!("Cannot compare: {}", left))?;
            let rn = right
                .as_f64()
                .ok_or_else(|| format!("Cannot compare: {}", right))?;
            let result = match op {
                BinOperator::Gt => ln > rn,
                BinOperator::Lt => ln < rn,
                BinOperator::Ge => ln >= rn,
                BinOperator::Le => ln <= rn,
                _ => unreachable!(),
            };
            Ok(serde_json::Value::Bool(result))
        }
        BinOperator::Add
        | BinOperator::Sub
        | BinOperator::Mul
        | BinOperator::Div
        | BinOperator::Mod => {
            if let (Some(ln), Some(rn)) = (left.as_f64(), right.as_f64()) {
                let result = match op {
                    BinOperator::Add => ln + rn,
                    BinOperator::Sub => ln - rn,
                    BinOperator::Mul => ln * rn,
                    BinOperator::Div => {
                        if rn == 0.0 {
                            return Err("Division by zero".to_string());
                        }
                        ln / rn
                    }
                    BinOperator::Mod => {
                        if rn == 0.0 {
                            return Err("Modulo by zero".to_string());
                        }
                        ln % rn
                    }
                    _ => unreachable!(),
                };
                Ok(serde_json::json!(result))
            } else if let BinOperator::Add = op {
                let left_s = left.to_string();
                let right_s = right.to_string();
                let ls = left.as_str().unwrap_or(&left_s);
                let rs = right.as_str().unwrap_or(&right_s);
                Ok(serde_json::Value::String(format!("{}{}", ls, rs)))
            } else {
                Err(format!("Cannot perform {:?} on {} and {}", op, left, right))
            }
        }
    }
}

fn is_truthy(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
    }
}

fn eval_builtin_func(name: &str, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
    match name {
        "len" => {
            let arg = args.first().ok_or("len() requires 1 argument")?;
            match arg {
                serde_json::Value::String(s) => Ok(serde_json::json!(s.len())),
                serde_json::Value::Array(a) => Ok(serde_json::json!(a.len())),
                serde_json::Value::Object(o) => Ok(serde_json::json!(o.len())),
                _ => Err(format!("len() not supported for: {}", arg)),
            }
        }
        "type_of" => {
            let arg = args.first().ok_or("type_of() requires 1 argument")?;
            let t = match arg {
                serde_json::Value::Null => "null",
                serde_json::Value::Bool(_) => "bool",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::String(_) => "string",
                serde_json::Value::Array(_) => "array",
                serde_json::Value::Object(_) => "object",
            };
            Ok(serde_json::Value::String(t.to_string()))
        }
        "contains" => {
            if args.len() != 2 {
                return Err("contains() requires 2 arguments".to_string());
            }
            let haystack = &args[0];
            let needle = &args[1];
            match haystack {
                serde_json::Value::String(s) => {
                    let n = needle.as_str().ok_or("contains() needle must be string")?;
                    Ok(serde_json::Value::Bool(s.contains(n)))
                }
                serde_json::Value::Array(a) => Ok(serde_json::Value::Bool(a.contains(needle))),
                _ => Err(format!("contains() not supported for: {}", haystack)),
            }
        }
        "keys" => {
            let arg = args.first().ok_or("keys() requires 1 argument")?;
            if let Some(obj) = arg.as_object() {
                let keys: Vec<_> = obj
                    .keys()
                    .map(|k| serde_json::Value::String(k.clone()))
                    .collect();
                Ok(serde_json::Value::Array(keys))
            } else {
                Err(format!("keys() requires an object, got: {}", arg))
            }
        }
        _ => Err(format!("Unknown function: {}", name)),
    }
}

// =============================================================================
// Dependency Graph & Topological Scheduler
// =============================================================================

fn build_output_map(blocks: &[Block]) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for block in blocks {
        match block {
            Block::Action(node) => {
                if let Some(outputs) = &node.outputs {
                    for out in outputs {
                        map.insert(out.address.clone(), node.id.clone());
                    }
                }
            }
            Block::Parallel(par) => {
                for node in &par.nodes {
                    if let Some(outputs) = &node.outputs {
                        for out in outputs {
                            map.insert(out.address.clone(), node.id.clone());
                        }
                    }
                }
            }
            _ => (),
        }
    }
    map
}

fn topological_sort(
    nodes: &[&ActionNode],
    output_map: &HashMap<String, String>,
) -> Result<Vec<Vec<String>>, String> {
    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

    for id in &node_ids {
        in_degree.insert(id.clone(), 0);
        adjacency.insert(id.clone(), Vec::new());
    }

    for node in nodes {
        for dep_addr in &node.depends_on {
            if let Some(producer_id) = output_map.get(dep_addr) {
                if node_ids.contains(producer_id) {
                    adjacency
                        .get_mut(producer_id)
                        .unwrap()
                        .push(node.id.clone());
                    *in_degree.get_mut(&node.id).unwrap() += 1;
                }
            }
        }
    }

    let mut levels: Vec<Vec<String>> = Vec::new();
    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(id, _)| id.clone())
        .collect();

    let mut visited = 0;

    while !queue.is_empty() {
        levels.push(queue.clone());
        let mut next_queue = Vec::new();

        for id in &queue {
            visited += 1;
            if let Some(neighbors) = adjacency.get(id) {
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        next_queue.push(neighbor.clone());
                    }
                }
            }
        }

        queue = next_queue;
    }

    if visited != node_ids.len() {
        return Err("Circular dependency detected in action nodes".to_string());
    }

    Ok(levels)
}

// =============================================================================
// Main Execution Engine
// =============================================================================

pub struct ExecutionConfig {
    pub auto_approve_level: SafetyLevel,
    /// Enable ASL registry intent validation (default: true)
    pub use_registry: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            auto_approve_level: SafetyLevel::L2StateMod,
            use_registry: true,
        }
    }
}

pub async fn execute_program(
    program: AetherProgram,
) -> Result<ExecutionLog, Box<dyn std::error::Error + Send + Sync>> {
    execute_with_config(program, ExecutionConfig::default()).await
}

pub async fn execute_with_config(
    program: AetherProgram,
    config: ExecutionConfig,
) -> Result<ExecutionLog, Box<dyn std::error::Error + Send + Sync>> {
    let mut ledger = StateLedger::new();
    let mut trace = Vec::new();
    let mut nodes_failed = 0usize;
    let nodes_skipped = 0usize;

    let registry = if config.use_registry {
        Some(AslRegistry::load())
    } else {
        None
    };

    let timestamp_start = Utc::now();
    let execution_id = format!("exe_{}", Uuid::new_v4());

    for root in &program.roots {
        // Phase 1: Load context blocks into ledger
        for block in &root.blocks {
            if let Block::Context(ctx) = block {
                for (k, v) in &ctx.data {
                    ledger
                        .write(k, v.to_json(), None)
                        .map_err(|e| format!("Context load error: {}", e))?;
                }
            }
        }

        // Phase 2: Build output map and collect action nodes
        let output_map = build_output_map(&root.blocks);

        let mut all_actions: Vec<&ActionNode> = Vec::new();

        for block in &root.blocks {
            match block {
                Block::Action(node) => {
                    all_actions.push(node);
                }
                Block::Parallel(par) => {
                    for node in &par.nodes {
                        all_actions.push(node);
                    }
                }
                _ => (),
            }
        }

        // Phase 3: Topological sort
        let levels = topological_sort(&all_actions, &output_map)
            .map_err(|e| format!("Scheduling error: {}", e))?;

        // Build node_id → [dep_node_ids] map for Lens visualization
        let mut node_deps: HashMap<String, Vec<String>> = HashMap::new();
        for node in &all_actions {
            let deps: Vec<String> = node
                .depends_on
                .iter()
                .filter_map(|addr| output_map.get(addr))
                .cloned()
                .collect();
            node_deps.insert(node.id.clone(), deps);
        }

        // Phase 4: Execute level by level
        let trace_start = trace.len();
        for level in &levels {
            let level_nodes: Vec<&ActionNode> = level
                .iter()
                .filter_map(|id| all_actions.iter().find(|n| n.id == *id))
                .cloned()
                .collect();

            if level_nodes.len() == 1 {
                let node = level_nodes[0];
                let result =
                    run_node_with_retry(node, &mut ledger, &config, registry.as_ref()).await;
                match result {
                    Ok(t) => trace.push(t),
                    Err(t) => {
                        nodes_failed += 1;
                        trace.push(t);
                    }
                }
            } else {
                // Multiple independent nodes — run in parallel
                let mut handles = Vec::new();
                for node in &level_nodes {
                    let node_clone = (*node).clone();
                    let ledger_snapshot = ledger.clone();
                    let auto_level = config.auto_approve_level.clone();
                    let use_reg = config.use_registry;
                    let reg_clone = registry.as_ref().map(|_| AslRegistry::load());

                    handles.push(tokio::spawn(async move {
                        let mut local_ledger = ledger_snapshot;
                        let local_config = ExecutionConfig {
                            auto_approve_level: auto_level,
                            use_registry: use_reg,
                        };
                        let result = run_node_with_retry(
                            &node_clone,
                            &mut local_ledger,
                            &local_config,
                            reg_clone.as_ref(),
                        )
                        .await;
                        (result, local_ledger)
                    }));
                }

                for handle in handles {
                    let (result, local_ledger) = handle
                        .await
                        .map_err(|e| format!("Task join error: {}", e))?;

                    for (k, v) in local_ledger.export() {
                        let _ = ledger.write(&k, v, None);
                    }

                    match result {
                        Ok(t) => trace.push(t),
                        Err(t) => {
                            nodes_failed += 1;
                            trace.push(t);
                        }
                    }
                }
            }
        }

        // Annotate this root's traces with dependency node IDs (for Lens)
        for t in &mut trace[trace_start..] {
            if let Some(deps) = node_deps.get(&t.node) {
                t.depends_on = deps.clone();
            }
        }
    }

    let timestamp_end = Utc::now();
    let total_ms = (timestamp_end - timestamp_start).num_milliseconds() as u64;
    let nodes_executed = trace.len();

    Ok(ExecutionLog {
        sys: SysInfo {
            execution_id,
            host_agent: "Aether_Kernel_v0.3".to_string(),
            timestamp_start: timestamp_start.to_rfc3339(),
            timestamp_end: timestamp_end.to_rfc3339(),
            global_status: if nodes_failed == 0 {
                "SUCCESS".to_string()
            } else {
                "PARTIAL_FAILURE".to_string()
            },
        },
        ledger: ledger.export(),
        trace,
        telemetry: Telemetry {
            total_duration_ms: total_ms,
            nodes_executed,
            nodes_failed,
            nodes_skipped,
        },
    })
}

// =============================================================================
// Self-Healing Retry Wrapper
// =============================================================================

/// Run a node with automatic RETRY+LLM healing on validation failures.
/// Falls back gracefully if no API key is set.
async fn run_node_with_retry(
    node: &ActionNode,
    ledger: &mut StateLedger,
    config: &ExecutionConfig,
    registry: Option<&AslRegistry>,
) -> Result<NodeTrace, NodeTrace> {
    // Find the maximum retry count across all RETRY assertions
    let max_retries = node
        .validation
        .as_ref()
        .map(|assertions| {
            assertions
                .iter()
                .filter_map(|a| match &a.on_fail {
                    HaltAction::Retry(n) => Some(n.unwrap_or(3)),
                    _ => None,
                })
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);

    let mut current_code = node.code.clone();
    let mut heal_log: Vec<String> = Vec::new();

    // Initial execution
    let mut last_result = execute_single_node(node, &current_code, ledger, config, registry).await;

    // Retry loop — only fires when RETRY assertions exist
    for attempt in 0..max_retries {
        let is_retry_failure = match &last_result {
            Err(trace) => {
                trace.status == "VALIDATION_FAILED"
                    && trace
                        .validation_results
                        .iter()
                        .any(|r| !r.passed && r.action == "RETRY")
            }
            _ => false,
        };

        if !is_retry_failure {
            break;
        }

        let failed_trace = last_result.as_ref().unwrap_err();
        let failed_assertion = failed_trace
            .validation_results
            .iter()
            .find(|r| !r.passed && r.action == "RETRY")
            .map(|r| r.assertion.clone())
            .unwrap_or_else(|| "assertion".to_string());

        let intent = node
            .meta
            .as_ref()
            .and_then(|m| m.intent.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("unknown intent");

        let actual_output = failed_trace.output.clone();

        heal_log.push(format!(
            "[attempt {}] validation failed on: {}",
            attempt + 1,
            failed_assertion
        ));

        match heal_node_code(
            intent,
            &node.language,
            &current_code,
            &failed_assertion,
            &actual_output,
        )
        .await
        {
            Ok(healed_code) => {
                heal_log.push(format!("[attempt {}] LLM healing applied", attempt + 1));
                current_code = healed_code;
                last_result =
                    execute_single_node(node, &current_code, ledger, config, registry).await;
            }
            Err(e) => {
                heal_log.push(format!(
                    "[attempt {}] healing unavailable: {}",
                    attempt + 1,
                    e
                ));
                break;
            }
        }
    }

    // Attach heal_log to whichever trace we're returning
    match last_result {
        Ok(mut t) => {
            t.heal_log = heal_log;
            Ok(t)
        }
        Err(mut t) => {
            t.heal_log = heal_log;
            Err(t)
        }
    }
}

// =============================================================================
// Single Node Executor
// =============================================================================

async fn execute_single_node(
    node: &ActionNode,
    code: &str, // May differ from node.code on retry
    ledger: &mut StateLedger,
    config: &ExecutionConfig,
    registry: Option<&AslRegistry>,
) -> Result<NodeTrace, NodeTrace> {
    let intent = node
        .meta
        .as_ref()
        .and_then(|m| m.intent.as_ref())
        .cloned()
        .unwrap_or_default();

    let safety_label = node
        .meta
        .as_ref()
        .and_then(|m| m.safety.as_ref())
        .map(|s| s.label().to_string())
        .unwrap_or_else(|| "L0 (Pure)".to_string());

    let declared_safety_str = node
        .meta
        .as_ref()
        .and_then(|m| m.safety.as_ref())
        .map(|s| match s {
            SafetyLevel::L0Pure => "l0",
            SafetyLevel::L1ReadOnly => "l1",
            SafetyLevel::L2StateMod => "l2",
            SafetyLevel::L3NetEgress => "l3",
            SafetyLevel::L4SystemRoot => "l4",
        })
        .unwrap_or("l0");

    // ASL Registry check
    let (asl_match, asl_warnings) = if let Some(reg) = registry {
        if !intent.is_empty() {
            let check = reg.check(&intent, declared_safety_str);
            let warnings = check.warning.into_iter().collect::<Vec<_>>();
            for w in &warnings {
                eprintln!("  [ASL] {}", w);
            }
            (check.matched_id, warnings)
        } else {
            (None, vec![])
        }
    } else {
        (None, vec![])
    };

    let start = Utc::now();

    // Safety gate
    if let Err(msg) = check_safety(node, &config.auto_approve_level) {
        let duration = (Utc::now() - start).num_milliseconds() as u64;
        return Err(NodeTrace {
            node: node.id.clone(),
            intent,
            safety: safety_label,
            status: "BLOCKED".to_string(),
            duration_ms: duration,
            output: serde_json::json!({ "error": msg }),
            validation_results: vec![],
            depends_on: vec![],
            asl_match,
            asl_warnings,
            heal_log: vec![],
        });
    }

    // Condition check
    if let Some(cond) = &node.condition {
        match eval_expr(cond, ledger) {
            Ok(v) if !is_truthy(&v) => {
                let duration = (Utc::now() - start).num_milliseconds() as u64;
                return Ok(NodeTrace {
                    node: node.id.clone(),
                    intent,
                    safety: safety_label,
                    status: "SKIPPED".to_string(),
                    duration_ms: duration,
                    output: serde_json::json!({ "reason": "Condition evaluated to false" }),
                    validation_results: vec![],
                    depends_on: vec![],
                    asl_match,
                    asl_warnings,
                    heal_log: vec![],
                });
            }
            Err(e) => {
                let duration = (Utc::now() - start).num_milliseconds() as u64;
                return Err(NodeTrace {
                    node: node.id.clone(),
                    intent,
                    safety: safety_label,
                    status: "ERROR".to_string(),
                    duration_ms: duration,
                    output: serde_json::json!({ "error": format!("Condition eval error: {}", e) }),
                    validation_results: vec![],
                    depends_on: vec![],
                    asl_match,
                    asl_warnings,
                    heal_log: vec![],
                });
            }
            _ => (),
        }
    }

    // Resolve inputs from ledger
    let mut input_data = serde_json::Map::new();
    if let Some(inputs) = &node.inputs {
        for binding in inputs {
            let value = match &binding.source {
                InputSource::Ref(addr) => ledger
                    .read(addr)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                InputSource::Literal(v) => v.to_json(),
            };
            input_data.insert(binding.address.clone(), value);
        }
    }
    let inputs_json = serde_json::Value::Object(input_data);

    if asl_match.is_some() {
        println!(
            "  [EXEC] {} — {} [{}] ✓ ASL:{}",
            node.id,
            intent,
            safety_label,
            asl_match.as_deref().unwrap_or("")
        );
    } else {
        println!("  [EXEC] {} — {} [{}]", node.id, intent, safety_label);
    }

    // Execute guest code
    let exec_result = execute_guest(&node.language, code, &inputs_json).await;

    let duration = (Utc::now() - start).num_milliseconds() as u64;

    match exec_result {
        Ok(output) => {
            // Snapshot ledger before writing outputs (needed for safe retry)
            let ledger_snapshot = ledger.clone();

            // Write outputs to ledger with type checking
            if let Some(out_bindings) = &node.outputs {
                for binding in out_bindings {
                    if let Err(e) = ledger.write(
                        &binding.address,
                        output.clone(),
                        Some(&binding.declared_type),
                    ) {
                        return Err(NodeTrace {
                            node: node.id.clone(),
                            intent,
                            safety: safety_label,
                            status: "TYPE_ERROR".to_string(),
                            duration_ms: duration,
                            output: serde_json::json!({ "error": e, "raw_output": output }),
                            validation_results: vec![],
                            depends_on: vec![],
                            asl_match,
                            asl_warnings,
                            heal_log: vec![],
                        });
                    }
                }
            }

            // Run validation assertions
            let mut validation_results = Vec::new();
            let mut validation_failed = false;
            let mut retry_needed = false;

            if let Some(assertions) = &node.validation {
                for assertion in assertions {
                    let result = eval_expr(&assertion.condition, ledger);
                    let passed = result.as_ref().map(is_truthy).unwrap_or(false);
                    let action_str = match &assertion.on_fail {
                        HaltAction::Halt => "HALT",
                        HaltAction::Retry(_) => "RETRY",
                        HaltAction::Warn => "WARN",
                    };

                    validation_results.push(ValidationResult {
                        assertion: format!("{:?}", assertion.condition),
                        passed,
                        action: action_str.to_string(),
                    });

                    if !passed {
                        match &assertion.on_fail {
                            HaltAction::Halt => {
                                validation_failed = true;
                            }
                            HaltAction::Warn => {
                                eprintln!("  [WARN] Validation warning in node {}", node.id);
                            }
                            HaltAction::Retry(_) => {
                                retry_needed = true;
                                validation_failed = true;
                            }
                        }
                    }
                }
            }

            if validation_failed {
                // Restore ledger to pre-write state so retry starts clean
                *ledger = ledger_snapshot;

                let _ = retry_needed; // status is VALIDATION_FAILED regardless
                let status = "VALIDATION_FAILED";
                Err(NodeTrace {
                    node: node.id.clone(),
                    intent,
                    safety: safety_label,
                    status: status.to_string(),
                    duration_ms: duration,
                    output,
                    validation_results,
                    depends_on: vec![],
                    asl_match,
                    asl_warnings,
                    heal_log: vec![],
                })
            } else {
                Ok(NodeTrace {
                    node: node.id.clone(),
                    intent,
                    safety: safety_label,
                    status: "COMPLETED".to_string(),
                    duration_ms: duration,
                    output,
                    validation_results,
                    depends_on: vec![],
                    asl_match,
                    asl_warnings,
                    heal_log: vec![],
                })
            }
        }
        Err(msg) => Err(NodeTrace {
            node: node.id.clone(),
            intent,
            safety: safety_label,
            status: "EXEC_ERROR".to_string(),
            duration_ms: duration,
            output: serde_json::json!({ "error": msg }),
            validation_results: vec![],
            depends_on: vec![],
            asl_match,
            asl_warnings,
            heal_log: vec![],
        }),
    }
}

// =============================================================================
// Utilities
// =============================================================================

fn dedent(code: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    let min_indent = lines
        .iter()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);

    let first_line_indent = if !lines[0].trim().is_empty() {
        lines[0].len() - lines[0].trim_start().len()
    } else {
        0
    };

    let effective_indent = if first_line_indent == 0 && min_indent > 0 {
        min_indent
    } else {
        std::cmp::min(first_line_indent, min_indent)
    };

    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if line.trim().is_empty() {
                String::new()
            } else {
                let this_indent = line.len() - line.trim_start().len();
                if i == 0 && first_line_indent == 0 {
                    line.to_string()
                } else if this_indent >= effective_indent {
                    line[effective_indent..].to_string()
                } else {
                    line.trim_start().to_string()
                }
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sanitize_address(addr: &str) -> String {
    let cleaned = addr.replace('$', "").replace('.', "_");
    format!("_ae_{}", cleaned)
}

fn which_exists(cmd: &str) -> bool {
    let check = if cfg!(windows) { "where" } else { "which" };
    std::process::Command::new(check)
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
