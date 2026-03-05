# Aether Open-Core Model

Aether is built on an **open-core** model: the runtime kernel is free and open source forever, while Aether Cloud and enterprise tooling are offered as commercial products.

---

## Community Tier — Free, Apache 2.0

Everything you need to write, run, and integrate Aether programs:

| Feature | Details |
|---------|---------|
| **Aether Kernel** | Full parser, AST, topological executor — the complete runtime |
| **Safety model** | 5-tier safety system (L0 Pure → L4 System-Root) |
| **ASL Registry** | 32 canonical intents across std.io, std.proc, std.math, std.ml, std.sec, std.text, std.sys |
| **Aether-Short (.as)** | Compact pipeline notation, auto-expanded to .ae |
| **Self-healing RETRY** | `ASSERT ... OR RETRY(n)` — uses your own `ANTHROPIC_API_KEY` |
| **English Toggle** | `aether gen "description"` — uses your own `ANTHROPIC_API_KEY` |
| **MCP server** | `aether-mcp` — full Claude Code IDE integration |
| **REST API** | `aether-api` — `/validate`, `/execute`, `/inspect`, `/grammar`, `/ui` |
| **Aether Lens** | DAG visualizer served at `/ui` and as standalone `lens/index.html` |
| **CLI** | `aether run`, `aether gen`, `aether translate` |
| **Python pip package** | `pip install aether-kernel` |
| **SDK & examples** | System prompt, MCP config, 8+ runnable .ae/.as programs |

**Source:** [github.com/baiers/aether](https://github.com/baiers/aether)
**License:** Apache 2.0

---

## Pro Tier — $29/month

For individuals and teams who want managed execution without running their own infrastructure or managing API keys:

| Feature | Details |
|---------|---------|
| **Aether Cloud execution** | Submit `.ae`/`.as` to `api.aether-lang.dev` — no local runtime required |
| **Managed self-healing** | RETRY healing via Aether's Anthropic subscription — no API key needed, Claude Sonnet quality |
| **Managed English Toggle** | `aether gen` via Aether's subscription — no API key needed |
| **Persistent execution history** | Full NodeTrace stored per-execution, browse and compare runs |
| **Cloud Lens dashboard** | Web-based DAG visualizer for all cloud executions |
| **Extended ASL registry** | 200+ canonical intents including enterprise integrations (Snowflake, Salesforce, SAP) |
| **LLM model** | Claude Sonnet 4.6 via Aether's managed subscription — no API key needed |

**Note:** Self-healing and English Toggle are available for free in Community tier when you supply your own `ANTHROPIC_API_KEY`. Both tiers use Claude Sonnet 4.6 — Pro simply removes the requirement to manage your own key.

---

## Enterprise Tier — Custom Contract

For organizations requiring on-premise deployment, compliance, and team governance:

| Feature | Details |
|---------|---------|
| **On-premise deployment** | Docker/Helm chart for private VPC, bring your own Anthropic key |
| **LLM model choice** | Claude Sonnet 4.6 (default) or Claude Opus 4.6 for maximum reasoning quality |
| **SSO / SAML** | Auth via Okta, Azure AD for the API server and dashboard |
| **RBAC safety gates** | Org policy overrides: "no L4 nodes without two-person approval" |
| **Audit log export** | NodeTrace streamed to Splunk, Datadog, S3 (SIEM integration) |
| **Private ASL registry** | Custom intents deployed org-wide, shared across teams |
| **SLA + dedicated support** | Guaranteed response times, dedicated Slack channel |
| **Compliance package** | SOC 2 report, HIPAA business associate agreement |

---

## What Will Never Be Gated

The following will always remain free and open source, regardless of tier:

- The core runtime (parser, executor, safety model)
- The `aether` CLI for local execution
- The `aether-mcp` MCP server (IDE integration)
- Self-healing (`RETRY`) when using your own `ANTHROPIC_API_KEY`
- English Toggle when using your own `ANTHROPIC_API_KEY`
- The Aether Lens visualizer
- All example programs and documentation

---

## Contact

- Community: [github.com/baiers/aether/issues](https://github.com/baiers/aether/issues)
- Pro/Enterprise: [github.com/baiers/aether/discussions](https://github.com/baiers/aether/discussions)
- Security: please open a private vulnerability report at [github.com/baiers/aether/security](https://github.com/baiers/aether/security)
