"""
AutoGen equivalent of Benchmark 4: Long Pipeline with Error Recovery (8 nodes)
Pipeline: ingest → normalize → deduplicate → enrich → transform → aggregate → format → write

At 8 agents, AutoGen's source cost is dominated by system message length.
Error recovery is handled conversationally: the orchestrator detects failure
in the transform agent's output and re-prompts. Each retry consumes LLM tokens.

NOT runnable without: pip install pyautogen
"""

import autogen
from autogen import AssistantAgent, UserProxyAgent, GroupChat, GroupChatManager

llm_config = {"model": "gpt-4o-mini", "temperature": 0}

# ── Agents ─────────────────────────────────────────────────────────────────────

orchestrator = UserProxyAgent(
    name="Orchestrator",
    human_input_mode="NEVER",
    max_consecutive_auto_reply=20,
    is_termination_msg=lambda m: "PIPELINE_COMPLETE" in (m.get("content") or ""),
    code_execution_config={"use_docker": False},
)

ingest_agent = AssistantAgent(
    name="IngestAgent",
    system_message=(
        "You ingest raw data. Write and execute Python code returning: "
        "{'rows': [{'id': i, 'val': i*3, 'tag': 'raw'} for i in range(1, 11)], 'count': 10}. "
        "Print as JSON."
    ),
    llm_config=llm_config,
)

normalize_agent = AssistantAgent(
    name="NormalizeAgent",
    system_message=(
        "You normalize data. Given rows JSON, write Python code to divide each val by 30.0 "
        "and set tag='norm'. Return {'rows': [...], 'count': N}. Print as JSON."
    ),
    llm_config=llm_config,
)

dedup_agent = AssistantAgent(
    name="DedupAgent",
    system_message=(
        "You deduplicate data. Given rows JSON, write Python code to remove rows with duplicate ids. "
        "Return {'rows': [...], 'count': N}. Print as JSON."
    ),
    llm_config=llm_config,
)

enrich_agent = AssistantAgent(
    name="EnrichAgent",
    system_message=(
        "You enrich data. Given rows JSON, write Python code to add score=val*100.0 to each row. "
        "Return {'rows': [...], 'count': N}. Print as JSON."
    ),
    llm_config=llm_config,
)

transform_agent = AssistantAgent(
    name="TransformAgent",
    system_message=(
        "You compute a checksum. Given enriched rows JSON, write Python code that computes: "
        "checksum = sum(r['id'] for r in rows) % 100. "
        "Return {'rows': rows, 'checksum': checksum, 'count': N}. Print as JSON. "
        "The expected checksum is 55. If you produce a different value, fix and retry."
        # NOTE: Recovery is natural-language instruction, not a structural assertion.
        # The LLM may not self-correct; retry cost is ~500-800 tokens per attempt.
    ),
    llm_config=llm_config,
)

aggregate_agent = AssistantAgent(
    name="AggregateAgent",
    system_message=(
        "You aggregate data. Given rows with score fields, write Python code to compute "
        "sum, avg, and max of scores. Return {'sum': X, 'avg': X, 'max': X, 'count': N}. "
        "Print as JSON."
    ),
    llm_config=llm_config,
)

format_agent = AssistantAgent(
    name="FormatAgent",
    system_message=(
        "You format reports. Given aggregated stats JSON, write Python code returning: "
        "{'report': 'Processed N records: sum=X, avg=X, max=X', 'status': 'ok'}. "
        "Print as JSON."
    ),
    llm_config=llm_config,
)

write_agent = AssistantAgent(
    name="WriteAgent",
    system_message=(
        "You write output. Given formatted report JSON, return "
        "{'written': True, 'output': report_string, 'status': 'ok'}. "
        "Print as JSON, then say PIPELINE_COMPLETE."
    ),
    llm_config=llm_config,
)

# ── Group chat wiring ──────────────────────────────────────────────────────────

groupchat = GroupChat(
    agents=[
        orchestrator, ingest_agent, normalize_agent, dedup_agent, enrich_agent,
        transform_agent, aggregate_agent, format_agent, write_agent,
    ],
    messages=[],
    max_round=20,
    speaker_selection_method="round_robin",
)

manager = GroupChatManager(groupchat=groupchat, llm_config=llm_config)

# ── Entrypoint ─────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    orchestrator.initiate_chat(
        manager,
        message=(
            "Run the 8-step data pipeline in order: "
            "1) IngestAgent, 2) NormalizeAgent, 3) DedupAgent, 4) EnrichAgent, "
            "5) TransformAgent (checksum must equal 55), 6) AggregateAgent, "
            "7) FormatAgent, 8) WriteAgent. Pass JSON output between steps."
        ),
    )
    # Runtime cost estimate (gpt-4o-mini, with 1 transform retry):
    #   system prompts: ~220 tokens × 8 agents     = ~1,760 tokens
    #   16 conversation turns × ~180 tokens each   = ~2,880 tokens
    #   code generation × 8 steps × ~120 tokens    = ~960 tokens
    #   transform retry (1 attempt) × ~600 tokens  = ~600 tokens
    #   Total runtime: ~6,200 tokens
