//! Aether Audit — AI-powered execution log analysis.
//!
//! Takes an Aether execution log (NodeTrace JSON) and returns a structured
//! natural-language audit report via Claude. Closes the AI-to-AI loop:
//!   English Toggle → generate .ae
//!   Aether runtime → execute, produce NodeTrace
//!   Audit → Claude reads NodeTrace, reports what happened

const AUDIT_PROMPT: &str = include_str!("../sdk/AUDIT_PROMPT.md");

/// Audit an Aether execution log using Claude.
///
/// `log_json` — the full contents of an `output.ae.json` file as a string.
/// Returns a structured markdown audit report.
/// Requires `ANTHROPIC_API_KEY` environment variable.
pub async fn audit(log_json: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY not set — audit requires Claude API access")?;

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 4096,
            "system": AUDIT_PROMPT,
            "messages": [{ "role": "user", "content": log_json }]
        }))
        .send()
        .await
        .map_err(|e| format!("Audit API call failed: {}", e))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Audit response parse failed: {}", e))?;

    if let Some(err) = body.get("error") {
        return Err(format!("Anthropic API error: {}", err).into());
    }

    body["content"][0]["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| format!("Unexpected API response shape: {}", body).into())
}
