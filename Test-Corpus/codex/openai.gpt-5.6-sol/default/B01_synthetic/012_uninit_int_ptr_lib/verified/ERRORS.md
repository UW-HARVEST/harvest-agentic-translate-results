# Error Surface

Mechanical searches covered `c_src/include/` and `c_src/src/` for error-return
statements and macros, assertions, null checks, range checks, enums, and
minimum/maximum constants. The C source contains none, so there are no explicit
rejection rows:

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

## Required Generic Boundaries

These are required FFI boundary cases rather than explicit C rejections.

| # | function | boundary input | expected C behavior | [ ] |
|---|----------|----------------|---------------------|-----|
| G1 | `printIntPtrLine` | null `intNumber` | invalid dereference; isolate the process and compare its terminating signal | [x] |
| G2 | `driver` | `useGood == 0` | calls `bad`; isolate both calls and compare the exact observed process behavior and output | [x] |

Length and enum boundaries are not applicable: the API has no length parameters
or enum parameters. `int` spans the complete FFI argument domain for
`driver`, and its zero/nonzero partition is listed in `CONFIGS.md`.
