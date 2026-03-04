"""
Aether Benchmark — Token & Complexity Counter
Usage: python benchmark/token_count.py

Measures source representation cost across four benchmark tasks:
  B1: 3-node pure pipeline (examples/demo.ae)
  B2: 5-node conditional branching (examples/bench2_branch.ae)
  B3: 4-node mixed LLM + deterministic (examples/bench3_classify.ae)
  B4: 8-node long pipeline with error recovery (examples/bench4_recovery.ae)

Metrics:
  - Total lines
  - Non-blank, non-comment lines (logical LOC)
  - Character count (raw source)
  - Token count (tiktoken cl100k_base — GPT-4 / Claude tokenizer approximation)
  - Boilerplate lines (imports, framework setup, non-logic lines)
  - Logic lines (actual computation the developer cares about)
  - Runtime LLM tokens (estimated tokens consumed during execution)
"""

import os
import re
import tiktoken

# ── Paths ──────────────────────────────────────────────────────────────────────
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

BENCHMARKS = {
    "B1: Pure 3-Node Pipeline": {
        "Aether (.ae)":       os.path.join(ROOT, "examples", "demo.ae"),
        "LangGraph (Python)": os.path.join(ROOT, "benchmark", "langchain_equiv.py"),
        "AutoGen (Python)":   os.path.join(ROOT, "benchmark", "autogen_equiv.py"),
    },
    "B2: Conditional Branching (5 Nodes)": {
        "Aether (.ae)":       os.path.join(ROOT, "examples", "bench2_branch.ae"),
        "LangGraph (Python)": os.path.join(ROOT, "benchmark", "bench2_langgraph.py"),
        "AutoGen (Python)":   os.path.join(ROOT, "benchmark", "bench2_autogen.py"),
    },
    "B3: Mixed LLM + Deterministic (4 Nodes)": {
        "Aether (.ae)":       os.path.join(ROOT, "examples", "bench3_classify.ae"),
        "LangGraph (Python)": os.path.join(ROOT, "benchmark", "bench3_langgraph.py"),
        "AutoGen (Python)":   os.path.join(ROOT, "benchmark", "bench3_autogen.py"),
    },
    "B4: Long Pipeline + Recovery (8 Nodes)": {
        "Aether (.ae)":       os.path.join(ROOT, "examples", "bench4_recovery.ae"),
        "LangGraph (Python)": os.path.join(ROOT, "benchmark", "bench4_langgraph.py"),
        "AutoGen (Python)":   os.path.join(ROOT, "benchmark", "bench4_autogen.py"),
    },
    "B5: Showcase — Fraud Monitor (6 Nodes, Safety+RETRY)": {
        "Aether (.ae)":       os.path.join(ROOT, "examples", "demo_showcase.ae"),
        "LangGraph (Python)": os.path.join(ROOT, "benchmark", "demo_langgraph.py"),
    },
}

# ── Tokenizer ─────────────────────────────────────────────────────────────────
enc = tiktoken.get_encoding("cl100k_base")

# ── Boilerplate detection ─────────────────────────────────────────────────────
BOILERPLATE_PATTERNS_PY = [
    r"^\s*(import |from )",
    r"^\s*class \w+\(TypedDict\)",
    r"^\s*\w+:\s+\w+",
    r"^\s*graph\s*=\s*StateGraph",
    r"^\s*graph\.add_(node|edge|conditional_edges)",
    r"^\s*graph\.set_entry_point",
    r"^\s*graph\.compile",
    r"^\s*return graph\.compile",
    r"^\s*app\s*=\s*build_graph",
    r"^\s*result\s*=\s*app\.invoke",
    r"^\s*\*\*state,",
    r"^\s*llm_config\s*=\s*\{",
    r"^\s*\"model\":",
    r"^\s*\"temperature\":",
    r"^\s*\w+\s*=\s*(AssistantAgent|UserProxyAgent|GroupChat|GroupChatManager|ChatAnthropic)",
    r"^\s*groupchat\s*=\s*GroupChat",
    r"^\s*manager\s*=\s*GroupChatManager",
    r"^\s*orchestrator\.initiate_chat",
    r"^\s*if __name__",
    r"^\s*\},$",
    r"^\s*\)$",
    r"^\s*llm\s*=\s*Chat",
]

BOILERPLATE_PATTERNS_AE = [
    r"^\s*§ROOT",
    r"^\s*::META",
    r"^\s*_intent:",
    r"^\s*_safety:",
    r"^\s*\}$",
]

# ── Runtime LLM estimates per file ────────────────────────────────────────────
RUNTIME_LLM = {
    # B1
    "demo.ae": 0,
    "langchain_equiv.py": 0,
    "autogen_equiv.py": 1950,
    # B2
    "bench2_branch.ae": 0,
    "bench2_langgraph.py": 0,
    "bench2_autogen.py": 2680,
    # B3
    "bench3_classify.ae": 150,
    "bench3_langgraph.py": 150,
    "bench3_autogen.py": 2360,
    # B4
    "bench4_recovery.ae": 500,
    "bench4_langgraph.py": 0,
    "bench4_autogen.py": 6200,
    # B5 showcase (Aether: 0 LLM tokens unless RETRY fires; LangGraph: 0 deterministic)
    "demo_showcase.ae": 0,
    "demo_langgraph.py": 0,
}


def count_boilerplate(lines: list[str], patterns: list[str]) -> int:
    compiled = [re.compile(p) for p in patterns]
    return sum(
        1 for line in lines
        if any(pat.match(line) for pat in compiled)
    )


def analyze(label: str, path: str) -> dict:
    with open(path, encoding="utf-8") as f:
        source = f.read()

    lines = source.splitlines()
    total_lines     = len(lines)
    non_blank_lines = [l for l in lines if l.strip()]
    logical_lines   = [
        l for l in non_blank_lines
        if not l.strip().startswith(("#", '"""', "'''", "//"))
    ]

    is_ae       = path.endswith((".ae", ".as"))
    patterns    = BOILERPLATE_PATTERNS_AE if is_ae else BOILERPLATE_PATTERNS_PY
    boilerplate = count_boilerplate(logical_lines, patterns)
    logic       = len(logical_lines) - boilerplate

    token_count = len(enc.encode(source))

    basename = os.path.basename(path)
    runtime  = RUNTIME_LLM.get(basename, 0)

    return {
        "label":              label,
        "total_lines":        total_lines,
        "logical_loc":        len(logical_lines),
        "boilerplate_loc":    boilerplate,
        "logic_loc":          logic,
        "chars":              len(source),
        "tokens":             token_count,
        "runtime_llm_tokens": runtime,
    }


def print_table(results: list[dict]) -> None:
    w = [28, 7, 7, 7, 7, 7, 7, 10]
    header = ["Framework", "Lines", "LOC", "Boiler", "Logic", "Chars", "Tokens", "LLM/run"]
    sep    = "+" + "+".join("-" * (wi + 2) for wi in w) + "+"
    fmt    = "| " + " | ".join(f"{{:<{wi}}}" for wi in w) + " |"

    print(sep)
    print(fmt.format(*header))
    print(sep)
    for r in results:
        print(fmt.format(
            r["label"],
            r["total_lines"],
            r["logical_loc"],
            r["boilerplate_loc"],
            r["logic_loc"],
            f"{r['chars']:,}",
            r["tokens"],
            r["runtime_llm_tokens"],
        ))
    print(sep)
    print()

    base = results[0]
    print("Overhead vs. Aether:")
    for r in results[1:]:
        tok_overhead = (r["tokens"] - base["tokens"]) / base["tokens"] * 100
        loc_overhead = (r["logical_loc"] - base["logical_loc"]) / base["logical_loc"] * 100
        print(f"  {r['label']:<28}  tokens {tok_overhead:+6.0f}%   LOC {loc_overhead:+6.0f}%")


if __name__ == "__main__":
    print("\nAether Benchmark — Source Representation Cost")
    print("Tokenizer: tiktoken cl100k_base (GPT-4 / Claude approximation)\n")

    for bench_name, files in BENCHMARKS.items():
        print(f"{'=' * 80}")
        print(f"  {bench_name}")
        print(f"{'=' * 80}\n")
        results = [analyze(label, path) for label, path in files.items()]
        print_table(results)
        print()

    # Cross-benchmark summary
    print(f"{'=' * 80}")
    print("  CROSS-BENCHMARK TOKEN SUMMARY")
    print(f"{'=' * 80}\n")

    summary_header = f"{'Benchmark':<45} {'Aether':>7} {'LangGr.':>8} {'AutoGen':>8} {'Adv(LG)':>8}"
    print(summary_header)
    print("-" * len(summary_header))

    for bench_name, files in BENCHMARKS.items():
        results = [analyze(label, path) for label, path in files.items()]
        ae_tok = results[0]["tokens"]
        lg_tok = results[1]["tokens"] if len(results) > 1 else 0
        ag_tok = results[2]["tokens"] if len(results) > 2 else None
        adv    = (lg_tok - ae_tok) / lg_tok * 100 if lg_tok else 0
        ag_str = f"{ag_tok:>8}" if ag_tok is not None else "       —"
        print(f"{bench_name:<45} {ae_tok:>7} {lg_tok:>8} {ag_str} {adv:>7.0f}%")

    print("\nColumn legend:")
    print("  Lines    = total source lines including blanks and comments")
    print("  LOC      = logical lines of code (non-blank, non-comment)")
    print("  Boiler   = framework setup lines (imports, wiring, config)")
    print("  Logic    = lines containing actual application computation")
    print("  Tokens   = tiktoken cl100k_base token count of source file")
    print("  LLM/run  = estimated LLM tokens consumed per execution")
    print("  Adv(LG)  = Aether's token advantage vs. LangGraph (fewer = better)")
