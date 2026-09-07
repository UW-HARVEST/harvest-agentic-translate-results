# Configuration-surface table

The public ABI is the ten symbols exported by the C shared library, not only the
single function declared in `include/lib.h`. There are no compile-time Cargo
features and no C preprocessor feature switches. Rows below are the branch
cross-product actually distinguished by `c_src/src/lib.c`: operation selection,
first versus cached operation lookup, checksum count buckets, null-free state
operations, and the composed pipeline.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `multiply_with_static` | randomized signed 32-bit operand pairs, including zero/sign/boundary/overflow cases | [x] |
| 2 | `add_with_static` | randomized signed 32-bit operand pairs, including zero/sign/boundary/overflow cases | [x] |
| 3 | `xor_operation` | randomized signed 32-bit operand pairs, including zero/sign/boundary cases | [x] |
| 4 | `shift_with_static` | randomized signed 32-bit operand pairs, including negative right operands and left-shift overflow | [x] |
| 5 | `get_operation` + returned function | fresh library state, opcode `0` (multiply) | [x] |
| 6 | `get_operation` + returned function | fresh library state, opcode `1` (add) | [x] |
| 7 | `get_operation` + returned function | fresh library state, opcode `2` (xor) | [x] |
| 8 | `get_operation` + returned function | fresh library state, opcode `3` (shift) | [x] |
| 9 | `get_operation` + returned function | warmed/cached operation table, opcode `0` | [x] |
| 10 | `get_operation` + returned function | warmed/cached operation table, opcode `1` | [x] |
| 11 | `get_operation` + returned function | warmed/cached operation table, opcode `2` | [x] |
| 12 | `get_operation` + returned function | warmed/cached operation table, opcode `3` | [x] |
| 13 | `execute_operation` | non-null multiply function and non-null operation name; randomized operands | [x] |
| 14 | `execute_operation` | non-null add function and non-null operation name; randomized operands | [x] |
| 15 | `execute_operation` | non-null xor function and non-null operation name; randomized operands | [x] |
| 16 | `execute_operation` | non-null shift function and non-null operation name; randomized operands | [x] |
| 17 | `execute_operation` | non-null function and null `op_name` (glibc `%s` null-pointer behavior) | [x] |
| 18 | `compute_checksum` | non-null values, `count == 1` (copies one native-endian `int`) | [x] |
| 19 | `compute_checksum` | non-null values, `count == 2` | [x] |
| 20 | `compute_checksum` | non-null values, `count == 3` | [x] |
| 21 | `compute_checksum` | non-null values, `count == 4` | [x] |
| 22 | `compute_checksum` | non-null values, `count > 4`, including `INT_MAX` (clamped to four ints) | [x] |
| 23 | `init_state` | non-null state and randomized signed 32-bit initial value | [x] |
| 24 | `apply_operation` | non-null state with multiply function; randomized state/value | [x] |
| 25 | `apply_operation` | non-null state with add function; randomized state/value | [x] |
| 26 | `apply_operation` | non-null state with xor function; randomized state/value | [x] |
| 27 | `apply_operation` | non-null state with shift function; randomized state/value | [x] |
| 28 | `checkshift` | full end-to-end pipeline with randomized four-tuples, including zero/sign/boundary/overflow cases | [x] |
