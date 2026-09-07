# Error-surface table

Derived from every `if` condition involving a null/range/allocation rejection and
every sentinel return in `c_src/src/lib.c`. The C source has no assertions or
enum-typed parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `get_operation` | `opcode < 0` | returns `NULL` | [x] |
| 2 | `get_operation` | `opcode >= 4` | returns `NULL` | [x] |
| 3 | `execute_operation` | `func == NULL` | prints the NULL-function error and returns `0` | [x] |
| 4 | `compute_checksum` | `values == NULL` (including positive and oversized `count`) | returns `0` | [x] |
| 5 | `compute_checksum` | `values != NULL && count <= 0` | returns `0` without reading `values` | [x] |
| 6 | `init_state` | `state == NULL` | prints the NULL-state error and returns without writing | [x] |
| 7 | `apply_operation` | `state == NULL` | prints the NULL-state error and returns | [x] |
| 8 | `apply_operation` | `state != NULL && func == NULL` | prints the NULL-function error and leaves the state byte-identical | [x] |
| 9 | `checkshift` | `malloc(sizeof(ComputeState)) == NULL` | prints the allocation error and returns `-1` | [x] |

Additional generic FFI boundary cases are covered by the differential tests:
`execute_operation` with a null `op_name`, checksum count `0`, checksum count
`INT_MAX`, and operation opcodes `-1` and `4` (one step outside the valid range).
