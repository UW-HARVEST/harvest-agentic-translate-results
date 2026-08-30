# Error Surface

Mechanically derived from the null and range predicates in
`../c_src/src/driver.c`. The API returns `void`, so rejection is observable as
exact stdout bytes.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| E1 | `printLine` | `line == NULL` (inverse of line 31) | no output; returns `void` | [x] |
| E2 | `bad` | `data < 0` (inverse of line 46) | `ERROR: Array index is negative.\n` | [x] |
| E3 | `good` / `goodB2G` | `data < 0` (first false term at line 85) | ten fixed `goodG2B` integer lines, then `ERROR: Array index is out-of-bounds\n` | [x] |
| E4 | `good` / `goodB2G` | `data >= 10` (second false term at line 85) | ten fixed `goodG2B` integer lines, then `ERROR: Array index is out-of-bounds\n` | [x] |
| E5 | `driver` via `good` | `goodData < 0` | exact composed `driver` output with the `goodB2G` out-of-bounds error | [x] |
| E6 | `driver` via `good` | `goodData >= 10` | exact composed `driver` output with the `goodB2G` out-of-bounds error | [x] |
| E7 | `driver` via `bad` | `badData < 0` | exact composed `driver` output with the negative-index error | [x] |

## Boundary Audit

- `goodG2B` has a syntactic negative-index error branch at lines 75-78, but
  its local `data` is unconditionally initialized to `7` at line 63. No API
  input can reach that branch, so it is not a constructible rejection row.
- No length parameters, enums, error codes, error-return statements, asserts,
  option setters, min/max macros, or compile-time feature branches exist.
- `bad` checks only `data >= 0`; it does **not** reject `data >= 10`.
  Such calls execute an out-of-bounds C write and therefore have undefined
  behavior. The one-past boundary is compared only in isolated subprocesses.
