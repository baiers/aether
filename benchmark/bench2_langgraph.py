"""
LangGraph equivalent of Benchmark 2: Conditional Branching Pipeline (5 nodes)
Pipeline: fetch → validate → [if valid: enrich → store] [if invalid: flag_and_log]

LangGraph handles conditional branching natively via add_conditional_edges.
This is LangGraph's primary structural differentiator — the routing function
and conditional edge map are idiomatic and clear.

NOT runnable without: pip install langgraph langchain-core
"""

from typing import TypedDict, Literal
from langgraph.graph import StateGraph, END


# ── State schema ──────────────────────────────────────────────────────────────

class BranchState(TypedDict):
    raw:       dict
    validated: dict
    result:    dict


# ── Node functions ─────────────────────────────────────────────────────────────

def fetch_records(state: BranchState) -> BranchState:
    return {
        **state,
        "raw": {
            "records": [
                {"id": "R001", "value": 42.5, "source": "sensor_a"},
                {"id": "R002", "value": -7.3, "source": "sensor_b"},
                {"id": "R003", "value": 19.1, "source": "sensor_a"},
            ],
            "count": 3,
        },
    }


def validate_records(state: BranchState) -> BranchState:
    raw   = state["raw"]
    valid = all(r["value"] > 0 for r in raw["records"])
    return {
        **state,
        "validated": {"records": raw["records"], "is_valid": valid, "count": raw["count"]},
    }


def route_on_validity(state: BranchState) -> Literal["enrich", "flag_and_log"]:
    """LangGraph routing function — determines which branch to execute."""
    return "enrich" if state["validated"]["is_valid"] else "flag_and_log"


def enrich(state: BranchState) -> BranchState:
    data     = state["validated"]
    enriched = [dict(r, score=round(r["value"] * 1.1, 2)) for r in data["records"]]
    return {**state, "result": {"enriched": enriched, "count": len(enriched)}}


def store(state: BranchState) -> BranchState:
    data = state["result"]
    return {**state, "result": {**data, "stored": data["count"], "status": "ok"}}


def flag_and_log(state: BranchState) -> BranchState:
    data    = state["validated"]
    invalid = [r for r in data["records"] if r["value"] <= 0]
    return {
        **state,
        "result": {"flagged": invalid, "logged": len(invalid), "status": "flagged"},
    }


# ── Graph wiring ───────────────────────────────────────────────────────────────
# NOTE: add_conditional_edges is LangGraph's native conditional routing mechanism.
# The routing function + edge map is ergonomically cleaner than Aether's
# embedded branch-guard pattern for this use case.

def build_graph():
    graph = StateGraph(BranchState)

    graph.add_node("fetch_records",   fetch_records)
    graph.add_node("validate_records", validate_records)
    graph.add_node("enrich",          enrich)
    graph.add_node("store",           store)
    graph.add_node("flag_and_log",    flag_and_log)

    graph.set_entry_point("fetch_records")
    graph.add_edge("fetch_records", "validate_records")
    graph.add_conditional_edges(
        "validate_records",
        route_on_validity,
        {"enrich": "enrich", "flag_and_log": "flag_and_log"},
    )
    graph.add_edge("enrich",       "store")
    graph.add_edge("store",        END)
    graph.add_edge("flag_and_log", END)

    return graph.compile()


# ── Entrypoint ─────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    app    = build_graph()
    result = app.invoke({"raw": {}, "validated": {}, "result": {}})
    print(result["result"])
