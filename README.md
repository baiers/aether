# Aether

**A deterministic orchestration language for AI agents.**

[![CI](https://github.com/baiers/aether/actions/workflows/ci.yml/badge.svg)](https://github.com/baiers/aether/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.3.0-green.svg)](https://github.com/baiers/aether/releases)

---

LLM agents fail silently, run code with no safety model, and produce pipelines that are impossible to audit. Aether is an intermediate representation (IR) that gives AI-generated pipelines **verifiable intent, typed outputs, and compile-time safety gates** — without replacing the guest languages (Python, JS, shell) your nodes already use.

**"Intent is Compilation."**

## How it works

You write (or generate) a `.ae` program describing a pipeline as a directed acyclic graph of typed nodes. Each node declares what it does (`_intent`), what safety level it requires (`_safety`), what it reads (`::IN`), what it writes (`::OUT`), and what must be true after it runs (`::VALIDATE`). The Aether kernel executes the DAG, enforces safety gates, validates outputs, and — if a node fails — optionally calls an LLM to repair the code and retry.

```
§ROOT 0xFF_MAIN {

  §ACT 0x1A {
    ::META { _intent: "fetch active users", _safety: "read_only" }
    ::EXEC<PYTHON> {
      import urllib.request, json
      return json.loads(urllib.request.urlopen("https://api.example.com/users").read())
    }
    ::OUT { $0xUSERS: Type<JSON> }
  }

  §ACT 0x2B {
    ::META { _intent: "filter to adults only", _safety: "pure" }
    ::IN   { $0xSRC: Ref($0xUSERS) }
    ::EXEC<PYTHON> {
      return [u for u in $0xSRC if u["age"] >= 18]
    }
    ::OUT      { $0xADULTS: Type<JSON> }
    ::VALIDATE { ASSERT len($0xADULTS) >= 0 OR HALT }
  }

}
```

Or use **Aether-Short** (`.as`) for ~60% fewer lines:

```
@pipeline 0xFF_MAIN

  $0xUSERS:  JSON = @std.io.net_get() {
    import urllib.request, json
    return json.loads(urllib.request.urlopen("https://api.example.com/users").read())
  }

  $0xADULTS: JSON = @std.proc.list.filter($0xUSERS) {
    return [u for u in $0xUSERS if u["age"] >= 18]
  } | ASSERT len($0xADULTS) >= 0 OR HALT

@end
```

## Why not just write Python?

| | Raw Python / LangGraph | Aether |
|---|---|---|
| **Safety model** | None | L0–L4 compile-time gates |
| **Typed outputs** | Runtime duck typing | Declared + validated at write time |
| **Audit trail** | Manual logging | Automatic `output.ae.json` with full node traces |
| **Self-healing** | Manual try/except | `ASSERT ... OR RETRY(3)` — LLM repairs the node |
| **LLM generation cost** | 715–1,225 tokens (LangGraph/AutoGen) | 415 tokens (3-node baseline) |
| **Runtime LLM calls** | 0 (LangGraph) — 6,200 (AutoGen) | 0 for deterministic pipelines |

See [`docs/benchmark.md`](docs/benchmark.md) for the full token-cost comparison.

## Features

- **5-tier safety model** — L0 (pure math) → L4 (system root). Nodes above your threshold are blocked before execution.
- **Typed state ledger** — outputs are written to an immutable address space (`$0xADDR`) with type validation on every write.
- **Parallel DAG execution** — independent nodes run concurrently via `tokio::spawn`; dependencies resolved with Kahn's topological sort.
- **ASL registry** — 32 canonical intents (`std.io.*`, `std.proc.*`, `std.ml.*`, `std.sec.*`, etc.) with safety and language defaults.
- **Self-healing RETRY** — `ASSERT expr OR RETRY(3)` sends failing code + assertion to Claude for repair. Requires your own `ANTHROPIC_API_KEY`.
- **English Toggle** — `aether gen "description"` turns plain English into a `.ae` program via Claude. Requires your own `ANTHROPIC_API_KEY`.
- **MCP server** — `aether-mcp` exposes validate/execute/inspect as tools for Claude Code and other MCP-compatible clients.
- **REST API** — `aether-api` runs on port 3737 for integration with LangChain, AutoGen, n8n, or any HTTP client.
- **Aether Lens** — a standalone DAG visualizer (`lens/index.html`) that renders any `output.ae.json` execution log.

## Installation

```bash
# Option 1: pip (recommended — no Rust required)
pip install aether-kernel

# Option 2: Pre-built binary
curl -fsSL https://raw.githubusercontent.com/baiers/aether/main/install.sh | bash

# Option 3: Build from source
cargo build --release
```

## Quick Start

```bash
# Run a pipeline
aether examples/demo.ae

# Expand Aether-Short to inspect generated .ae
aether examples/pipeline.as --expand-only

# Run with a higher safety threshold (allow network calls)
aether examples/demo_showcase.ae --safety l3

# Self-healing (requires ANTHROPIC_API_KEY)
ANTHROPIC_API_KEY=sk-... aether examples/self_heal_demo.ae

# Generate a .ae program from plain English
ANTHROPIC_API_KEY=sk-... aether gen "fetch the top 10 HN stories and summarize each one"

# Start the REST API
aether-api  # → http://localhost:3737

# Start the MCP server (for Claude Code integration)
aether-mcp
```

## Claude Code / MCP Integration

Add to your MCP config (`~/.claude/mcp_config.json`):

```json
{
  "mcpServers": {
    "aether-kernel": {
      "command": "aether-mcp",
      "args": []
    }
  }
}
```

Claude can then call `aether_validate`, `aether_execute`, and `aether_inspect` directly.

## Project Structure

```
src/          Rust kernel (parser, executor, ASL registry, self-healing, MCP, REST API)
asl/          ASL registry — 32 canonical intents (JSON)
examples/     Runnable .ae and .as programs
spec/         Formal EBNF grammar and type system docs
docs/         Whitepaper, benchmark paper, kernel manual
lens/         Aether Lens DAG visualizer (standalone HTML, no build step)
sdk/          LLM system prompt, MCP config, audit prompt
benchmark/    LangGraph / AutoGen equivalents + token counting scripts
python/       pip-installable wrapper (aether-kernel)
```

## What's Included (Community — Free, Apache 2.0)

| Feature | |
|---|---|
| Parser, AST, topological executor | ✓ |
| 5-tier safety model (L0–L4) | ✓ |
| ASL registry (32 canonical intents) | ✓ |
| Aether-Short (.as) notation | ✓ |
| Self-healing RETRY (your `ANTHROPIC_API_KEY`) | ✓ |
| English Toggle `aether gen` (your `ANTHROPIC_API_KEY`) | ✓ |
| MCP server for Claude Code | ✓ |
| REST API | ✓ |
| Aether Lens DAG visualizer | ✓ |

**Aether Pro** — hosted execution, managed LLM calls (no API key needed), extended ASL (200+ intents), persistent history. [Learn more →](https://aether-lang.dev/pro)

## License

Licensed under the [Apache License 2.0](LICENSE).

See [docs/open-core.md](docs/open-core.md) for the Community vs Pro vs Enterprise breakdown.
