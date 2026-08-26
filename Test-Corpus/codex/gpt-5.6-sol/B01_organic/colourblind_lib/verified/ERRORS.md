# Error Surface

Mechanical searches covered `RETURN_ERROR`, `return -1`, `return NULL`,
`assert`, null/range comparisons, and min/max constants in `c_src/include`
and `c_src/src`. The C source contains no rejection, assertion, error return,
explicit range check, or null check.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

## Generic FFI Boundaries

These are required boundary cases, but are not C rejection branches:

| # | function | boundary input | expected C result | [ ] |
|---|----------|----------------|-------------------|-----|
| B1 | `colourblind` | impairment `-1` (one below valid range) | no-op; pointed-to bytes unchanged | [x] |
| B2 | `colourblind` | impairment `3` (one above valid range) | no-op; pointed-to bytes unchanged | [x] |
| B3 | `colourblind` | impairment `INT_MIN` or `INT_MAX` | no-op; pointed-to bytes unchanged | [x] |
| B4 | `colourblind` | invalid impairment and all three pointers null | no-op; returns without dereferencing pointers | [x] |
| B5 | `colourblind` | valid impairment and any required pointer null | C undefined behavior; default C build terminates with `SIGSEGV` | [x] |

The API has no length argument, so zero and oversized lengths do not apply.
Valid-mode null pointers have no language-level C return value or sentinel;
crash parity is checked in isolated child processes against the requested
default build.
