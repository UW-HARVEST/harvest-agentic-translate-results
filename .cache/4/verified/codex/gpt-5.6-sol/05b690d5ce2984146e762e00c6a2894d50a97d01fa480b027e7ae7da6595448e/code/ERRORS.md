# Error Surface

Mechanically derived from every pointer/range check and error return in
`c_src/src/lib.c`. There are no C `assert` statements, error enums, or
error-return macros. `checkshift`'s allocation failure is included even though
it requires allocator interposition to trigger.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|---|
| E01 | `get_operation` | `opcode < 0` | returns `NULL` | [x] |
| E02 | `get_operation` | `opcode >= 4` (including `INT_MAX`) | returns `NULL` | [x] |
| E03 | `execute_operation` | `func == NULL` | returns `0`; does not invoke a callback | [x] |
| E04 | `compute_checksum` | `values == NULL` with `count > 0` | returns `0` | [x] |
| E05 | `compute_checksum` | non-null `values` with `count == 0` | returns `0` | [x] |
| E06 | `compute_checksum` | non-null `values` with `count < 0` (including `INT_MIN`) | returns `0` | [x] |
| E07 | `init_state` | `state == NULL` | returns without writing state | [x] |
| E08 | `apply_operation` | `state == NULL` | returns without invoking `func` | [x] |
| E09 | `apply_operation` | non-null `state` and `func == NULL` | returns without changing any state byte | [x] |
| E10 | `checkshift` | `malloc(sizeof(ComputeState)) == NULL` | returns `-1` | [x] |

## Generic FFI Boundaries

The tests also exercise `execute_operation` with a null `op_name`. C does not
reject that pointer; on the target libc `%s` renders it while the callback is
still invoked. Oversized checksum counts use the C `int` maximum and are
covered as valid truncating inputs in `CONFIGS.md`. There are no enum
parameters in this API; `get_operation`'s integer opcode rows cover values
outside the documented dispatch range.

