# Aether Whitepaper: The Intermediate Representation for the Agentic Age

## 1. Introduction
In the transition from traditional software to agentic systems, the layer between "Natural Language Intent" and "Machine Execution" remains fragmented. Current AI agents often generate high-level code (Python, JS) that is non-deterministic, hard to verify, and carries significant security risks.

**Aether (AE)** is proposed as a new intermediate layer. It is a language where **Intent is Compilation**.

## 2. The Atomic Unit: The Action Node (§ACT)
Execution in Aether is structured around Action Nodes. Each node is an atomic, immutable block of logic.

```ae
§ACT 0x1A {
  ::META { _intent: "Fetch user profile", _safety: "net_egress" }
  ::IN { $0x1: "123" }
  ::EXEC<PYTHON> {
    return {"id": $0x1, "name": "Agent Smith"}
  }
  ::OUT { $0x2: Type<JSON> }
}
```

## 3. The Memory Model: Hex-Hash Addressing
Aether does not use variables in the traditional sense. Outputs of nodes are addressed by their sequence or a generated hex hash (e.g., `$0x1`). This ensures that data once written cannot be mutated, only transformed into a new node output.

## 4. Safety Taxonomy
Every Action Node must declare its safety Level (L0–L4):
- **L0 (Pure)**: No state changes, no I/O.
- **L1 (Read-Only)**: File system/DB read access only.
- **L2 (State-Mod)**: Local state modifications allowed.
- **L3 (Net-Egress)**: Network calls allowed.
- **L4 (System-Root)**: Full system access.

## 5. The Kernel
The Aether Kernel acts as the orchestrator. It parses `.ae` files, verifies intents against the "Vibe Registry", and dispatches execution to Guest Language Runners in isolated MicroVMs or WASM sandboxes.
