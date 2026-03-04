"""
AutoGen equivalent of Benchmark 2: Conditional Branching Pipeline (5 nodes)
Pipeline: fetch → validate → [if valid: enrich + store] [if invalid: flag_and_log]

Conditional routing in AutoGen requires a custom speaker_selection_method function
that inspects the last message for routing signals embedded in the ValidateAgent's output.
This is non-structural: if the LLM omits the routing signal, the pipeline misfires.

NOT runnable without: pip install pyautogen
"""

import autogen
from autogen import AssistantAgent, UserProxyAgent, GroupChat, GroupChatManager

llm_config = {"model": "gpt-4o-mini", "temperature": 0}

# ── Agents ─────────────────────────────────────────────────────────────────────

orchestrator = UserProxyAgent(
    name="Orchestrator",
    human_input_mode="NEVER",
    max_consecutive_auto_reply=12,
    is_termination_msg=lambda m: "PIPELINE_COMPLETE" in (m.get("content") or ""),
    code_execution_config={"use_docker": False},
)

fetch_agent = AssistantAgent(
    name="FetchAgent",
    system_message=(
        "You are a data fetch agent. When asked, write and execute Python code returning: "
        "{'records': [{'id':'R001','value':42.5,'source':'sensor_a'},"
        "{'id':'R002','value':-7.3,'source':'sensor_b'},"
        "{'id':'R003','value':19.1,'source':'sensor_a'}], 'count':3}. "
        "Print the result as JSON."
    ),
    llm_config=llm_config,
)

validate_agent = AssistantAgent(
    name="ValidateAgent",
    system_message=(
        "You are a validation agent. When given records JSON, check if ALL values > 0. "
        "Return the records dict with an 'is_valid' boolean field. Print result as JSON. "
        "IMPORTANT: After the JSON, write exactly 'ROUTE:VALID' if valid, or 'ROUTE:INVALID' if not. "
        # NOTE: The routing signal is a string convention — the LLM may omit or rephrase it,
        # which would break routing. There is no structural enforcement.
    ),
    llm_config=llm_config,
)

enrich_store_agent = AssistantAgent(
    name="EnrichStoreAgent",
    system_message=(
        "You are an enrichment and storage agent. When given validated records JSON, "
        "write and execute Python code that multiplies each value by 1.1 to add a 'score' field, "
        "then simulates storage. Return {'enriched': [...], 'stored': N, 'status': 'ok'}. "
        "Print as JSON, then say PIPELINE_COMPLETE."
    ),
    llm_config=llm_config,
)

flag_log_agent = AssistantAgent(
    name="FlagLogAgent",
    system_message=(
        "You are a flagging agent. When given invalid records JSON, "
        "write and execute Python code that identifies non-positive values and logs them. "
        "Return {'flagged': [...], 'logged': N, 'status': 'flagged'}. "
        "Print as JSON, then say PIPELINE_COMPLETE."
    ),
    llm_config=llm_config,
)

# ── Custom routing ─────────────────────────────────────────────────────────────
# NOTE: Custom speaker selection must inspect message content for routing signals.
# This is imperative string-matching, not declarative conditional edges.

def custom_speaker_selection(last_speaker, groupchat):
    messages = groupchat.messages
    if last_speaker.name == "ValidateAgent" and messages:
        content = messages[-1].get("content", "")
        if "ROUTE:VALID" in content:
            return next(a for a in groupchat.agents if a.name == "EnrichStoreAgent")
        elif "ROUTE:INVALID" in content:
            return next(a for a in groupchat.agents if a.name == "FlagLogAgent")
    # Sequential default
    order = ["Orchestrator", "FetchAgent", "ValidateAgent"]
    for i, name in enumerate(order):
        if last_speaker.name == name and i + 1 < len(order):
            return next(a for a in groupchat.agents if a.name == order[i + 1])
    return groupchat.agents[0]


groupchat = GroupChat(
    agents=[orchestrator, fetch_agent, validate_agent, enrich_store_agent, flag_log_agent],
    messages=[],
    max_round=12,
    speaker_selection_method=custom_speaker_selection,
)

manager = GroupChatManager(groupchat=groupchat, llm_config=llm_config)

# ── Entrypoint ─────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    orchestrator.initiate_chat(
        manager,
        message=(
            "Run the conditional pipeline: "
            "1) FetchAgent: fetch records. "
            "2) ValidateAgent: validate and output ROUTE:VALID or ROUTE:INVALID. "
            "3a) If VALID: EnrichStoreAgent enriches and stores. "
            "3b) If INVALID: FlagLogAgent flags and logs. "
            "Pass JSON between steps."
        ),
    )
    # Runtime cost estimate (gpt-4o-mini, valid path):
    #   system prompts: ~270 tokens × 5 agents  = ~1,350 tokens
    #   routing signal generation: ~50 tokens
    #   7 conversation turns × ~140 tokens each = ~980 tokens
    #   code generation × 3 steps × ~100 tokens = ~300 tokens
    #   Total runtime: ~2,680 tokens
