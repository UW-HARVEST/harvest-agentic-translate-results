# Error Surface

The C source contains no error-return statements, error enums, assertions,
explicit range checks, null checks, or min/max rejection constants. The rows
below are the mandatory generic FFI-boundary invalid cases and the two public
paths that mechanically reach the C source's uninitialized-pointer
dereference.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| 1 | `printIntPtrLine` | `intNumber == NULL` | Undefined behavior from `*intNumber`; compare the built C and Rust libraries in isolated subprocesses | [x] |
| 2 | `bad` | Always: local `data` is read without initialization and passed to `printIntPtrLine` | Undefined behavior from an indeterminate pointer; compare isolated subprocess outcomes | [x] |
| 3 | `driver` | `useGood == 0`, which calls `bad` | Same behavior as `bad`; compare isolated subprocess outcomes | [x] |

There are no length parameters, enum parameters, allocation results, or
documented numeric ranges in this API, so zero/oversized lengths and
out-of-range enum tests are not applicable.

All rows pass in both the default and `--no-default-features`
configurations.
