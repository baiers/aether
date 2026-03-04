# Aether Type System (Volume I)

Aether is strictly typed to ensure deterministic data exchange between AI agents. Variables (`$0x...`) are immutable and must match their declared type.

| Type Name | Serialization Format | Description |
| :--- | :--- | :--- |
| `Type<Bool>` | Boolean (`true`/`false`) | Logic gate status. |
| `Type<Int>` | 64-bit Integer | Numerical counters and indices. |
| `Type<Float>` | 64-bit Float | Precise measurements. |
| `Type<String>` | UTF-8 String | Textual data. |
| `Type<JSON>` | Minified JSON String | Structured data exchange. |
| `Type<Blob>` | Base64 String | Raw binary or file content. |
| `Type<Tensor>` | N-Dimensional Array | Multi-dimensional data for AI weights/math. |
| `Type<Ref>` | Memory Address | Pointer to another Node's output. |
| `Type<Map>` | JSON Object | Key-value pairs. |
| `Type<List>` | JSON Array | Ordered collection of elements. |

## Specialized AI Types
- `Type<JSON_String>`: A string containing valid JSON.
- `Type<JSON_Object>`: A parsed JSON object.
- `Type<Table>`: Tabular data (often serialized as CSV or Parquet refs).
