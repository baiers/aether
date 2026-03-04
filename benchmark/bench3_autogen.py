"""
AutoGen equivalent of Benchmark 3: Mixed LLM + Deterministic Pipeline (4 nodes)
Pipeline: ingest → LLM classify → extract → format

This benchmark is closest to AutoGen's design intent: the classification step
is naturally conversational. However, the surrounding deterministic steps
(ingest, extract, format) still require scaffolding that AutoGen does not
make more compact than LangGraph or Aether.

NOT runnable without: pip install pyautogen
"""

import autogen
from autogen import AssistantAgent, UserProxyAgent, GroupChat, GroupChatManager

llm_config = {"model": "gpt-4o-mini", "temperature": 0}

# ── Agents ─────────────────────────────────────────────────────────────────────

orchestrator = UserProxyAgent(
    name="Orchestrator",
    human_input_mode="NEVER",
    max_consecutive_auto_reply=10,
    is_termination_msg=lambda m: "PIPELINE_COMPLETE" in (m.get("content") or ""),
    code_execution_config={"use_docker": False},
)

ingest_agent = AssistantAgent(
    name="IngestAgent",
    system_message=(
        "You ingest text documents. Return this document as JSON: "
        "{'content': 'Q3 revenue increased 23% YoY driven by enterprise sales expansion.', "
        "'source': 'report_q3.txt', 'length': 68}. Print as JSON."
    ),
    llm_config=llm_config,
)

classify_agent = AssistantAgent(
    name="ClassifyAgent",
    system_message=(
        "You classify text into exactly one of: FINANCIAL, TECHNICAL, or OPERATIONAL. "
        "When given text JSON, return {'category': '<CATEGORY>', 'confidence': 1.0}. "
        "Print as JSON."
    ),
    llm_config=llm_config,
)

extract_agent = AssistantAgent(
    name="ExtractAgent",
    system_message=(
        "You extract structured information from text. When given the original text JSON "
        "and a classification JSON, write Python code to extract numeric tokens and return "
        "{'category': '...', 'entities': [...], 'word_count': N, 'char_count': N}. "
        "Print as JSON."
    ),
    llm_config=llm_config,
)

format_agent = AssistantAgent(
    name="FormatAgent",
    system_message=(
        "You format pipeline output. When given extracted data JSON, return "
        "{'output': '[CATEGORY] N words, entities: [...]', 'status': 'ok'}. "
        "Print as JSON, then say PIPELINE_COMPLETE."
    ),
    llm_config=llm_config,
)

# ── Group chat wiring ──────────────────────────────────────────────────────────

groupchat = GroupChat(
    agents=[orchestrator, ingest_agent, classify_agent, extract_agent, format_agent],
    messages=[],
    max_round=12,
    speaker_selection_method="round_robin",
)

manager = GroupChatManager(groupchat=groupchat, llm_config=llm_config)

# ── Entrypoint ─────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    orchestrator.initiate_chat(
        manager,
        message=(
            "Run the text classification pipeline: "
            "1) IngestAgent: ingest the document. "
            "2) ClassifyAgent: classify the text. "
            "3) ExtractAgent: extract entities using the text and classification. "
            "4) FormatAgent: format the final output."
        ),
    )
    # Runtime cost estimate (gpt-4o-mini):
    #   system prompts: ~240 tokens × 4 agents   = ~960 tokens
    #   8 conversation turns × ~150 tokens each  = ~1,200 tokens
    #   code generation (ExtractAgent) × 1 step  = ~200 tokens
    #   Total runtime: ~2,360 tokens
    # NOTE: vs. Aether runtime: ~150 tokens for 1 classification call.
    #       AutoGen pays ~2,360 tokens for the same pipeline; Aether pays ~150.
