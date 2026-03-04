"""
LangGraph equivalent of Benchmark 4: Long Pipeline with Error Recovery (8 nodes)
Pipeline: ingest → normalize → deduplicate → enrich → transform* → aggregate → format → write
(* node 5 has a deliberate bug that triggers manual retry logic)

LangGraph has no built-in self-healing primitive. Retry must be implemented manually
in the node function. There is no structural assertion language, no ledger snapshot,
and no audit log of healing attempts.

NOT runnable without: pip install langgraph langchain-core
"""

from typing import TypedDict
from langgraph.graph import StateGraph, END


# ── State schema (8 fields — schema grows linearly with pipeline length) ───────

class PipelineState(TypedDict):
    ingested:    dict
    normalized:  dict
    deduped:     dict
    enriched:    dict
    transformed: dict
    aggregated:  dict
    formatted:   dict
    output:      dict


# ── Node functions ─────────────────────────────────────────────────────────────

def ingest(state: PipelineState) -> PipelineState:
    return {
        **state,
        "ingested": {
            "rows":  [{"id": i, "val": i * 3, "tag": "raw"} for i in range(1, 11)],
            "count": 10,
        },
    }


def normalize(state: PipelineState) -> PipelineState:
    data       = state["ingested"]
    normalized = [{"id": r["id"], "val": round(r["val"] / 30.0, 4), "tag": "norm"} for r in data["rows"]]
    return {**state, "normalized": {"rows": normalized, "count": data["count"]}}


def deduplicate(state: PipelineState) -> PipelineState:
    data            = state["normalized"]
    seen, deduped   = set(), []
    for r in data["rows"]:
        if r["id"] not in seen:
            seen.add(r["id"])
            deduped.append(r)
    return {**state, "deduped": {"rows": deduped, "count": len(deduped)}}


def enrich(state: PipelineState) -> PipelineState:
    data     = state["deduped"]
    enriched = [dict(r, score=r["val"] * 100.0) for r in data["rows"]]
    return {**state, "enriched": {"rows": enriched, "count": data["count"]}}


def transform_with_retry(state: PipelineState, max_retries: int = 3) -> PipelineState:
    """
    Manual retry implementation — no structural assertion language, no LLM repair,
    no ledger snapshot. The 'fix' on attempt > 0 is hard-coded, not computed.
    In a real scenario, the developer must know the correct fix in advance.
    """
    data     = state["enriched"]
    last_exc = None

    for attempt in range(max_retries):
        try:
            if attempt == 0:
                # Deliberate bug: sums score field instead of id
                checksum = sum(int(r["score"]) for r in data["rows"]) % 100
            else:
                # Manual fix — developer must hard-code the correction
                checksum = sum(r["id"] for r in data["rows"]) % 100
            assert checksum == 55, f"Checksum mismatch: got {checksum}, expected 55"
            return {
                **state,
                "transformed": {"rows": data["rows"], "checksum": checksum, "count": data["count"]},
            }
        except AssertionError as e:
            last_exc = e
            # No ledger snapshot — partial state may persist across attempts
            # No audit log — no record of healing attempts
            # No LLM repair — fix is manual and predetermined

    raise RuntimeError(f"transform failed after {max_retries} retries: {last_exc}")


def aggregate(state: PipelineState) -> PipelineState:
    data   = state["transformed"]
    scores = [r["score"] for r in data["rows"]]
    return {
        **state,
        "aggregated": {
            "sum":   sum(scores),
            "avg":   sum(scores) / len(scores),
            "max":   max(scores),
            "count": data["count"],
        },
    }


def format_output(state: PipelineState) -> PipelineState:
    d = state["aggregated"]
    return {
        **state,
        "formatted": {
            "report": f"Processed {d['count']} records: sum={d['sum']:.2f}, avg={d['avg']:.2f}, max={d['max']:.2f}",
            "status": "ok",
        },
    }


def write_output(state: PipelineState) -> PipelineState:
    data = state["formatted"]
    assert data["status"] == "ok", f"Unexpected status: {data['status']}"
    return {**state, "output": {"written": True, "output": data["report"], "status": "ok"}}


# ── Graph wiring ───────────────────────────────────────────────────────────────

def build_graph():
    graph = StateGraph(PipelineState)

    nodes = [
        ("ingest",               ingest),
        ("normalize",            normalize),
        ("deduplicate",          deduplicate),
        ("enrich",               enrich),
        ("transform_with_retry", transform_with_retry),
        ("aggregate",            aggregate),
        ("format_output",        format_output),
        ("write_output",         write_output),
    ]
    for name, fn in nodes:
        graph.add_node(name, fn)

    graph.set_entry_point("ingest")
    pairs = [(nodes[i][0], nodes[i + 1][0]) for i in range(len(nodes) - 1)]
    for a, b in pairs:
        graph.add_edge(a, b)
    graph.add_edge("write_output", END)

    return graph.compile()


# ── Entrypoint ─────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    app    = build_graph()
    result = app.invoke({k: {} for k in PipelineState.__annotations__})
    print(result["output"])
