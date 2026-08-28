# Error Surface

Derived from every `NULL`, range, allocation, and early-return condition in
`../c_src/src/lib.c`. Rows 1-7 are source branches. Row 8 is the additional
generic null-pointer FFI boundary for the unchecked `op_name` argument.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `get_operation` | `opcode < 0 || opcode >= 4` (the inverse of `opcode >= 0 && opcode < 4`) | returns `NULL` | [x] |
| 2 | `execute_operation` | `func == NULL` | returns `0`; does not invoke a function | [x] |
| 3 | `compute_checksum` | `values == NULL` with positive `count` | returns `0`; reads no bytes | [x] |
| 4 | `compute_checksum` | non-null `values` with `count <= 0` | returns `0`; reads no bytes | [x] |
| 5 | `init_state` | `state == NULL` | returns `void`; performs no write | [x] |
| 6 | `apply_operation` | `state == NULL` | returns `void`; performs no write or call | [x] |
| 7 | `apply_operation` | non-null `state` and `func == NULL` | returns `void`; state remains byte-identical | [x] |
| 8 | `checkshift` | `malloc(sizeof(ComputeState)) == NULL` | returns `-1` | [x] |
| 9 | `execute_operation` | non-null `func` and `op_name == NULL` (generic FFI null boundary; C has no rejection branch) | returns the exact function result; on this glibc target `%s` prints `(null)` | [x] |

There are no assertions, error enums, `RETURN_ERROR` macros, public enum
arguments, or caller-supplied byte lengths. The sole count is
`compute_checksum.count`; counts greater than four are accepted and capped,
and are covered as a valid configuration.
