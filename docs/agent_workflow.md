# Agentic Integration & Workflow (Volume III)

How Agents "speak" Aether and collaborate.

## 1. Action Requests (`§REQ`)
Agents use `§REQ` blocks to delegate tasks. 

```ae
§REQ 0xOPT_99 {
  ::SENDER: "Cost_Governor_v2"
  ::TARGET: "Coder_LLM_v4"
  ::CONTEXT {
    _source_id: "0xFF_MAIN",
    _issue: "Redundant Serialization"
  }
  ::INSTRUCTIONS { "Merge 0x2B and 0x3C" }
}
```

## 2. Handshake Protocol
1. **Agent A** writes a task in `.ae` containing a `§REQ`.
2. **Agent B** reads the `.ae`, processes it, and generates a solution in Aether.
3. **Agent A** (or the Orchestrator) validates the solution's `::META` intent against the original request.

## 3. "Vibe Coding" Workflow
1. **Human** provides intent ("vibe").
2. **Orchestrator** generates an Aether DAG.
3. **Kernel** performs Safety Audit and Static Resource Analysis.
4. **Human** approves high-risk nodes (e.g., L3/L4) via the "Aether Lens" UI.
5. **Kernel** executes and produces a structured JSON Execution Log.
