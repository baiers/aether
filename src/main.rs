use std::fs;

use aether_kernel::ast::SafetyLevel;
use aether_kernel::audit;
use aether_kernel::executor::{execute_with_config, ExecutionConfig};
use aether_kernel::parser::parse_aether;
use aether_kernel::short;
use aether_kernel::translate;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "translate" | "gen" => return cmd_translate(&args[2..]).await,
        "audit" => return cmd_audit(&args[2..]).await,
        "--help" | "-h" => {
            print_usage();
            return Ok(());
        }
        "--version" | "-v" => {
            println!("Aether Kernel v0.3.0");
            return Ok(());
        }
        _ => {}
    }

    cmd_run(&args[1..]).await
}

// =============================================================================
// translate subcommand
// =============================================================================

async fn cmd_translate(args: &[String]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut description: Option<String> = None;
    let mut run_after = false;
    let mut output_path: Option<String> = None;
    let mut safety_level = SafetyLevel::L2StateMod;
    let mut no_registry = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--run" | "-r" => {
                run_after = true;
            }
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    output_path = Some(args[i].clone());
                }
            }
            "--safety" | "-s" => {
                i += 1;
                if i < args.len() {
                    safety_level = SafetyLevel::from_str(&args[i])
                        .ok_or_else(|| format!("Unknown safety level: {}", args[i]))?;
                }
            }
            "--no-registry" => {
                no_registry = true;
            }
            "--help" | "-h" => {
                print_translate_usage();
                return Ok(());
            }
            _ => {
                description = Some(args[i].clone());
            }
        }
        i += 1;
    }

    let description = description.ok_or(
        "No description provided.\n\
         Usage: aether translate \"<natural language description>\" [--run] [-o output.ae]",
    )?;

    println!("=== Aether English Toggle ===");
    println!("Translating: \"{}\"", description);
    println!("Calling Claude...");
    println!();

    let code = translate::translate(&description)
        .await
        .map_err(|e| format!("Translation failed: {}", e))?;

    println!("{}", code);

    // Optionally save to file
    if let Some(ref path) = output_path {
        fs::write(path, &code)?;
        println!();
        println!("Saved to {}", path);
    }

    // Optionally execute immediately
    if run_after {
        println!();
        println!("=== Executing Generated Program ===");

        let program =
            parse_aether(&code).map_err(|e| format!("Parse error in generated code: {}", e))?;

        let root_count = program.roots.len();
        let node_count: usize = program
            .roots
            .iter()
            .flat_map(|r| r.blocks.iter())
            .filter(|b| matches!(b, aether_kernel::ast::Block::Action(_)))
            .count();

        println!("  {} root(s), {} action node(s)", root_count, node_count);
        println!("  Safety auto-approve: {}", safety_level.label());
        println!();

        let config = ExecutionConfig {
            auto_approve_level: safety_level,
            use_registry: !no_registry,
            strict_registry: false,
        };

        let log = execute_with_config(program, config).await?;

        println!("=== Execution Complete ===");
        println!("  Status: {}", log.sys.global_status);
        println!(
            "  Nodes: {} executed, {} failed",
            log.telemetry.nodes_executed, log.telemetry.nodes_failed
        );
        println!("  Duration: {}ms", log.telemetry.total_duration_ms);

        let healed_nodes: Vec<_> = log
            .trace
            .iter()
            .filter(|t| !t.heal_log.is_empty())
            .collect();
        if !healed_nodes.is_empty() {
            println!();
            println!("=== Self-Healing Log ===");
            for t in healed_nodes {
                println!("  Node {}:", t.node);
                for entry in &t.heal_log {
                    println!("    {}", entry);
                }
                println!("  Final status: {}", t.status);
            }
        }

        let log_path = output_path
            .as_deref()
            .map(|p| format!("{}.json", p))
            .unwrap_or_else(|| "output.ae.json".to_string());

        let log_json = serde_json::to_string_pretty(&log)?;
        fs::write(&log_path, &log_json)?;
        println!("  Log: {}", log_path);

        if !log.ledger.is_empty() {
            println!();
            println!("=== State Ledger ===");
            for (k, v) in &log.ledger {
                let display = if v.is_string() {
                    format!("\"{}\"", v.as_str().unwrap())
                } else {
                    let s = v.to_string();
                    if s.len() > 80 {
                        format!("{}...", &s[..80])
                    } else {
                        s
                    }
                };
                println!("  {} = {}", k, display);
            }
        }
    }

    Ok(())
}

// =============================================================================
// run subcommand (original behavior)
// =============================================================================

async fn cmd_run(args: &[String]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut file_path = None;
    let mut safety_level = SafetyLevel::L2StateMod;
    let mut output_path = String::from("output.ae.json");
    let mut no_registry = false;
    let mut strict_registry = false;
    let mut expand_only = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--safety" | "-s" => {
                i += 1;
                if i < args.len() {
                    safety_level = SafetyLevel::from_str(&args[i])
                        .ok_or_else(|| format!("Unknown safety level: {}", args[i]))?;
                }
            }
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    output_path = args[i].clone();
                }
            }
            "--no-registry" => {
                no_registry = true;
            }
            "--strict-registry" => {
                strict_registry = true;
            }
            "--expand-only" | "-e" => {
                expand_only = true;
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            "--version" | "-v" => {
                println!("Aether Kernel v0.3.0");
                return Ok(());
            }
            _ => {
                file_path = Some(args[i].clone());
            }
        }
        i += 1;
    }

    let file_path = file_path.ok_or("No input file specified")?;
    let raw =
        fs::read_to_string(&file_path).map_err(|e| format!("Cannot read {}: {}", file_path, e))?;

    // Aether-Short: auto-expand .as files to full .ae before parsing
    let is_short = file_path.ends_with(".as");
    let content = if is_short {
        let expanded =
            short::expand(&raw).map_err(|e| format!("Aether-Short expansion error: {}", e))?;

        if expand_only {
            println!("{}", expanded);
            return Ok(());
        }

        expanded
    } else {
        if expand_only {
            println!("{}", raw);
            return Ok(());
        }
        raw
    };

    println!("=== Aether Kernel v0.3 ===");
    if is_short {
        println!("Expanding {} (Aether-Short)...", file_path);
    } else {
        println!("Parsing {}...", file_path);
    }

    let program = parse_aether(&content)?;

    let root_count = program.roots.len();
    let node_count: usize = program
        .roots
        .iter()
        .flat_map(|r| r.blocks.iter())
        .filter(|b| matches!(b, aether_kernel::ast::Block::Action(_)))
        .count();

    println!("  {} root(s), {} action node(s)", root_count, node_count);
    println!("  Safety auto-approve: {}", safety_level.label());
    if !no_registry {
        print!("  ASL registry: enabled");
        if strict_registry {
            print!(" (strict)");
        }
        println!();
    }
    println!();
    println!("Executing...");

    let config = ExecutionConfig {
        auto_approve_level: safety_level,
        use_registry: !no_registry,
        strict_registry,
    };

    let log = execute_with_config(program, config).await?;

    println!();
    println!("=== Execution Complete ===");
    println!("  Status: {}", log.sys.global_status);
    println!(
        "  Nodes: {} executed, {} failed",
        log.telemetry.nodes_executed, log.telemetry.nodes_failed
    );
    println!("  Duration: {}ms", log.telemetry.total_duration_ms);

    // Print self-healing summary
    let healed_nodes: Vec<_> = log
        .trace
        .iter()
        .filter(|t| !t.heal_log.is_empty())
        .collect();
    if !healed_nodes.is_empty() {
        println!();
        println!("=== Self-Healing Log ===");
        for t in healed_nodes {
            println!("  Node {}:", t.node);
            for entry in &t.heal_log {
                println!("    {}", entry);
            }
            println!("  Final status: {}", t.status);
        }
    }

    let log_json = serde_json::to_string_pretty(&log)?;
    fs::write(&output_path, &log_json)?;
    println!("  Log: {}", output_path);

    if !log.ledger.is_empty() {
        println!();
        println!("=== State Ledger ===");
        for (k, v) in &log.ledger {
            let display = if v.is_string() {
                format!("\"{}\"", v.as_str().unwrap())
            } else {
                let s = v.to_string();
                if s.len() > 80 {
                    format!("{}...", &s[..80])
                } else {
                    s
                }
            };
            println!("  {} = {}", k, display);
        }
    }

    Ok(())
}

// =============================================================================
// audit subcommand
// =============================================================================

async fn cmd_audit(args: &[String]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut file_path = String::from("output.ae.json");

    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("aether audit — AI Execution Auditor");
                println!();
                println!("USAGE:");
                println!("    aether audit [file.ae.json]");
                println!();
                println!("Reads an Aether execution log and returns a structured audit report.");
                println!("Defaults to output.ae.json in the current directory.");
                println!();
                println!("Requires ANTHROPIC_API_KEY environment variable.");
                return Ok(());
            }
            path => {
                file_path = path.to_string();
            }
        }
    }

    let log_json =
        fs::read_to_string(&file_path).map_err(|e| format!("Cannot read {}: {}", file_path, e))?;

    println!("=== Aether Audit ===");
    println!("Auditing: {}", file_path);
    println!("Calling Claude...");
    println!();

    let report = audit::audit(&log_json)
        .await
        .map_err(|e| format!("Audit failed: {}", e))?;

    println!("{}", report);

    Ok(())
}

fn print_usage() {
    println!("Aether Kernel v0.3.0 — The Runtime for the Agentic Age");
    println!();
    println!("USAGE:");
    println!("    aether <file.ae|file.as> [OPTIONS]");
    println!("    aether translate \"<description>\" [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    translate, gen      English Toggle: natural language -> .ae code");
    println!("    audit               AI Audit: Claude reads execution log, reports what happened");
    println!();
    println!("OPTIONS (run):");
    println!("    -s, --safety <level>    Auto-approve safety level (default: l2)");
    println!("                            l0=pure  l1=read-only  l2=state-mod");
    println!("                            l3=net-egress  l4=system-root");
    println!("    -o, --output <path>     Output log path (default: output.ae.json)");
    println!("    -e, --expand-only       Print expanded .ae and exit (for .as files)");
    println!("        --no-registry       Disable ASL registry validation");
    println!("    -v, --version           Print version");
    println!("    -h, --help              Print this help");
    println!();
    println!("OPTIONS (translate):");
    println!("    -r, --run               Execute the generated program immediately");
    println!("    -o, --output <path>     Save generated .ae to file");
    println!("    -s, --safety <level>    Safety level for --run (default: l2)");
    println!();
    println!("AETHER-SHORT:");
    println!("    .as files use compact pipeline notation, auto-expanded to .ae.");
    println!("    Use --expand-only to inspect the generated .ae code.");
    println!();
    println!("SELF-HEALING:");
    println!("    Set ANTHROPIC_API_KEY to enable automatic RETRY healing via Claude Haiku.");
    println!();
    println!("ENGLISH TOGGLE:");
    println!("    Set ANTHROPIC_API_KEY to enable natural language -> .ae translation.");
    println!();
    println!("AI AUDIT:");
    println!("    Set ANTHROPIC_API_KEY to enable Claude-powered execution log analysis.");
}

fn print_translate_usage() {
    println!("aether translate — English Toggle");
    println!();
    println!("USAGE:");
    println!("    aether translate \"<natural language description>\" [OPTIONS]");
    println!("    aether gen \"<description>\" [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -r, --run               Execute the generated .ae program immediately");
    println!("    -o, --output <path>     Save generated .ae code to file");
    println!("    -s, --safety <level>    Safety level for --run (default: l2)");
    println!("        --no-registry       Disable ASL registry for --run");
    println!("    -h, --help              Print this help");
    println!();
    println!("EXAMPLE:");
    println!("    aether translate \"fetch users from API, filter to active ones, save count\"");
    println!("    aether gen \"compute mean and std of 1000 random numbers\" --run");
    println!("    aether translate \"summarize a CSV file\" -o pipeline.ae --run");
    println!();
    println!("Requires ANTHROPIC_API_KEY environment variable.");
}
