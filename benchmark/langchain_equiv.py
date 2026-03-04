"""
LangGraph equivalent of examples/demo.ae
Pipeline: generate_users → filter_adults → summarize_report

This file exists for benchmarking purposes only.
It is NOT runnable without `pip install langgraph langchain-core`.

Reference pipeline: Aether demo.ae (3 nodes, pure computation, no external I/O)
"""

from typing import TypedDict
from langgraph.graph import StateGraph, END


# ── State schema ──────────────────────────────────────────────────────────────
# LangGraph requires an explicit shared-state container passed through all nodes.
# Every field that any node reads or writes must be declared here up front.

class PipelineState(TypedDict):
    users:   list[dict]
    adults:  dict
    report:  dict


# ── Node functions ─────────────────────────────────────────────────────────────
# Each node receives the full state dict and must return a (partial) update.
# There is no built-in safety declaration; safety is the developer's responsibility.

def generate_users(state: PipelineState) -> PipelineState:
    """Intent: Generate a list of sample users  |  Safety: pure (undeclared)"""
    return {
        **state,
        "users": [
            {"id": 1, "name": "Alice",   "age": 30},
            {"id": 2, "name": "Bob",     "age": 17},
            {"id": 3, "name": "Charlie", "age": 25},
        ],
    }


def filter_adults(state: PipelineState) -> PipelineState:
    """Intent: Filter users to adults only  |  Safety: pure (undeclared)"""
    users  = state["users"]
    adults = [u for u in users if u["age"] >= 18]
    return {
        **state,
        "adults": {"adults": adults, "count": len(adults)},
    }


def generate_report(state: PipelineState) -> PipelineState:
    """Intent: Generate summary report  |  Safety: pure (undeclared)"""
    data  = state["adults"]
    names = [u["name"] for u in data["adults"]]
    return {
        **state,
        "report": {
            "report": f"Found {data['count']} adults: {', '.join(names)}",
            "status": "ok",
        },
    }


# ── Graph wiring ───────────────────────────────────────────────────────────────
# Developer must manually declare every node, every edge, and the entry point.
# Dependency order is implicit in edge declarations — no data-flow enforcement.

def build_graph():
    graph = StateGraph(PipelineState)

    graph.add_node("generate_users",  generate_users)
    graph.add_node("filter_adults",   filter_adults)
    graph.add_node("generate_report", generate_report)

    graph.set_entry_point("generate_users")
    graph.add_edge("generate_users",  "filter_adults")
    graph.add_edge("filter_adults",   "generate_report")
    graph.add_edge("generate_report", END)

    return graph.compile()


# ── Entrypoint ─────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    app    = build_graph()
    result = app.invoke({"users": [], "adults": {}, "report": {}})
    print(result["report"])
