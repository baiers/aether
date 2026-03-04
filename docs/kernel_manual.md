# Aether Kernel Manual (Volume II)

The Aether Kernel is the engine that parses, validates, and executes `.ae` files.

## 1. Execution Lifecycle
1. **Ingestion**: Lexer/Parser converts `.ae` into an Internal Dependency Graph (IDG).
2. **Cyclic Verification**: Runs a DFS algorithm to ensure no circular dependencies exist.
3. **Governance Pre-Flight**:
    - Scans `::META` for safety levels (L0-L4).
    - Checks projected resource usage against User Budget Policy.
4. **Topological Scheduling**: Nodes are queued based on dependencies. Independent nodes are parallelized.
5. **Sanitized Dispatch**: Spins up a sandbox (Firecracker/Wasm), injects data, executes, and extracts output.
6. **Validation & Teardown**: Asserts `::VALIDATE` rules and commits output to the Global State Ledger.

## 2. Safety Levels (The Governor)
| Level | Tag | Description | Allowed Actions |
| :--- | :--- | :--- | :--- |
| L0 | `pure` | Math/Logic only. No IO. | CPU, RAM. |
| L1 | `read_only` | Idempotent reads. | GET requests, File Read. |
| L2 | `state_mod` | Writes to local state. | File Write, SQL Insert. |
| L3 | `full_egress` | External network calls. | POST requests, Email. |
| L4 | `system_root` | High-privilege access. | Shell access, pip install. |

## 3. Static Resource Analysis
The Kernel performs pre-flight checks by combining `::META` complexity (e.g., `O(n^2)`) with input data sizes to predict costs and prevent resource exhaustion.
