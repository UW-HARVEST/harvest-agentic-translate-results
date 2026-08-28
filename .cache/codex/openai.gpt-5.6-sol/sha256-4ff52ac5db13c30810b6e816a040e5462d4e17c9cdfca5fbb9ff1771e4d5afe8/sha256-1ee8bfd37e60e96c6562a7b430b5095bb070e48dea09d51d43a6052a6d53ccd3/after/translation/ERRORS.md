# Error Surface

Mechanical source scans covered `RETURN_ERROR`, `return -1`, `return NULL`,
error enums, assertions, explicit range checks, null checks, and min/max
constants. The exported API has no pointer, length, enum, assertion, macro, or
sentinel-error branches. Its sole rejection is the `default` branch below.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] 1 | `call_predict` | `pfcn < 0` or `pfcn > 11` | returns `0` |

Generic FFI boundaries that apply: signed `int` values immediately outside the
valid range (`-1`, `12`) and the full-width boundaries (`INT_MIN`, `INT_MAX`).
Null pointers, zero/oversized lengths, and invalid enums do not apply because
the only parameter is a by-value C `int`.
