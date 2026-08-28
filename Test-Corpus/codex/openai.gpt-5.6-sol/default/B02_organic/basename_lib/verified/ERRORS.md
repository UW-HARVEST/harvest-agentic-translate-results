# Error Surface

Mechanical searches covered `RETURN_ERROR`, negative and null returns,
assertions, explicit range/null checks, enums, and min/max constants in all C
headers and sources.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are no defined rejection paths. `tool_basename` unconditionally passes
`path` to `strrchr`, so `path == NULL` has undefined behavior rather than a C
error result.

Generic FFI boundaries:

| Boundary | Applicability | [ ] |
|----------|---------------|-----|
| Null `path` | Unchecked C undefined behavior; compare isolated-process outcomes | [x] |
| Zero/oversized lengths | Not applicable: no length parameter | [x] |
| One past valid range | Not applicable: no numeric range | [x] |
| Out-of-range enum | Not applicable: no enum parameter | [x] |
