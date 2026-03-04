# Aether Standard Library (ASL) Registry (Volume IV)

The ASL is a shared dictionary of intents used in `::META` blocks to prevent "Semantic Drift" between agents.

## 1. I/O & Networking (`std.io`)
| Intent ID | Description | Safety | Recommended Guest |
| :--- | :--- | :--- | :--- |
| `std.io.net_get` | Standard HTTP GET request. | L1 | Python (Requests) |
| `std.io.net_post` | HTTP POST with payload. | L3 | Python (Requests) |
| `std.io.fs_write` | Persistent file creation. | L2 | Python / JS |
| `std.io.db_query` | Execute SQL Select statement. | L1 | SQL |

## 2. Data Processing (`std.proc`)
| Intent ID | Description | Safety | Logic Type |
| :--- | :--- | :--- | :--- |
| `std.proc.transform` | Data mapping/cleaning. | L0 | Deterministic |
| `std.proc.list.filter` | Remove items based on criteria. | L0 | Functional |
| `std.proc.tab.pivot` | Reshape table structure. | L0 | Pandas |

## 3. AI & ML (`std.ml`)
| Intent ID | Description | Safety | AI Type |
| :--- | :--- | :--- | :--- |
| `std.ml.infer.text` | Request text generation. | L3 | LLM |
| `std.ml.vec.search` | Cosine similarity in vector space. | L0 | Vector DB |
| `std.ml.agent.handshake`| Initiate Aether §REQ to peer. | L3 | Orchestration |

## 4. Security (`std.sec`)
| Intent ID | Description | Safety | Encryption |
| :--- | :--- | :--- | :--- |
| `std.sec.hash.sha2` | Secure SHA-256 Hashing. | L0 | Hashing |
| `std.sec.mask.pii` | Anonymize sensitive strings. | L0 | Privacy |

> [!NOTE]
> For a full list of 100+ entries, refer to the Volume IV Technical Specification.
