# Error and boundary surface

The first table is derived mechanically from every explicit rejection,
null/range check, and failure return in `../c_src/src/lib.c`. Separate rows are
used when opposite sides of a range need distinct FFI inputs.

| # | function | trigger (the exact invalid input/condition) | expected C result | Verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `get_operation` | `opcode < 0` (the `opcode >= 0 && opcode < 4` check fails) | returns `NULL` | [x] |
| 2 | `get_operation` | `opcode >= 4` (the `opcode >= 0 && opcode < 4` check fails) | returns `NULL` | [x] |
| 3 | `execute_operation` | `func == NULL` | prints the error and returns `0` | [x] |
| 4 | `compute_checksum` | `values == NULL` (including a positive `count`) | returns `0` | [x] |
| 5 | `compute_checksum` | non-null `values` and `count <= 0` | returns `0` without reading `values` | [x] |
| 6 | `init_state` | `state == NULL` | prints the error and returns without writing | [x] |
| 7 | `apply_operation` | `state == NULL` | prints the error and returns without calling `func` | [x] |
| 8 | `apply_operation` | non-null `state` and `func == NULL` | prints the error and leaves the complete state unchanged | [x] |
| 9 | `checkshift` | `malloc(sizeof(ComputeState)) == NULL` | prints the error and returns `-1` | [x] |

Phase C also requires generic FFI boundaries even where C accepts rather than
rejects them:

| # | function | boundary input/condition | expected C result | Verified |
|---|----------|--------------------------|-------------------|----------|
| 10 | `execute_operation` | valid `func`, `op_name == NULL` | on this glibc target `%s` accepts null; callback result is returned | [x] |
| 11 | `compute_checksum` | non-null `values`, `count == 0` | returns `0` | [x] |
| 12 | `compute_checksum` | non-null four-element buffer, oversized `count == INT_MAX` | clamps to four integers and returns their checksum | [x] |
| 13 | `get_operation` | one step below valid range: `opcode == -1` | returns `NULL` | [x] |
| 14 | `get_operation` | one step above valid range: `opcode == 4` | returns `NULL` | [x] |

There are no C enum parameters. `opcode` is the only integer selector crossing
the FFI boundary, and rows 1, 2, 13, and 14 cover invalid selector values.
