//! Aether-Short (AS) Preprocessor — Phase 2c
//!
//! Converts compact .as pipeline notation into full Aether (.ae) syntax,
//! then hands off to the standard parser. Enables "intent is compilation."
//!
//! ## Format (.as file)
//!
//! ```text
//! // comments
//! @pipeline 0xFF_NAME
//!
//!   ::CTX {
//!     $0xAPI_URL: "https://api.example.com"
//!   }
//!
//!   $0xUSERS: JSON = @std.io.net_get($0xAPI_URL) {
//!     import urllib.request, json
//!     return json.loads(urllib.request.urlopen($0xAPI_URL).read())
//!   }
//!
//!   $0xACTIVE: JSON = @std.proc.list.filter($0xUSERS) {
//!     users = $0xUSERS
//!     return [u for u in users if u.get("active", False)]
//!   } | ASSERT $0xACTIVE["count"] >= 0 OR HALT
//!
//! @end
//! ```

use crate::registry::AslRegistry;

// =============================================================================
// Public API
// =============================================================================

pub fn expand(source: &str) -> Result<String, String> {
    let registry = AslRegistry::load();
    let mut output = String::new();
    let mut node_counter: u32 = 0;
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        i += 1;

        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@pipeline") {
            let id_part = rest.trim();
            let pipeline_id = if id_part.is_empty() { "0xFF_MAIN" } else { id_part };

            output.push_str(&format!("\u{00a7}ROOT {} {{\n\n", pipeline_id));

            loop {
                if i >= lines.len() {
                    return Err("Unexpected end of file — missing @end".to_string());
                }
                let line = lines[i];
                let t = line.trim();
                i += 1;

                if t == "@end" {
                    break;
                }
                if t.is_empty() || t.starts_with("//") {
                    continue;
                }

                // Pass ::CTX blocks verbatim — already valid .ae syntax
                if t.starts_with("::CTX") {
                    let mut ctx_depth: i32 =
                        t.chars().filter(|&c| c == '{').count() as i32
                        - t.chars().filter(|&c| c == '}').count() as i32;
                    output.push_str("  ");
                    output.push_str(t);
                    output.push('\n');
                    while ctx_depth > 0 && i < lines.len() {
                        let ctx_line = lines[i];
                        i += 1;
                        ctx_depth += ctx_line.chars().filter(|&c| c == '{').count() as i32;
                        ctx_depth -= ctx_line.chars().filter(|&c| c == '}').count() as i32;
                        output.push_str("  ");
                        output.push_str(ctx_line);
                        output.push('\n');
                    }
                    output.push('\n');
                    continue;
                }

                // Node: $0xOUT: TYPE = @intent(args) [lang]? { code }
                if t.starts_with('$') && t.contains('=') && t.contains('@') {
                    node_counter += 1;
                    let (node_ae, consumed) =
                        expand_node(t, &lines[i..], &registry, node_counter)?;
                    i += consumed;
                    output.push_str(&node_ae);
                    continue;
                }

                eprintln!("[AS] Skipping unrecognized line: {}", t);
            }

            output.push_str("}\n");
        }
    }

    if output.is_empty() {
        return Err("No @pipeline blocks found in Aether-Short source".to_string());
    }

    Ok(output)
}

// =============================================================================
// Node Expansion
// =============================================================================

fn expand_node(
    header: &str,
    remaining: &[&str],
    registry: &AslRegistry,
    counter: u32,
) -> Result<(String, usize), String> {
    let node_id = format!("0xA{:02X}", counter);

    let eq_pos = header
        .find('=')
        .ok_or_else(|| format!("Expected '=' in AS node: {}", header))?;
    let lhs = header[..eq_pos].trim();
    let rhs = header[eq_pos + 1..].trim();

    // LHS: $0xOUT: TYPE
    let colon_pos = lhs
        .rfind(':')
        .ok_or_else(|| format!("Expected ':' in output declaration: {}", lhs))?;
    let out_addr = lhs[..colon_pos].trim();
    let out_type = lhs[colon_pos + 1..].trim();

    // RHS: @intent(args) [lang]? {
    if !rhs.starts_with('@') {
        return Err(format!("Expected '@intent' on RHS: {}", rhs));
    }
    let rhs_body = &rhs[1..];

    let intent_end = rhs_body
        .find(['(', ' ', '\t', '{'])
        .unwrap_or(rhs_body.len());
    let intent = &rhs_body[..intent_end];
    let after_intent = rhs_body[intent_end..].trim_start();

    // Args
    let (args, after_args) = if after_intent.starts_with('(') {
        let close = after_intent
            .find(')')
            .ok_or_else(|| format!("Missing ')': {}", after_intent))?;
        let args: Vec<String> = after_intent[1..close]
            .split(',')
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty())
            .collect();
        (args, after_intent[close + 1..].trim_start())
    } else {
        (vec![], after_intent)
    };

    let (lang_hint, after_lang) = parse_lang_hint(after_args);
    let has_code = after_lang.trim().ends_with('{');

    let (code, validate_clause, consumed) = if has_code {
        collect_code_block(remaining)?
    } else {
        (String::new(), None, 0)
    };

    // ASL registry lookup → safety + default lang
    let (safety_str, default_lang) = match registry.lookup(intent) {
        Some(entry) => {
            let s = match entry.safety.as_str() {
                "l0" => "pure",
                "l1" => "read_only",
                "l2" => "state_mod",
                "l3" => "net_egress",
                "l4" => "system_root",
                _ => "pure",
            };
            let dl = entry.recommended_lang.as_deref().unwrap_or("PYTHON").to_string();
            (s, dl)
        }
        None => ("pure", "PYTHON".to_string()),
    };

    let exec_lang = lang_hint.map(|l| l.to_uppercase()).unwrap_or(default_lang);

    // ::IN block
    let in_block: String = if args.is_empty() {
        String::new()
    } else {
        let mut s = String::from("    ::IN {\n");
        for addr in &args {
            s.push_str(&format!("      {}: Ref({})\n", addr, addr));
        }
        s.push_str("    }\n");
        s
    };

    // ::VALIDATE block
    let validate_block: String = match validate_clause {
        Some(v) => format!("    ::VALIDATE {{\n      {}\n    }}\n", v.trim()),
        None => String::new(),
    };

    // Indent code body uniformly
    let indented_code: String = code
        .lines()
        .map(|l| {
            if l.trim().is_empty() {
                String::new()
            } else {
                format!("      {}", l.trim())
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Build §ACT node
    let mut node_ae = String::new();
    node_ae.push_str(&format!("  \u{00a7}ACT {} {{\n", node_id));
    node_ae.push_str("    ::META {\n");
    node_ae.push_str(&format!("      _intent: \"{}\",\n", intent));
    node_ae.push_str(&format!("      _safety: \"{}\"\n", safety_str));
    node_ae.push_str("    }\n");
    node_ae.push_str(&in_block);
    node_ae.push_str(&format!("    ::EXEC<{}> {{\n", exec_lang));
    if !indented_code.is_empty() {
        node_ae.push_str(&indented_code);
        node_ae.push('\n');
    }
    node_ae.push_str("    }\n");
    node_ae.push_str("    ::OUT {\n");
    node_ae.push_str(&format!("      {}: Type<{}>\n", out_addr, out_type));
    node_ae.push_str("    }\n");
    node_ae.push_str(&validate_block);
    node_ae.push_str("  }\n\n");

    Ok((node_ae, consumed))
}

// =============================================================================
// Helpers
// =============================================================================

fn parse_lang_hint(s: &str) -> (Option<&str>, &str) {
    let s = s.trim_start();
    for lang in &["python", "js", "shell"] {
        if let Some(rest) = s.strip_prefix(lang) {
            return (Some(lang), rest.trim_start());
        }
    }
    (None, s)
}

/// Collect a brace-balanced code block from remaining lines.
/// Caller has already consumed the opening `{`.
/// Returns (code_text, optional_validate_clause, lines_consumed).
fn collect_code_block(lines: &[&str]) -> Result<(String, Option<String>, usize), String> {
    let mut code_lines: Vec<String> = Vec::new();
    let mut depth: i32 = 1;
    let mut consumed = 0;

    for line in lines {
        consumed += 1;
        let trimmed = line.trim();

        let mut close_byte: Option<usize> = None;
        let mut running_depth = depth;

        for (byte_idx, ch) in trimmed.char_indices() {
            match ch {
                '{' => running_depth += 1,
                '}' => {
                    running_depth -= 1;
                    if running_depth == 0 {
                        close_byte = Some(byte_idx);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(pos) = close_byte {
            let before = trimmed[..pos].trim();
            if !before.is_empty() {
                code_lines.push(before.to_string());
            }
            let after = trimmed[pos + 1..].trim();
            let validate_clause = after.strip_prefix('|').map(|stripped| stripped.trim().to_string());
            return Ok((code_lines.join("\n"), validate_clause, consumed));
        }

        depth = running_depth;
        code_lines.push(line.to_string());
    }

    Err("Unclosed '{' in Aether-Short code block".to_string())
}
