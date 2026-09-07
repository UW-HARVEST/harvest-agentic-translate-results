# Error Surface

Mechanically derived by scanning every C source/header for error returns,
`return -1`, `return NULL`, assertions, runtime range checks, null checks,
error enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|

There are **0 rejection paths** in the C implementation. `driver(int)` accepts
every value representable by the C `int` parameter and returns `void`.
Phase C is complete with **0 unchecked rows**.

Generic FFI boundaries:

- Null pointers: not applicable; the API has no pointer parameters.
- Zero or oversized lengths: not applicable; the API has no length parameters.
- Out-of-range enum values: not applicable; the API has no enum parameters.
- One-past-range scalar values: not representable across the `int` FFI type;
  `INT_MIN`, `INT_MAX`, and arithmetic-overflow-producing inputs are covered as
  valid configurations in `CONFIGS.md`.
