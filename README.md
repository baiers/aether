# Aether (AE) Programming Language

**"Intent is Compilation"**

Aether is an AI-centric programming language designed as an intermediate representation for AI-to-AI communication and deterministic agent orchestration. It is the "machine code" of the Agentic Age.

## Core Philosophy
- **Deterministic**: Every execution node has a verifiable intent and safety level.
- **Polyglot**: Aether orchestrates logic written in Guest Languages (Python, JS, Rust, etc.) within isolated sandboxes.
- **Agent-First**: Designed for LLMs to generate and verify, rather than for humans to write manually (though it remains human-readable).
- **Immutable**: Memory is addressed via hex hashes of node outputs, preventing side effects and race conditions.

## Installation

```bash
# Option 1: pip (recommended — no Rust required)
pip install aether-kernel

# Option 2: Pre-built binary
curl -fsSL https://raw.githubusercontent.com/baiers/aether/main/install.sh | bash

# Option 3: Build from source (requires Rust toolchain)
cargo build --release
```

## Quick Start

```bash
# Run a pipeline
aether examples/demo.ae

# Run with safety level enforcement
aether examples/safety_demo.ae --safety 2

# Expand Aether-Short to full syntax
aether examples/pipeline.as --expand-only

# Self-healing (requires ANTHROPIC_API_KEY)
ANTHROPIC_API_KEY=... aether examples/self_heal_demo.ae
```

## Project Structure
- `spec/`: Formal EBNF grammar and language specification.
- `docs/`: Whitepapers, design docs, and standard library registry.
- `src/`: The Aether Kernel (Runtime) implemented in Rust.
- `examples/`: Sample `.ae` programs.
- `python/`: pip-installable Python wrapper (`aether-kernel`).
- `benchmark/`: Benchmark implementations and token counting.
- `asl/`: Aether Standard Library registry (32 canonical intents).

## What's Included (Community — Free)

| Feature | Status |
|---------|--------|
| Parser, AST, topological executor | ✓ |
| 5-tier safety model (L0–L4) | ✓ |
| ASL registry (32 canonical intents) | ✓ |
| Aether-Short (.as) notation | ✓ |
| Self-healing RETRY (your `ANTHROPIC_API_KEY`) | ✓ |
| English Toggle (`aether gen`) (your `ANTHROPIC_API_KEY`) | ✓ |
| MCP server for Claude Code | ✓ |
| REST API (`aether-api`) | ✓ |
| Aether Lens DAG visualizer | ✓ |

**Aether Pro** — hosted execution, managed LLM calls (no API key needed), extended ASL (200+ intents), persistent history, and more. [Learn more →](https://aether-lang.dev/pro)

## License

Licensed under the [Apache License 2.0](LICENSE).

See [docs/open-core.md](docs/open-core.md) for the full Community vs Pro vs Enterprise breakdown.
