# Aether vs. LangGraph vs. AutoGen: A Structural Benchmark

**Version**: 2.0 — Aether Kernel v0.3.0
**Date**: 2026-03-04
**Reproducible**: `python benchmark/token_count.py`

---

## Abstract

We benchmark Aether, LangGraph, and AutoGen across four pipeline tasks — a 3-node
pure computation pipeline, a 5-node conditional branching pipeline, a 4-node mixed
LLM + deterministic pipeline, and an 8-node long pipeline with injected failure —
measuring source token cost, runtime LLM consumption, structural safety guarantees,
and error recovery behavior. Aether encodes the 3-node reference pipeline in **415
tokens**, 42% fewer than LangGraph (715) and 66% fewer than AutoGen (1,225 source).
This advantage **narrows significantly at scale**: to 20% over LangGraph on mixed
LLM pipelines (Benchmark 3), and to 7–8% on conditional branching and long pipelines
(Benchmarks 2 and 4). On Benchmark 4, AutoGen source tokens (1,246) are within 1% of
Aether (1,235). Aether is the only framework with a built-in, compile-time safety model
and produces zero LLM calls at runtime for deterministic pipelines — and it is in
**runtime LLM cost**, not source tokens, where the gap remains widest: AutoGen consumes
1,950–6,200 LLM tokens per execution across our benchmarks, while Aether consumes zero
for deterministic paths. We discuss the design trade-offs, the workload classes where
each approach is appropriate, and the conditions under which Aether's source token
advantage effectively disappears.

---

## 1. Introduction

The rapid adoption of agent orchestration frameworks has produced three dominant paradigms:

- **LangGraph** (LangChain Inc., 2024; benchmarked against v0.2.x API patterns) —
  state-machine graph of Python functions, compile-then-invoke execution model,
  deterministic for pure-computation pipelines.
  Primary structural advantage: explicit conditional edge routing.

- **AutoGen** (Microsoft Research, 2023; benchmarked against v0.2.x API patterns) —
  multi-agent conversation loop where agents communicate via natural-language messages,
  typically backed by LLM inference per turn.
  Primary structural advantage: natural fit for conversational and human-in-the-loop tasks.

- **Aether** (this work, 2026) — structured intermediate representation (IR) for AI-to-AI
  communication; pipelines expressed as typed, safety-annotated action graphs, executed
  deterministically by a Rust kernel with Python/JS subprocess execution.
  Primary structural advantage: minimal representation cost, compile-time safety model,
  structural self-healing.

The central question this paper addresses is: **what is the minimum representation cost
to express a well-defined agent pipeline?** Representation cost matters because:

1. Pipelines sent to LLMs (for generation, explanation, or repair) consume context tokens.
2. System prompts and agent configs that express orchestration logic are tokens that cannot
   be used for task-relevant information.
3. Ambiguous natural-language representations produce non-deterministic results.

---

## 2. Benchmark Design

### 2.1 Reference Task

The primary benchmark pipeline is `examples/demo.ae` — a canonical 3-node data transformation:

| Node | Intent                            | Safety | Depends On |
|------|-----------------------------------|--------|------------|
| 0x1A | Generate list of sample users     | pure   | —          |
| 0x2B | Filter users to adults (age >= 18)| pure   | 0x1A       |
| 0x3C | Generate summary report           | pure   | 0x2B       |

This task was chosen because it is pure computation with explicit data flow and is
representative of the dominant agentic pipeline shape: fetch-filter-summarize.

### 2.2 Implementations

| File                               | Framework    | Status             |
|------------------------------------|--------------|---------------------|
| `examples/demo.ae`                 | Aether       | Runnable, tested    |
| `benchmark/langchain_equiv.py`     | LangGraph    | Correct (requires install) |
| `benchmark/autogen_equiv.py`       | AutoGen      | Correct (requires install) |
| `examples/bench2_branch.ae`        | Aether       | Runnable, tested    |
| `benchmark/bench2_langgraph.py`    | LangGraph    | Correct (requires install) |
| `benchmark/bench2_autogen.py`      | AutoGen      | Correct (requires install) |
| `examples/bench3_classify.ae`      | Aether       | Runnable (requires ANTHROPIC_API_KEY) |
| `benchmark/bench3_langgraph.py`    | LangGraph    | Correct (requires install) |
| `benchmark/bench3_autogen.py`      | AutoGen      | Correct (requires install) |
| `examples/bench4_recovery.ae`      | Aether       | Runnable (requires ANTHROPIC_API_KEY for RETRY) |
| `benchmark/bench4_langgraph.py`    | LangGraph    | Correct (requires install) |
| `benchmark/bench4_autogen.py`      | AutoGen      | Correct (requires install) |

### 2.3 Metrics

| Metric           | Definition                                                              |
|------------------|-------------------------------------------------------------------------|
| **Source tokens**| `tiktoken cl100k_base` token count of the source file                  |
| **Total lines**  | Raw line count including blanks and comments                            |
| **Logical LOC**  | Non-blank, non-comment source lines                                     |
| **Boilerplate**  | Framework setup lines: imports, state schemas, wiring, config dicts    |
| **Logic**        | Lines containing actual application computation                         |
| **LLM/run**      | Estimated LLM tokens consumed per pipeline execution at runtime         |

Token counts use `tiktoken cl100k_base`, which closely approximates both GPT-4 and
Claude tokenization for code. All numbers are reproducible via `python benchmark/token_count.py`.

---

## 3. Results — Benchmark 1: Pure 3-Node Pipeline

### 3.1 Source Representation Cost

```
+------------------------------+---------+---------+---------+---------+---------+---------+------------+
| Framework                    | Lines   | LOC     | Boiler  | Logic   | Chars   | Tokens  | LLM/run    |
+------------------------------+---------+---------+---------+---------+---------+---------+------------+
| Aether (.ae)                 | 70      | 58      | 27      | 31      | 1,192   | 415     | 0          |
| LangGraph (Python)           | 88      | 51      | 23      | 28      | 2,942   | 715     | 0          |
| AutoGen (Python)             | 131     | 82      | 20      | 62      | 5,255   | 1,225   | 1,950 est. |
+------------------------------+---------+---------+---------+---------+---------+---------+------------+

Overhead vs. Aether:
  LangGraph (Python)    tokens +72%    LOC -12%
  AutoGen (Python)      tokens +195%   LOC +41%
```

**Aether is the most token-efficient representation.** Despite having fewer raw lines than
LangGraph (70 vs 88), the token efficiency advantage (72%) is proportionally larger than
the line count difference, because Aether's syntax is dense and avoids Python's verbose
scaffolding (TypedDict definitions, `{**state, ...}` merges, graph wiring calls).

Note on overhead framing: "AutoGen uses 195% more tokens than Aether" and "Aether uses
66% fewer tokens than AutoGen" are both mathematically valid but use different bases.
The body of this paper uses the overhead percentage (relative to Aether) consistently.

### 3.2 Where Tokens Go

**Aether token breakdown (415 total):**
- `§ROOT` / `§ACT` structural delimiters: ~40 tokens (10%)
- `::META` intent + safety declarations: ~60 tokens (14%)
- `::IN` / `::OUT` / `::EXEC` blocks: ~80 tokens (19%)
- Actual Python logic: ~235 tokens (57%)

**LangGraph token breakdown (715 total):**
- Imports and TypedDict schema: ~120 tokens (17%)
- Node function signatures and docstrings: ~100 tokens (14%)
- `{**state, ...}` state merge overhead: ~80 tokens (11%)
- `build_graph()` wiring (add_node × 3 + add_edge × 3 + set_entry_point + compile): ~85 tokens (12%)
- Entrypoint and invocation: ~45 tokens (6%)
- Actual Python logic: ~285 tokens (40%)

**AutoGen token breakdown (1,225 source + ~1,950 runtime = ~3,175 total per run):**
- LLM config dict: ~60 tokens (source, 5%)
- Agent definitions (4 agents × ~110 tokens per system prompt): ~440 tokens (source, 36%)
- GroupChat / GroupChatManager setup: ~90 tokens (source, 7%)
- Orchestration message: ~80 tokens (source, 7%)
- Code execution and documentation comments: ~555 tokens (source, 45%)
- Per-execution LLM inference: ~1,950 tokens (runtime, not in source)

### 3.3 Runtime LLM Cost

For a single pipeline execution:

| Framework  | Source tokens | Runtime LLM tokens | Total per run |
|------------|:-------------:|:------------------:|:-------------:|
| Aether     | 415           | 0                  | **415**       |
| LangGraph  | 715           | 0                  | **715**       |
| AutoGen    | 1,225         | ~1,950             | **~3,175**    |

AutoGen's runtime cost is driven by:
- 3 agent system prompts sent on every invocation: ~750 tokens
- 6 conversation turns × ~150 tokens each: ~900 tokens
- Code generation by LLM × 3 steps × ~100 tokens: ~300 tokens

*This estimate is conservative — it assumes `gpt-4o-mini`, no retries, and no multi-turn
clarification. In practice, AutoGen pipelines often consume 3–5× more tokens than this
floor estimate.*

---

## 4. Safety Model Comparison

This is the sharpest structural difference between the three frameworks.

### 4.1 Aether

Safety is **declared, compile-time, and enforced by the kernel**:

```aether
§ACT 0x2B {
  ::META {
    _intent: "Filter users to adults only",
    _safety: "net_egress"          // ← structural declaration
  }
  ...
}
```

Safety levels (L0–L4) are checked before execution begins. A node declaring
`net_egress` but running in an L2 environment is **blocked before its code runs**.
The ASL registry cross-checks declared safety against the canonical intent definition
and emits warnings for mismatches. Developers cannot accidentally omit safety
declarations — the field is required in `::META`.

```
// At runtime, attempting to exceed the auto-approve level:
[BLOCKED] Node 0x2B — net_egress > L2 (state_mod). Manual approval required.
```

### 4.2 LangGraph

LangGraph has **no built-in safety model**. There is no mechanism to declare
capability requirements, no kernel-level gate, and no audit trail of what each
node is permitted to do.

Safety must be implemented by the developer:

```python
def generate_report(state: PipelineState) -> PipelineState:
    # No safety declaration. This function can do anything.
    # Import requests, write files, call subprocess — no gate stops it.
    ...
```

LangChain provides optional callback hooks (`on_tool_start`, `on_agent_action`)
and some LLM providers offer content filtering — but these are post-hoc or
LLM-mediated, not structural pre-execution gates on the orchestration layer.

### 4.3 AutoGen

AutoGen's safety model is **LLM-dependent and prompt-based**:

- Each agent has a `system_message` that can instruct it to stay within bounds.
- The orchestrator can monitor output and terminate on conditions.
- **There is no enforcement.** A malformed LLM response, adversarial input, or
  model version change can cause an agent to deviate and execute unintended code
  with full system privileges.
- `code_execution_config={"use_docker": False}` runs generated code in the host
  process with no capability isolation by default.

| Dimension               | Aether             | LangGraph          | AutoGen            |
|-------------------------|--------------------|--------------------|--------------------|
| Safety declaration      | Structural, required | None             | Prompt-based, optional |
| Pre-execution gate      | Yes (kernel-level) | No                | No                 |
| Capability enforcement  | L0–L4 levels       | Developer-defined  | LLM judgment       |
| Audit trail             | NodeTrace + ledger | None built-in      | Conversation log   |
| ASL registry check      | Yes (32 intents)   | N/A                | N/A                |

---

## 5. Determinism Analysis

### 5.1 Aether

**Fully deterministic.** Given the same program and input state, Aether produces
identical output on every run:

- Python code executes in a subprocess with a deterministic entry point.
- No LLM is invoked during execution (only optionally during RETRY healing).
- Dependency order is computed once via topological sort; concurrent nodes run in
  deterministic topological order.
- The state ledger is append-only and type-validated — no silent overwrites.

### 5.2 LangGraph

**Deterministic for pure-computation pipelines** — as long as no LLM-backed nodes
are used. The `StateGraph` executes nodes in declared edge order; Python functions
are deterministic by default.

Determinism breaks when LangGraph is used with `ChatOpenAI`, tool-calling agents,
or conditional edges backed by LLM routing — which is the common production usage.
For the pure-computation benchmark task, LangGraph is equivalently deterministic
to Aether.

### 5.3 AutoGen

**Non-deterministic by design.** AutoGen generates code via LLM inference:

- `temperature: 0` reduces but does not eliminate variance.
- LLM output format is unstable across model versions and API changes.
- String-matched termination conditions (`"PIPELINE_COMPLETE" in message`) are
  fragile — a model that adds punctuation or rephrases can break the pipeline.
- Conversation history grows with turns, causing the same prompt to produce
  different outputs as context shifts.

---

## 6. Self-Healing Comparison

### 6.1 Aether RETRY

Self-healing is **structural and automatic**:

```aether
::VALIDATE {
  ASSERT $0xRESULT["count"] == 5 OR RETRY(3)
}
```

When validation fails:
1. The kernel snapshots the ledger (clean state guaranteed on retry).
2. `heal_node_code()` calls Claude Haiku with: the failing code, the failed
   assertion, and the actual output.
3. The LLM returns corrected code, which is substituted and re-executed.
4. Up to N retries; heal_log records each attempt for audit.
5. Degrades gracefully if `ANTHROPIC_API_KEY` is absent — no panic, just
   records "healing unavailable" and proceeds to VALIDATION_FAILED.

### 6.2 LangGraph

No built-in self-healing. Developers must implement retry manually:

```python
def filter_adults(state):
    for attempt in range(3):
        try:
            result = ...
            assert result["count"] >= 0
            return {**state, "adults": result}
        except AssertionError:
            pass  # Retry — but how do we "heal" the logic?
    raise RuntimeError("Validation failed after 3 attempts")
    # No LLM repair. No audit log. No ledger snapshot.
```

LangGraph offers `interrupt_before`/`interrupt_after` for human-in-the-loop
correction — a different (manual) healing model that requires UI integration.

### 6.3 AutoGen

AutoGen's conversation model can support iterative correction:
the orchestrator can send a follow-up message to an agent that failed.
However:
- Each correction attempt consumes LLM tokens (typically 500–1,000 tokens per retry).
- There is no structured assertion language — failure conditions are natural-language.
- Correction quality depends on model capability and prompt engineering.
- The ledger equivalent (conversation history) accumulates all failed attempts,
  potentially causing context degradation on subsequent nodes.

---

## 7. Latency and Startup Cost

### 7.1 Pipeline Execution Latency

Aether introduces two sources of latency that pure-Python frameworks do not:

**Rust parser startup (~1–3ms):** The `aether` binary parses the `.ae` source, builds
the AST, resolves dependencies, and validates against the ASL registry before executing
the first node. Measured on a 2024 M2 MacBook Pro with `examples/demo.ae` (70 lines):

```
Parser + registry load:  ~1.2ms  (p50), ~2.1ms (p99)
Dependency resolution:   ~0.1ms  (topological sort, 3 nodes)
Total pre-execution:     ~1.3ms
```

*These measurements are from developer testing; independent reproduction recommended.*

For comparison, a `python -c "from langgraph.graph import StateGraph"` import takes
~180ms cold start on the same machine due to Python interpreter and package import
overhead. The Rust parser startup is negligible by comparison.

**Subprocess execution overhead (~5–15ms per node):** Each `::EXEC<PYTHON>` block
spawns a new Python subprocess. This is intentional — it provides process isolation
and prevents state leakage between nodes — but it adds latency relative to LangGraph's
in-process function calls. For a 3-node pipeline:

```
LangGraph (in-process, 3 nodes):    ~0.1ms execution overhead
Aether (subprocess, 3 nodes):       ~30ms execution overhead (3 × ~10ms)
```

**The subprocess overhead is Aether's primary latency cost**, not the Rust parser.
For pipelines where node execution is compute-heavy (LLM calls, data processing),
this overhead is negligible. For pipelines of very fast pure-computation nodes
called in a tight loop, it is material.

### 7.2 When Latency Matters

| Scenario                              | Latency concern? | Notes                                    |
|---------------------------------------|:----------------:|------------------------------------------|
| LLM call nodes (>100ms each)          | No               | Subprocess overhead (<1%) is negligible  |
| Data processing (>10ms each)          | No               | Acceptable overhead                      |
| Sub-millisecond pure compute, <5 nodes| Yes              | LangGraph in-process is faster           |
| Real-time streaming pipelines         | Yes              | Aether is not designed for this use case |
| Batch processing, long pipelines      | No               | Startup cost amortized over execution    |
| AI-to-AI pipeline generation/transmission | No           | Token budget dominates                   |

**Conclusion:** Latency is only a meaningful concern for pipelines of trivial pure-computation
nodes called at high frequency. This is not the primary use case for orchestration frameworks
in general, and Aether in particular.

---

## 8. Scalability Analysis

### 8.1 Token Cost Scaling — Measured Data

Rather than projections from structural overhead estimates, we now have actual tiktoken
counts from all four benchmarks:

```
+--------------------------------------+--------+---------+---------+-----------+
| Benchmark                            | Aether | LangGr. | AutoGen | Aether    |
|                                      | tokens | tokens  | source  | adv. (LG) |
+--------------------------------------+--------+---------+---------+-----------+
| B1: Pure 3-node pipeline             |   415  |   715   |  1,225  |  42%      |
| B2: Conditional branching (5 nodes)  |   812  |   885   |  1,216  |   8%      |
| B3: Mixed LLM + deterministic (4 n.) |   703  |   878   |    874  |  20%      |
| B4: Long pipeline + recovery (8 n.)  | 1,235  | 1,324   |  1,246  |   7%      |
+--------------------------------------+--------+---------+---------+-----------+
(actual tiktoken cl100k_base counts — python benchmark/token_count.py)
```

### 8.2 The Convergence Effect

The measured data contradicts a naive scaling model. Per-node token costs are:

```
Aether:    B1: 138/node  →  B4: 154/node  (increases with complexity)
LangGraph: B1: 238/node  →  B4: 166/node  (decreases as fixed costs amortize)
AutoGen:   B1: 408/node  →  B4: 156/node  (decreases sharply at scale)
```

**LangGraph and AutoGen have high fixed costs (imports, TypedDict schema, config dicts,
GroupChat setup) that amortize across nodes.** Aether's per-node cost is lower initially
but stays constant — every `§ACT` block carries the full structural overhead regardless
of pipeline length. At 8 nodes, LangGraph's amortized per-node cost (166 tokens) is
within 8% of Aether's (154 tokens).

Extrapolating from these rates, **the three frameworks converge toward parity around
15–20 nodes** for source token count alone. Beyond that, the difference is negligible.
Aether's structural advantages — safety model, self-healing, audit trail — become the
primary differentiators, not token efficiency.

### 8.3 Scalability Limitations Not Captured by Token Count

Token count is not the only scalability dimension:

- **Aether's topological scheduler** resolves dependencies in O(V + E) time. For
  50-node pipelines with complex dependency graphs, this adds ~2–5ms — negligible.
- **Subprocess spawning** at 50 nodes contributes ~500ms of execution overhead if
  nodes are run serially. Parallel nodes are tokio-spawned; actual overhead depends
  on concurrency degree.
- **LangGraph's in-process execution** scales better for very large pure-computation
  pipelines where latency is critical, because there is no per-node process isolation cost.
- **AutoGen's context window** accumulates all conversation history. At 50 agents,
  the conversation context itself can exceed 32K tokens, causing context truncation
  or degraded output quality — a qualitative failure mode that token counts do not capture.

---

## 9. Benchmark 2: Conditional Branching Pipeline (5 Nodes)

### 9.1 Task Description

```
fetch → validate → [is_valid=true:  enrich → store  ]
                   [is_valid=false: flag_and_log     ]
```

This task was chosen to test LangGraph's primary differentiating feature:
`add_conditional_edges`. If Aether's advantage holds here, it holds broadly.
If it narrows, that is an honest finding.

| Node | Intent               | Safety     | Depends On    |
|------|----------------------|------------|---------------|
| 0x1A | Fetch sensor records | net_egress | —             |
| 0x2B | Validate all > 0     | pure       | 0x1A          |
| 0x3C | Enrich (valid path)  | pure       | 0x2B          |
| 0x4D | Store (valid path)   | fs_write   | 0x3C          |
| 0x5E | Flag and log (invalid path) | pure | 0x2B        |

### 9.2 Aether Implementation

Aether has no native conditional-edge syntax equivalent to LangGraph's
`add_conditional_edges`. Conditional branching is represented by embedding
branch guards at the start of each branch's `::EXEC` block, returning a sentinel
value if the branch is not taken. This is a **current limitation** of Aether's
grammar, and one where LangGraph's routing model is ergonomically superior.

```aether
§ROOT 0xFF_BRANCH {
  ...
  §ACT 0x3C {
    ::META { _intent: "std.proc.transform", _safety: "pure" }
    ::IN { $0xSRC: Ref($0xVALIDATED) }
    ::EXEC<PYTHON> {
      data = $0xSRC
      if not data["is_valid"]:
        return {"skipped": True}                // branch guard — not taken
      enriched = [dict(r, score=round(r["value"] * 1.1, 2)) for r in data["records"]]
      return {"enriched": enriched, "count": len(enriched)}
    }
    ::OUT { $0xENRICHED: Type<JSON> }
  }
  ...
}
```

Full source: `examples/bench2_branch.ae`

### 9.3 LangGraph Implementation (key excerpt)

LangGraph's routing is declarative and structurally cleaner for this use case:

```python
def route_on_validity(state: BranchState) -> Literal["enrich", "flag_and_log"]:
    return "enrich" if state["validated"]["is_valid"] else "flag_and_log"

graph.add_conditional_edges(
    "validate_records",
    route_on_validity,
    {"enrich": "enrich", "flag_and_log": "flag_and_log"},
)
```

Full source: `benchmark/bench2_langgraph.py`

### 9.4 AutoGen Implementation

AutoGen requires a custom `speaker_selection_method` function that inspects the last
message for a routing signal string (`ROUTE:VALID` / `ROUTE:INVALID`) embedded in the
ValidateAgent's output. If the LLM omits or rephrases the signal, routing fails silently.
Full source: `benchmark/bench2_autogen.py`

### 9.5 Benchmark 2 Results

```
+------------------------------+---------+---------+------------+
| Framework                    | Tokens  | LLM/run | vs. Aether |
+------------------------------+---------+---------+------------+
| Aether (.ae)                 |   812   |   0     | baseline   |
| LangGraph (Python)           |   885   |   0     | +9%        |
| AutoGen (Python)             | 1,216   | ~2,680  | +50%       |
+------------------------------+---------+---------+------------+
(actual tiktoken cl100k_base counts)
```

**Analysis.** Aether's source advantage collapses from 42% (Benchmark 1) to **only 9%**
over LangGraph on this task — a near-parity result. The narrowing has two causes:
first, LangGraph's `add_conditional_edges` routing pattern is compact (~35 tokens for
the routing function and edge map), while Aether's embedded branch-guard pattern adds
conditional checks inside each affected `::EXEC` block. Second, Aether's per-node
structural overhead (`§ACT`, `::META`, `::IN`, `::OUT` delimiters) does not amortize
— it is paid in full for every node regardless of pipeline size.

**LangGraph's routing model is ergonomically superior for this task.** The branch
condition is visible at the graph wiring level (`route_on_validity`), not buried in node
bodies. A 9% token advantage for Aether is within noise for most practical purposes.
On conditional branching pipelines, the safety model and audit trail — not token
efficiency — are Aether's remaining differentiators.

---

## 10. Benchmark 3: Mixed LLM + Deterministic Pipeline (4 Nodes)

### 10.1 Task Description

```
ingest_text → LLM_classify → extract_entities → format_output
```

This task requires a legitimate LLM call (text classification). It is the fairest
comparison against AutoGen (closer to its design intent) and tests whether Aether's
token efficiency advantage persists when `::EXEC` blocks must contain LLM API code.

| Node | Intent              | Safety     | Depends On    |
|------|---------------------|------------|---------------|
| 0x1A | Read raw text       | fs_read    | —             |
| 0x2B | LLM classification  | net_egress | 0x1A          |
| 0x3C | Extract entities    | pure       | 0x1A, 0x2B    |
| 0x4D | Format output       | pure       | 0x3C          |

### 10.2 Aether Implementation

The LLM call is explicit in the `::EXEC<PYTHON>` block. The `_safety: "net_egress"`
declaration is required and structurally enforced:

```aether
§ACT 0x2B {
  ::META {
    _intent: "std.ml.classify",
    _safety: "net_egress"
  }
  ::IN { $0xTEXT: Ref($0xRAW) }
  ::EXEC<PYTHON> {
    import os, anthropic
    client = anthropic.Anthropic(api_key=os.environ["ANTHROPIC_API_KEY"])
    text = $0xTEXT["content"]
    resp = client.messages.create(
      model="claude-haiku-4-5-20251001",
      max_tokens=64,
      messages=[{"role": "user", "content": f"Classify as FINANCIAL, TECHNICAL, "
                 f"or OPERATIONAL. Text: {text}\nReply with only the category name."}]
    )
    return {"category": resp.content[0].text.strip(), "confidence": 1.0}
  }
  ::OUT { $0xCLASS: Type<JSON> }
  ::VALIDATE {
    ASSERT $0xCLASS["category"] in ["FINANCIAL", "TECHNICAL", "OPERATIONAL"] OR RETRY(2)
  }
}
```

Full source: `examples/bench3_classify.ae`

### 10.3 LangGraph Implementation

LangGraph with `langchain_anthropic` is more compact than Aether for the LLM node,
because the LangChain SDK abstracts away the full API call:

```python
llm = ChatAnthropic(model="claude-haiku-4-5-20251001", max_tokens=64, temperature=0)

def classify_text(state: ClassifyState) -> ClassifyState:
    response = llm.invoke([HumanMessage(content=f"Classify as FINANCIAL, TECHNICAL, "
                           f"or OPERATIONAL. Text: {state['raw']['content']}")])
    return {**state, "classification": {"category": response.content.strip(), "confidence": 1.0}}
```

Full source: `benchmark/bench3_langgraph.py`

### 10.4 Benchmark 3 Results

```
+------------------------------+---------+-------------------+------------+
| Framework                    | Source  | Runtime LLM       | vs. Aether |
+------------------------------+---------+-------------------+------------+
| Aether (.ae)                 |   703   | ~150 (1 LLM call) | baseline   |
| LangGraph (Python)           |   878   | ~150 (1 LLM call) | +25%       |
| AutoGen (Python)             |   874   | ~2,360            | +24%       |
+------------------------------+---------+-------------------+------------+
(actual tiktoken cl100k_base counts)
```

**Analysis.** Three notable findings here:

1. **AutoGen source tokens (874) are essentially equal to LangGraph (878).** When the
   pipeline is genuinely LLM-mediated, AutoGen's conversation model is no more verbose
   than LangGraph's explicit graph wiring. The agent system messages are compact for a
   natural-language task.

2. **Aether's advantage holds at ~25% over LangGraph** because the `::EXEC` block must
   contain the full Anthropic SDK call (imports, client init, `messages.create`), while
   LangGraph's `langchain_anthropic` provides a one-line `llm.invoke()` abstraction.
   Aether's structural overhead is partially offset by not needing the TypedDict schema.

3. **Runtime LLM cost is the real differentiator.** Aether and LangGraph both make exactly
   one classification API call (~150 tokens). AutoGen's conversation loop consumes ~2,360
   tokens to accomplish the same task — 15× more runtime LLM cost. For mixed-LLM pipelines,
   AutoGen's disadvantage is in runtime cost, not source cost.

---

## 11. Benchmark 4: Long Pipeline with Error Recovery (8 Nodes)

### 11.1 Task Description

An 8-node data pipeline with a deliberate bug injected at node 5 (`transform`) to trigger
Aether's ASSERT/RETRY self-healing. Node 5 uses the wrong field for checksum computation
(`score` instead of `id`), producing checksum 50 when the assertion requires 55.

```
ingest → normalize → deduplicate → enrich → transform* → aggregate → format → write
                                              (* bug injected here — RETRY triggers)
```

| Node | Intent             | Safety   | Depends On |
|------|--------------------|----------|------------|
| 0x1A | Ingest raw rows    | fs_read  | —          |
| 0x2B | Normalize values   | pure     | 0x1A       |
| 0x3C | Deduplicate by id  | pure     | 0x2B       |
| 0x4D | Enrich with score  | pure     | 0x3C       |
| 0x5E | Compute checksum*  | pure     | 0x4D       |
| 0x6F | Aggregate scores   | pure     | 0x5E       |
| 0x7A | Format report      | pure     | 0x6F       |
| 0x8B | Write output       | fs_write | 0x7A       |

### 11.2 Aether Self-Healing in Practice

The VALIDATE assertion on node 0x5E:

```aether
::VALIDATE {
  ASSERT $0xTRANSFORMED["checksum"] == 55 OR RETRY(3)
}
```

On first execution: buggy code produces checksum = 550 % 100 = 50. Assertion fails.
The kernel:
1. Snapshots the ledger state before the retry.
2. Calls `heal_node_code()` with the failing code, assertion, and actual output.
3. Claude Haiku identifies the bug (wrong field) and returns corrected code.
4. Re-executes node 0x5E with corrected code → checksum = 55. Assertion passes.
5. Records both attempts in `heal_log` for audit.

Full source: `examples/bench4_recovery.ae`

### 11.3 LangGraph — Manual Retry Without LLM Repair

```python
def transform_with_retry(state, max_retries=3):
    for attempt in range(max_retries):
        try:
            if attempt == 0:
                checksum = sum(int(r["score"]) for r in data["rows"]) % 100  # bug
            else:
                checksum = sum(r["id"] for r in data["rows"]) % 100          # fix hard-coded
            assert checksum == 55
            return {**state, "transformed": {...}}
        except AssertionError:
            pass  # No LLM repair. No audit log. Fix must be pre-known.
    raise RuntimeError("failed after 3 retries")
```

The critical distinction: LangGraph's "retry" is a loop that re-runs the same code
(or a hard-coded correction). There is no mechanism to compute the correct fix at
runtime — the developer must know and hard-code it. Aether's RETRY uses LLM inference
to derive the correction from the failure evidence.

Full source: `benchmark/bench4_langgraph.py`

### 11.4 Benchmark 4 Results

```
+------------------------------+---------+------------------------+------------+
| Framework                    | Source  | Runtime LLM (w/ retry) | vs. Aether |
+------------------------------+---------+------------------------+------------+
| Aether (.ae)                 | 1,235   | ~500 (1 heal call)     | baseline   |
| LangGraph (Python)           | 1,324   | 0                      | +7%        |
| AutoGen (Python)             | 1,246   | ~6,200                 | +1%        |
+------------------------------+---------+------------------------+------------+
(actual tiktoken cl100k_base counts)

Aether NodeTrace for node 0x5E (heal attempted):
  [VALIDATE] ASSERT $0xTRANSFORMED["checksum"] == 55 → FAIL (got 50)
  [HEAL] attempt 1/3 — calling claude-haiku-4-5-20251001
  [HEAL] fix applied: sum(r["id"] ...) instead of sum(int(r["score"]) ...)
  [VALIDATE] ASSERT $0xTRANSFORMED["checksum"] == 55 → PASS
  heal_log: ["attempt 1: replaced score aggregation with id aggregation"]
```

**Analysis.** This is the most important result in the paper.

At 8 nodes, **all three frameworks converge to near-parity in source tokens**: Aether
1,235, AutoGen 1,246 (+1%), LangGraph 1,324 (+7%). The per-node amortization effect
described in §8.2 is fully visible here: LangGraph's and AutoGen's fixed costs (imports,
TypedDict schema, config dicts, GroupChat setup) are spread across 8 nodes, while Aether's
`§ACT` / `::META` / `::OUT` overhead is paid per-node without amortization.

**Source token efficiency is not Aether's differentiator at scale.** What differentiates
the frameworks at 8 nodes is:

1. **Self-healing.** Aether's RETRY computes the fix at runtime using failure evidence
   (§6.1). LangGraph requires the developer to pre-code the correction. AutoGen's
   conversational retry costs ~600 LLM tokens per attempt.
2. **Runtime LLM cost.** AutoGen's source tokens are within 1% of Aether, but its
   runtime cost (~6,200 tokens per execution) is 5× Aether's total (source + heal call).
3. **Audit trail.** The NodeTrace + heal_log above is produced automatically. Neither
   LangGraph nor AutoGen generates a structured failure record without custom code.

---

## 12. When NOT to Use Aether

Aether's architecture incurs costs that make other frameworks more appropriate for specific
workload classes. This section is direct.

### 12.1 Exploratory and Prototype Pipelines

**Use LangGraph or AutoGen instead.** Aether requires writing a `.ae` source file,
running the Rust binary, and debugging output via NodeTrace JSON. For a one-off data
transformation or exploratory analysis, a Python script or a LangGraph notebook is faster
to iterate on. Aether's structured format has no ergonomic advantage when the pipeline
shape will be discarded in an hour.

### 12.2 Conversational, Human-in-the-Loop Tasks

**Use AutoGen instead.** Aether's deterministic execution model has no concept of
multi-turn conversation, user interjection, or LLM-mediated routing. A task like
"help me iteratively refine this document" or "ask the user for clarification if the
input is ambiguous" is fundamentally outside Aether's design. Forcing it into a `.ae`
file would require one node per conversation turn, with no dynamic turn count.

### 12.3 Pipelines with Heavy LLM Usage in Every Node

**Advantage narrows; evaluate case by case.** Benchmark 3 shows that when LLM calls
are the dominant operation, Aether's source token advantage over LangGraph drops to 27%.
If the pipeline is entirely LLM-call nodes and the team already uses LangChain tooling,
the migration cost may not be justified by the token savings.

### 12.4 Tight Latency Loops (Sub-10ms Turnaround)

**Use LangGraph (in-process) instead.** Aether's subprocess execution model adds
~10ms overhead per node. A pipeline called thousands of times per second with
sub-millisecond node logic would accumulate meaningful overhead. Aether is not designed
for high-frequency execution.

### 12.5 Teams Without a Rust Toolchain

**This is Aether's most significant adoption barrier.** Aether requires a Rust
toolchain (`rustup` + `cargo build`) to compile from source. For the target audience —
Python-centric AI/ML teams — this is a hard blocker, not a soft friction:

- Installing `rustup` requires admin privileges on many corporate machines.
- Compiling from source takes 30–90 seconds on a clean build (vs. 2 seconds for `pip install`).
- Rust compiler errors during build are opaque to Python developers.
- CI/CD pipelines must add a Rust toolchain step.
- Docker images must include the Rust build chain or use a multi-stage build.

LangGraph and AutoGen install with `pip install langgraph` / `pip install pyautogen`.
As of v0.3.1, Aether provides pre-built binaries via GitHub Releases and a
`pip install aether-kernel` package with bundled native binaries for
Windows (x86_64), Linux (x86_64), and macOS (x86_64 + ARM64). This
eliminates the Rust toolchain requirement for end users, though the
build-from-source path remains available for unsupported platforms.

### 12.6 Very Small Pipelines (1–2 Nodes)

**Overhead-to-logic ratio inverts.** A single-node Aether pipeline (`§ROOT`, `§ACT`,
`::META`, `::EXEC`, `::OUT`) costs ~120 tokens of structure around whatever logic it
contains. A single Python function costs ~20 tokens of structure. For very small pipelines,
Aether's structural overhead is proportionally larger, not smaller.

---

## 13. Related Work

### 13.1 Apache Airflow

Airflow is a workflow orchestration platform for data engineering pipelines. Like Aether,
it uses a DAG representation and dependency-based scheduling. **Key differences:** Airflow
is designed for scheduled batch jobs on infrastructure (S3, databases, Kubernetes), not
AI agent communication. It has no concept of LLM-generated pipelines, token cost, or
self-healing via LLM repair. Its operator definitions are verbose Python classes (~200–400
tokens per operator including imports and config). Airflow's safety model is infrastructure-
level (RBAC, connections), not node-level capability declaration.

### 13.2 Prefect

Prefect provides Python-native flow/task orchestration with built-in retries, caching,
and observability. Its `@flow` / `@task` decorator model is ergonomically similar to
LangGraph. **Key differences:** Prefect's retry model (`retries=3, retry_delay_seconds=1`)
is structural at the task level, making it closer to Aether's RETRY primitive than
LangGraph's manual loops. However, Prefect's retries re-execute the same code — there
is no LLM-assisted repair. Prefect's observability (run history, state persistence) is
more mature than Aether's NodeTrace for production deployments.

### 13.3 Temporal

Temporal is a durable workflow engine with at-least-once execution guarantees,
workflow replay, and long-running saga support. **Key differences:** Temporal solves a
different problem: durable execution over hours/days with compensation logic for failures.
Aether's RETRY is a within-execution repair primitive; Temporal's compensation is a
cross-execution rollback primitive. For pipelines requiring durable multi-day execution
with external service calls, Temporal is the appropriate tool. Temporal's workflow
definitions (Go or Java) are significantly more verbose than Aether.

### 13.4 DSPy

DSPy (Demonstrate–Search–Predict) is a framework for programming LLM pipelines through
optimizable modules rather than hand-written prompts. **Key differences:** DSPy optimizes
the prompts themselves via compiled signatures and few-shot demonstration selection.
Aether does not optimize prompts — it assumes the node's logic is correct as written
(or uses RETRY to repair failures). DSPy's compiled signatures are compact (~50–100
tokens per module declaration), making it potentially more token-efficient than Aether
for all-LLM pipelines. For mixed or deterministic pipelines, Aether's structured IR
is more appropriate. DSPy has no safety model or audit trail primitive.

---

## 14. Summary Comparison Table

### 14.1 Cross-Benchmark Token Summary

```
+--------------------------------------+--------+---------+---------+-----------+
| Benchmark                            | Aether | LangGr. | AutoGen | Aether    |
|                                      | tokens | tokens  | source  | adv. (LG) |
+--------------------------------------+--------+---------+---------+-----------+
| B1: Pure 3-node pipeline             |   415  |   715   |  1,225  |    42%    |
| B2: Conditional branching (5 nodes)  |   812  |   885   |  1,216  |     8%    |
| B3: Mixed LLM + deterministic (4 n.) |   703  |   878   |    874  |    20%    |
| B4: Long pipeline + recovery (8 n.)  | 1,235  | 1,324   |  1,246  |     7%    |
+--------------------------------------+--------+---------+---------+-----------+
| Range of Aether advantage (vs LG):   | 7% – 42% fewer source tokens           |
| Range of Aether advantage (vs AG):   | 1% – 66% fewer source tokens           |
+--------------------------------------+--------+---------+---------+-----------+
(all counts verified via tiktoken cl100k_base — python benchmark/token_count.py)
```

**The data tells a clear story: Aether's source token advantage is large on small,
pure-computation pipelines (42%) and converges toward zero as pipelines grow in
complexity and scale (7–8% at 5–8 nodes).** The advantage is largest where it arguably
matters least (small pipelines) and smallest where it matters most (large pipelines).

Aether's enduring differentiators across all four benchmarks are not token count, but:
the structural safety model, the self-healing RETRY primitive, and the zero runtime LLM
cost for deterministic paths.

### 14.2 Property Comparison

| Property                          | Aether                     | LangGraph              | AutoGen                    |
|-----------------------------------|----------------------------|------------------------|----------------------------|
| **Source tokens (3-node)**        | **415**                    | 715 (+72%)             | 1,225 (+195%)              |
| **Source tokens (8-node)**        | 1,235                      | 1,324 (+7%)            | 1,246 (+1%)                |
| **Source token advantage range**  | 7–42% fewer                | baseline               | 1–66% more (vs. Aether)   |
| **Runtime LLM tokens/run (det.)** | **0**                      | **0**                  | ~1,950–6,200 (est. floor)  |
| **Runtime LLM tokens (LLM node)** | ~150 (task calls only)     | ~150 (task calls only) | ~2,360–6,200 (+ overhead)  |
| **Safety model**                  | Structural L0–L4           | None                   | Prompt-based               |
| **Pre-execution gate**            | Yes                        | No                     | No                         |
| **Determinism**                   | Always                     | Pure computation only  | Rarely                     |
| **Conditional branching**         | Embedded guards (limited)  | Native (clean)         | Message-based routing      |
| **Data-flow enforcement**         | Typed Ref() declarations   | Implicit (state dict)  | Message content            |
| **Output type validation**        | Type<JSON/STR/NUM>         | No                     | No                         |
| **Self-healing**                  | ASSERT / RETRY(n) + LLM    | Manual (no LLM repair) | Conversational (LLM cost)  |
| **Audit trail**                   | NodeTrace + ledger JSON    | None built-in          | Conversation log           |
| **ASL standard library**          | 32 canonical intents       | N/A                    | N/A                        |
| **Startup latency**               | ~1–3ms parse + ~10ms/node subprocess | ~180ms Python import | ~180ms Python import |
| **LLM pipeline advantage**        | Weakest (27% vs LangGraph) | Moderate               | Conversational tasks       |
| **Branching ergonomics**          | Embedded guards (limited)  | **Best** (native)      | Message-based              |
| **Primary use case**              | AI-to-AI orchestration     | Python agent graphs    | Human-AI conversation      |
| **Execution engine**              | Rust kernel + subprocesses | Python in-process      | Python + LLM generation    |
| **Installation**                  | Rust toolchain required    | pip install            | pip install                |

---

## 15. Limitations and Scope

This benchmark measures four tasks across three metric classes (token cost, structural
properties, and runtime LLM cost). We explicitly do **not** claim:

- **Aether is always token-efficient.** Benchmarks 2 and 4 show the source token
  advantage shrinks to 7–8% over LangGraph and 1% over AutoGen at scale. The advantage
  depends on pipeline size, complexity, and how much framework-specific boilerplate
  each implementation carries.
- **LangGraph is always more verbose.** For conditional branching, LangGraph's
  `add_conditional_edges` routing pattern is more compact and ergonomically cleaner
  than Aether's embedded branch-guard approach (§9). Aether does not currently have
  native conditional edge syntax.
- **The token counts are invariant.** All token counts are actual `tiktoken cl100k_base`
  measurements on the source files in this repository (`python benchmark/token_count.py`).
  However, different implementations of the same task (shorter system messages, fewer
  comments, more compact coding style) would produce different counts. The implementations
  aim for comparable comment density and idiomatic style, but reasonable alternative
  implementations could shift the numbers by 10–20%.
- **Latency measurements are authoritative.** The latency figures in §7 are from
  developer testing, not a controlled benchmarking study. They should be treated as
  order-of-magnitude guidance, not precise measurements.
- **Aether's safety claims are absolute.** Aether's L0–L4 model governs orchestration-
  level capability. It does not prevent a `pure`-tagged node from containing malicious
  code — it prevents the *framework* from granting capabilities the node didn't declare.
- **AutoGen is inferior for its designed use case.** AutoGen excels at conversational,
  multi-agent tasks requiring natural-language negotiation. All four benchmarks use
  predominantly deterministic pipelines, which understate AutoGen's strengths.
- **DSPy is irrelevant.** For all-LLM pipelines, DSPy's compiled signatures may
  achieve lower token costs than Aether. This comparison is not made here.

---

## 16. Conclusion

Across four benchmark tasks spanning pure computation, conditional branching, mixed LLM
workloads, and long pipelines with error recovery, Aether's structured IR uses 7–42%
fewer source tokens than LangGraph and 1–66% fewer source tokens than AutoGen.

**The source token advantage is real but limited.** It is widest on small, deterministic
pipelines (42%, Benchmark 1) and converges toward parity at scale: 7% over LangGraph
and 1% over AutoGen at 8 nodes (Benchmark 4). The convergence occurs because LangGraph
and AutoGen amortize their fixed costs (imports, schemas, configs) across nodes, while
Aether's `§ACT` structural overhead is paid per-node. At 15–20 nodes, the three frameworks
are projected to reach token parity. **Source token efficiency alone does not justify
adopting Aether for large pipelines.**

Three properties differentiate Aether regardless of pipeline size:

1. **Structural safety model.** `_safety` declarations are compile-time, required,
   and kernel-enforced. No equivalent exists in LangGraph or AutoGen. This gap does not
   narrow with scale.
2. **LLM-assisted self-healing with audit trail.** Aether's RETRY derives the code fix
   from failure evidence at runtime. LangGraph requires the developer to pre-code the fix;
   AutoGen burns LLM tokens on every correction attempt with no structural assertion language.
3. **Zero runtime LLM cost for deterministic pipelines.** Aether makes no LLM calls
   during execution of non-RETRY paths. AutoGen consumes 1,950–6,200 LLM tokens per
   pipeline run across our benchmarks. This is the gap that grows with scale: at 8 nodes
   AutoGen's runtime cost is 5× Aether's total (source + heal call), even though the
   source tokens are within 1% of each other.

The core insight is that **Aether's value proposition shifts as pipeline size increases**:
for small pipelines (1–5 nodes), token efficiency is the primary advantage; for large
pipelines (8+ nodes), the safety model, self-healing, and runtime LLM cost become the
dominant differentiators. The structured IR matters most when pipelines are generated by
LLMs (token budget is money), run in sensitive environments (safety must be structural),
or must be auditable and repairable under failure (NodeTrace + RETRY).

For human-authored, exploratory, conversational, or branch-heavy tasks — LangGraph and
AutoGen remain more ergonomic choices.

---

## Appendix: Reproducing Results

```bash
# Option 1: pip install (no Rust required)
pip install aether-kernel

# Option 2: Pre-built binary
curl -fsSL https://raw.githubusercontent.com/baiers/aether/main/install.sh | bash

# Option 3: Build from source (requires Rust toolchain)
git clone <repo>
cd aether
cargo build --release

# Run Benchmark 1 (reference)
aether examples/demo.ae    # or: cargo run --bin aether -- examples/demo.ae

# Run Benchmark 2 (conditional branching)
cargo run --bin aether -- examples/bench2_branch.ae

# Run Benchmark 3 (mixed LLM — requires API key)
ANTHROPIC_API_KEY=... cargo run --bin aether -- examples/bench3_classify.ae

# Run Benchmark 4 (error recovery — RETRY requires API key)
ANTHROPIC_API_KEY=... cargo run --bin aether -- examples/bench4_recovery.ae

# Token counting
pip install tiktoken
python benchmark/token_count.py
```

All source files referenced in this paper:

| File                               | Description                                        |
|------------------------------------|----------------------------------------------------|
| `examples/demo.ae`                 | Benchmark 1: Aether reference pipeline             |
| `benchmark/langchain_equiv.py`     | Benchmark 1: LangGraph equivalent                  |
| `benchmark/autogen_equiv.py`       | Benchmark 1: AutoGen equivalent                    |
| `examples/bench2_branch.ae`        | Benchmark 2: Aether conditional branching          |
| `benchmark/bench2_langgraph.py`    | Benchmark 2: LangGraph conditional branching       |
| `benchmark/bench2_autogen.py`      | Benchmark 2: AutoGen conditional branching         |
| `examples/bench3_classify.ae`      | Benchmark 3: Aether mixed LLM pipeline             |
| `benchmark/bench3_langgraph.py`    | Benchmark 3: LangGraph mixed LLM pipeline          |
| `benchmark/bench3_autogen.py`      | Benchmark 3: AutoGen mixed LLM pipeline            |
| `examples/bench4_recovery.ae`      | Benchmark 4: Aether 8-node with error recovery     |
| `benchmark/bench4_langgraph.py`    | Benchmark 4: LangGraph 8-node with manual retry    |
| `benchmark/bench4_autogen.py`      | Benchmark 4: AutoGen 8-node with conversational retry |
| `benchmark/token_count.py`         | Reproduces Table 1 numbers                         |
| `asl/registry.json`                | ASL canonical intent registry (32 entries)         |
| `src/executor.rs`                  | Aether execution engine                            |
| `src/short.rs`                     | Aether-Short preprocessor                          |
