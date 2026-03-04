# Aether Demo — Financial Transaction Anomaly Monitor

Every payment company runs a transaction monitoring pipeline. You ingest a batch, validate the schema, score each transaction for anomalous behavior, flag the high-risk ones, write an audit report, and send an alert. The pipeline is deterministic, the output must be auditable, and — critically — the code that touches external systems must not run unless you explicitly allow it.

This is where LangGraph and AutoGen require you to build the safety and reliability layer yourself. Aether has it in the language.

---

## The Pipeline

```ae
§ROOT 0xFF_FRAUD {

  §ACT 0x1A {
    ::META { _intent: "std.io.read", _safety: "pure" }
    ::EXEC<PYTHON> { return { "transactions": [ ... ], "batch_id": "BATCH-20260304-001", "count": 10 } }
    ::OUT { $0xINGESTED: Type<JSON> }
  }

  §ACT 0x2B {
    ::META { _intent: "std.proc.validate", _safety: "pure" }
    ::IN { $0xBATCH: Ref($0xINGESTED) }
    ::EXEC<PYTHON> { ... }
    ::OUT { $0xVALIDATED: Type<JSON> }
    ::VALIDATE { ASSERT $0xVALIDATED["valid_count"] == $0xVALIDATED["total_count"] OR HALT }
  }

  §ACT 0x3C {
    // DELIBERATE BUG: baseline=1.0 instead of median(amounts)
    // All 10 transactions score 1.0 — ASSERT fires — Claude repairs at runtime
    ::META { _intent: "std.ml.score", _safety: "pure" }
    ::IN { $0xDATA: Ref($0xVALIDATED) }
    ::EXEC<PYTHON> { ... }
    ::OUT { $0xSCORED: Type<JSON> }
    ::VALIDATE { ASSERT $0xSCORED["flagged_count"] < $0xSCORED["total_count"] OR RETRY(2) }
  }

  §ACT 0x4D { ::META { _intent: "std.proc.filter", _safety: "pure" } ... }
  §ACT 0x5E { ::META { _intent: "std.io.write",    _safety: "state_mod" } ... }

  // Safety gate — BLOCKED at --safety l2 (default)
  §ACT 0x6F { ::META { _intent: "std.io.send", _safety: "net_egress" } ... }
}
```

Full source: [`examples/demo_showcase.ae`](examples/demo_showcase.ae)

---

## Run It

```bash
cargo run --bin aether -- examples/demo_showcase.ae
```

No external dependencies. No API key required to see the safety model and bug detection in action. Set `ANTHROPIC_API_KEY` to see the self-healing repair.

---

## What You'll See

### Without API key — the bug is caught, the safety gate fires

```
=== Aether Kernel v0.3 ===
Parsing examples/demo_showcase.ae...
  1 root(s), 6 action node(s)
  Safety auto-approve: L2 (State-Mod)
  ASL registry: enabled

Executing...
  [EXEC] 0x1A — std.io.read    [L0 (Pure)]       ✓
  [EXEC] 0x2B — std.proc.validate [L0 (Pure)]    ✓
  [EXEC] 0x3C — std.ml.score   [L0 (Pure)]       ✗  ← ASSERT fires

=== Self-Healing Log ===
  Node 0x3C:
    [attempt 1] validation failed: flagged_count < total_count
    [attempt 1] healing unavailable: ANTHROPIC_API_KEY not set

=== Execution Complete ===
  Status: PARTIAL_FAILURE
  Nodes: 6 executed, 4 failed
```

Node 0x3C's bug caused `flagged_count=10 == total_count=10`. The assertion
`flagged_count < total_count` is false — a sane scorer cannot flag every transaction.
The pipeline stopped rather than producing a garbage audit report.

Node 0x6F shows `BLOCKED` in the execution trace because it declares `_safety: "net_egress"`
and the runtime is set to `--safety l2` (state_mod). No alert can fire until you
explicitly permit it.

---

### With API key — Claude repairs the scoring formula at runtime

```bash
ANTHROPIC_API_KEY=sk-... cargo run --bin aether -- examples/demo_showcase.ae
```

```
=== Self-Healing Log ===
  Node 0x3C:
    [attempt 1] validation failed: flagged_count < total_count
    [attempt 1] calling Claude Haiku for repair...
    [attempt 1] healed. retrying node 0x3C...
  Final status: COMPLETED

=== Execution Complete ===
  Status: PARTIAL_FAILURE   ← 0x6F still BLOCKED (net_egress gate)
  Nodes: 6 executed, 1 failed

=== State Ledger ===
  $0xREPORT = {
    "report": "=== FRAUD DETECTION AUDIT REPORT ===\n
               Batch:          BATCH-20260304-001\n
               Transactions:   3 flagged\n
               Total exposure: $37,450.00\n\n
               Flagged Transactions:\n
                 [CRITICAL] TX008: $9,850.00 via Cash Transfer Service (NG) — score 1.0\n
                 [CRITICAL] TX009: $12,400.00 via Crypto Exchange (RU) — score 1.0\n
                 [CRITICAL] TX010: $15,200.00 via Unregistered Vendor (CN) — score 1.0\n\n
               Recommended action: BLOCK_AND_REVIEW all flagged transactions.",
    "flagged_ids": ["TX008", "TX009", "TX010"],
    "exposure_usd": 37450.0,
    "status": "AUDIT_COMPLETE"
  }
```

Claude received the broken code, the failed assertion, and the actual output
(all scores=1.0). It rewrote the baseline computation to use `median(amounts)`
from the batch. On retry, only the three high-value transfers scored above the
threshold. The pipeline continued.

To send the alert: `--safety l3` raises the auto-approve ceiling to include net_egress.

---

## The Execution Trail (NodeTrace)

Every execution writes `output.ae.json`. Excerpt:

```json
{
  "trace": [
    {
      "node": "0x3C",
      "intent": "std.ml.score",
      "safety": "L0 (Pure)",
      "status": "COMPLETED",
      "duration_ms": 312,
      "output": {
        "flagged_count": 3,
        "total_count": 10,
        "transactions": [
          { "id": "TX001", "amount": 45.0,    "anomaly_score": 0.022 },
          { "id": "TX008", "amount": 9850.0,  "anomaly_score": 1.0   },
          ...
        ]
      },
      "validation_results": [
        { "assertion": "flagged_count < total_count", "passed": true }
      ],
      "heal_log": [
        "[attempt 1] validation failed: flagged_count < total_count",
        "[attempt 1] healed. retrying node 0x3C..."
      ],
      "depends_on": ["0x2B"]
    },
    {
      "node": "0x6F",
      "intent": "std.io.send",
      "safety": "L3 (Net-Egress)",
      "status": "BLOCKED",
      "output": {
        "error": "Node 0x6F requires L3 (Net-Egress) but auto-approve is set to L2 (State-Mod). Execution blocked."
      }
    }
  ],
  "sys": { "global_status": "PARTIAL_FAILURE" },
  "telemetry": { "nodes_executed": 6, "nodes_failed": 1, "total_duration_ms": 490 }
}
```

Load `output.ae.json` into [Aether Lens](lens/index.html) to see the DAG rendered
with node status, timing, and healing info as a visual graph.

---

## Why Not LangGraph?

LangGraph is a good library for building stateful, multi-step pipelines. This
comparison is not about capability — it is about what you build vs. what you get.

| Concern | Aether | LangGraph |
|---------|--------|-----------|
| **Safety gates** | `_safety: "net_egress"` blocks the node before it runs — enforced by the runtime | No equivalent. You add middleware, policy checks, or conditional edges yourself |
| **Self-healing** | `OR RETRY(n)` in `::VALIDATE` sends the failing code + evidence to Claude for repair | No equivalent. You build the repair infrastructure (~100–150 lines) and wire it in |
| **Audit trail** | Every node's output, timing, validation results, and heal_log are captured in `output.ae.json` automatically | You instrument each node manually and aggregate the results |
| **Determinism** | Zero LLM calls for this pipeline (unless RETRY fires). Cost is fixed and predictable | Depends on how you build it. AutoGen-style agents burn 2,000–6,000 tokens per run for equivalent pipelines |

**When LangGraph is the right choice:**
- You need complex conditional branching with human-in-the-loop steps
- Your team already uses Python exclusively and has no need for a typed audit trail
- Your pipeline evolves constantly and a fixed DAG syntax is too rigid

**When Aether is the right choice:**
- You need a written record of exactly what ran, what it produced, and why it passed or failed — without building that yourself
- You have nodes that touch external systems and you want the runtime to enforce a capability boundary
- You want runtime code repair without wiring it into your graph logic

The benchmark paper ([`docs/benchmark.md`](docs/benchmark.md)) has the full
token-cost comparison across four pipeline types, including convergence analysis
showing where LangGraph's fixed overhead amortizes at scale.

---

## Project Structure

```
src/            Rust kernel: parser, executor, safety model, self-healing
  aether.pest   PEG grammar
  executor.rs   Topological scheduler, RETRY loop, Claude API calls
  translate.rs  English Toggle: natural language → .ae via Claude
examples/       Sample pipelines
  demo_showcase.ae   ← this demo
  demo.ae            3-node user filter (B1 benchmark baseline)
asl/            Abstract Standard Library registry (32 intents)
lens/           Aether Lens: standalone DAG visualizer (open index.html)
sdk/            SYSTEM_PROMPT.md for LLM code generation
benchmark/      Token-count scripts and framework equivalents
docs/           Benchmark paper (v2.0)
```

Three binaries: `aether` (CLI), `aether-mcp` (MCP server), `aether-api` (REST on :3737).
