"""
LangGraph equivalent of examples/demo_showcase.ae
Financial Transaction Anomaly Monitor

Implements the same 5 deterministic nodes as the Aether demo.
The 6th node (fraud alert) is omitted — net_egress gating is not a LangGraph concept.

Structural differences vs. Aether (not missing features — structural absence):
  - No safety model: there is no mechanism to declare that a node requires
    net_egress capability and have the runtime block it without explicit approval.
    You would need to build this as a separate middleware or policy layer.
  - No RETRY with LLM repair: LangGraph supports conditional edges and error
    handling, but it has no concept of sending a failing node's code + failure
    evidence to a model for automated repair. You would need to write that
    infrastructure yourself (~100–150 lines) and wire it into the graph.
  - No audit trail: LangGraph does not automatically record per-node timing,
    per-node output, and validation results into a structured JSON log.
    You would need to instrument each node and aggregate the results manually.
"""

from typing import TypedDict, List, Dict, Any, Optional


# ── State ─────────────────────────────────────────────────────────────────────

class PipelineState(TypedDict):
    batch_id: str
    transactions: List[Dict[str, Any]]
    count: int
    valid_count: int
    total_count: int
    invalid_ids: List[str]
    scored_transactions: List[Dict[str, Any]]
    flagged_count: int
    flagged: List[Dict[str, Any]]
    total_exposure: float
    report: str
    flagged_ids: List[str]
    exposure_usd: float
    status: str


# ── Nodes ─────────────────────────────────────────────────────────────────────

def ingest(state: PipelineState) -> PipelineState:
    """Ingest a batch of financial transactions (synthetic data)."""
    return {
        **state,
        "transactions": [
            {"id": "TX001", "amount": 45.00,    "currency": "USD", "country": "UK", "merchant": "Coffee & Bakery",        "timestamp": "2026-03-04T08:12:00Z"},
            {"id": "TX002", "amount": 89.00,    "currency": "USD", "country": "US", "merchant": "Pharmacy",               "timestamp": "2026-03-04T08:34:00Z"},
            {"id": "TX003", "amount": 120.00,   "currency": "USD", "country": "CA", "merchant": "Grocery Mart",           "timestamp": "2026-03-04T09:01:00Z"},
            {"id": "TX004", "amount": 175.00,   "currency": "USD", "country": "US", "merchant": "Online Retailer",        "timestamp": "2026-03-04T09:17:00Z"},
            {"id": "TX005", "amount": 230.00,   "currency": "USD", "country": "DE", "merchant": "Electronics Store",      "timestamp": "2026-03-04T09:38:00Z"},
            {"id": "TX006", "amount": 310.00,   "currency": "USD", "country": "AU", "merchant": "Restaurant",             "timestamp": "2026-03-04T09:55:00Z"},
            {"id": "TX007", "amount": 67.00,    "currency": "USD", "country": "US", "merchant": "Gas Station",            "timestamp": "2026-03-04T10:08:00Z"},
            {"id": "TX008", "amount": 9850.00,  "currency": "USD", "country": "NG", "merchant": "Cash Transfer Service",  "timestamp": "2026-03-04T10:22:00Z"},
            {"id": "TX009", "amount": 12400.00, "currency": "USD", "country": "RU", "merchant": "Crypto Exchange",        "timestamp": "2026-03-04T10:23:00Z"},
            {"id": "TX010", "amount": 15200.00, "currency": "USD", "country": "CN", "merchant": "Unregistered Vendor",    "timestamp": "2026-03-04T10:24:00Z"},
        ],
        "batch_id": "BATCH-20260304-001",
        "count": 10,
    }


def validate_schema(state: PipelineState) -> PipelineState:
    """Validate that all transactions have required fields and positive amounts."""
    required = {"id", "amount", "currency", "country", "merchant", "timestamp"}
    valid, invalid = [], []
    for tx in state["transactions"]:
        if required.issubset(tx.keys()) and isinstance(tx["amount"], (int, float)) and tx["amount"] > 0:
            valid.append(tx)
        else:
            invalid.append(tx.get("id", "UNKNOWN"))

    # NOTE: In LangGraph, there is no HALT mechanism that stops the graph
    # mid-execution if validation fails — you must check state in the next node
    # or add a conditional edge. Aether's ASSERT ... OR HALT is structural.
    if invalid:
        raise ValueError(f"Schema validation failed for: {invalid}")

    return {
        **state,
        "transactions": valid,
        "valid_count": len(valid),
        "total_count": state["count"],
        "invalid_ids": invalid,
    }


def score_anomalies(state: PipelineState) -> PipelineState:
    """Score each transaction: how many times its amount exceeds the batch median.
    Score 1.0 = 10x or more the median.

    NOTE: This is the corrected version. The Aether demo deliberately injects
    baseline=1.0 here, which causes all scores to be 1.0. In Aether, the
    ASSERT catches this at runtime and RETRY sends the failing code + evidence
    to Claude Haiku for automatic repair. LangGraph has no equivalent —
    you would need to build that repair infrastructure manually.
    """
    amounts = sorted(tx["amount"] for tx in state["transactions"])
    n = len(amounts)
    median = (amounts[n // 2 - 1] + amounts[n // 2]) / 2 if n % 2 == 0 else amounts[n // 2]
    baseline = median  # Correctly computed — Aether demo starts with baseline=1.0 (bug)

    scored = []
    for tx in state["transactions"]:
        ratio = tx["amount"] / max(baseline, 1.0)
        anomaly_score = round(min(ratio / 10.0, 1.0), 4)
        scored.append({**tx, "anomaly_score": anomaly_score})

    flagged_count = sum(1 for tx in scored if tx["anomaly_score"] > 0.7)

    return {
        **state,
        "scored_transactions": scored,
        "flagged_count": flagged_count,
    }


def filter_high_risk(state: PipelineState) -> PipelineState:
    """Extract high-risk transactions and assign risk labels."""
    flagged = []
    for tx in state["scored_transactions"]:
        if tx["anomaly_score"] > 0.7:
            label = "CRITICAL" if tx["anomaly_score"] >= 0.95 else "HIGH"
            flagged.append({**tx, "risk_label": label, "action": "BLOCK_AND_REVIEW"})

    return {
        **state,
        "flagged": flagged,
        "total_exposure": round(sum(tx["amount"] for tx in flagged), 2),
    }


def compile_report(state: PipelineState) -> PipelineState:
    """Compile the compliance audit report.

    NOTE: In Aether, this node is declared _safety: state_mod. The runtime
    checks this declaration before executing and blocks the node if the
    environment permits only read_only operations. There is no LangGraph
    equivalent — safety gating requires a separate policy layer.
    """
    data = state
    lines = [
        "=== FRAUD DETECTION AUDIT REPORT ===",
        f"Batch:          {data['batch_id']}",
        f"Transactions:   {len(data['flagged'])} flagged",
        f"Total exposure: ${data['total_exposure']:,.2f}",
        "",
        "Flagged Transactions:",
    ]
    for tx in data["flagged"]:
        lines.append(
            f"  [{tx['risk_label']}] {tx['id']}: ${tx['amount']:,.2f} "
            f"via {tx['merchant']} ({tx['country']}) — score {tx['anomaly_score']}"
        )
    lines += ["", "Recommended action: BLOCK_AND_REVIEW all flagged transactions."]

    return {
        **state,
        "report": "\n".join(lines),
        "flagged_ids": [tx["id"] for tx in data["flagged"]],
        "exposure_usd": data["total_exposure"],
        "status": "AUDIT_COMPLETE",
    }


# NOTE: The net_egress alert node (0x6F in Aether) is omitted.
# In Aether, declaring _safety: net_egress causes the runtime to BLOCK the node
# if the environment permits only state_mod or lower — visible as BLOCKED in the
# NodeTrace. LangGraph has no concept of capability-gated execution.


# ── Graph ─────────────────────────────────────────────────────────────────────

def build_graph():
    from langgraph.graph import StateGraph

    graph = StateGraph(PipelineState)
    graph.add_node("ingest",          ingest)
    graph.add_node("validate_schema", validate_schema)
    graph.add_node("score_anomalies", score_anomalies)
    graph.add_node("filter_high_risk",filter_high_risk)
    graph.add_node("compile_report",  compile_report)
    graph.set_entry_point("ingest")
    graph.add_edge("ingest",           "validate_schema")
    graph.add_edge("validate_schema",  "score_anomalies")
    graph.add_edge("score_anomalies",  "filter_high_risk")
    graph.add_edge("filter_high_risk", "compile_report")
    graph.add_edge("compile_report",   "__end__")
    return graph.compile()


if __name__ == "__main__":
    app = build_graph()
    result = app.invoke({})
    print(result["report"])
