"""
LangGraph equivalent of Benchmark 3: Mixed LLM + Deterministic Pipeline (4 nodes)
Pipeline: ingest → LLM classify → extract → format

In this benchmark, an LLM call is legitimately required (text classification).
LangGraph with langchain_anthropic is idiomatic for this use case and requires
fewer tokens than Aether because langchain's LLM invocation syntax is more compact
than writing the full Anthropic SDK call in an EXEC block.

NOT runnable without: pip install langgraph langchain-core langchain-anthropic
"""

from typing import TypedDict
from langgraph.graph import StateGraph, END
from langchain_anthropic import ChatAnthropic
from langchain_core.messages import HumanMessage


# ── State schema ──────────────────────────────────────────────────────────────

class ClassifyState(TypedDict):
    raw:            dict
    classification: dict
    extracted:      dict
    output:         dict


# ── LLM client ────────────────────────────────────────────────────────────────
# NOTE: No safety declaration; this node can call any model with any prompt.
# There is no kernel-level gate preventing net_egress in restricted environments.

llm = ChatAnthropic(model="claude-haiku-4-5-20251001", max_tokens=64, temperature=0)


# ── Node functions ─────────────────────────────────────────────────────────────

def ingest_text(state: ClassifyState) -> ClassifyState:
    return {
        **state,
        "raw": {
            "content": "Q3 revenue increased 23% YoY driven by enterprise sales expansion.",
            "source":  "report_q3.txt",
            "length":  68,
        },
    }


def classify_text(state: ClassifyState) -> ClassifyState:
    text     = state["raw"]["content"]
    response = llm.invoke([
        HumanMessage(content=(
            f"Classify as FINANCIAL, TECHNICAL, or OPERATIONAL. "
            f"Text: {text}\nReply with only the category name."
        ))
    ])
    return {**state, "classification": {"category": response.content.strip(), "confidence": 1.0}}


def extract_entities(state: ClassifyState) -> ClassifyState:
    data    = state["raw"]
    tag     = state["classification"]
    words   = data["content"].split()
    numbers = [w for w in words if any(c.isdigit() for c in w)]
    return {
        **state,
        "extracted": {
            "category":   tag["category"],
            "entities":   numbers,
            "word_count": len(words),
            "char_count": data["length"],
        },
    }


def format_output(state: ClassifyState) -> ClassifyState:
    d = state["extracted"]
    return {
        **state,
        "output": {
            "output": f"[{d['category']}] {d['word_count']} words, entities: {d['entities']}",
            "status": "ok",
        },
    }


# ── Graph wiring ───────────────────────────────────────────────────────────────

def build_graph():
    graph = StateGraph(ClassifyState)

    graph.add_node("ingest_text",      ingest_text)
    graph.add_node("classify_text",    classify_text)
    graph.add_node("extract_entities", extract_entities)
    graph.add_node("format_output",    format_output)

    graph.set_entry_point("ingest_text")
    graph.add_edge("ingest_text",      "classify_text")
    graph.add_edge("classify_text",    "extract_entities")
    graph.add_edge("extract_entities", "format_output")
    graph.add_edge("format_output",    END)

    return graph.compile()


# ── Entrypoint ─────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    app    = build_graph()
    result = app.invoke({"raw": {}, "classification": {}, "extracted": {}, "output": {}})
    print(result["output"])
    # Runtime LLM cost: 1 classification call ~= 150 tokens (same task as Aether)
