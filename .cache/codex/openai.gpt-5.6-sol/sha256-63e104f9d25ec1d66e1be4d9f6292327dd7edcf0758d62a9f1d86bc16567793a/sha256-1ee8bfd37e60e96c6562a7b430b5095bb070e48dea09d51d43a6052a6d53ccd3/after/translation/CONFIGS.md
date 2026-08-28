# Configuration Surface

Derived from the public dynamic symbols, the `if` conditions, function-pointer
dispatch, count cap, and composed call graph in `../c_src/src/lib.c`.

There are no Cargo features, C preprocessor feature switches, runtime option
setters, modes, flags, enums, element-type choices, or format/byte-order
options. Integer byte order is the host C ABI byte order. Randomized rows cover
zero, positive, negative, and boundary `int` values.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `multiply_with_static` | arbitrary pair of C `int` values | [x] |
| 2 | `add_with_static` | arbitrary pair of C `int` values | [x] |
| 3 | `xor_operation` | arbitrary pair of C `int` values | [x] |
| 4 | `shift_with_static` | arbitrary pair of C `int` values | [x] |
| 5 | `get_operation` | first valid request, exercising `ops[0] == NULL` lazy initialization | [x] |
| 6 | `get_operation` | initialized table, `opcode == 0`; invoke returned multiply function | [x] |
| 7 | `get_operation` | initialized table, `opcode == 1`; invoke returned add function | [x] |
| 8 | `get_operation` | initialized table, `opcode == 2`; invoke returned XOR function | [x] |
| 9 | `get_operation` | initialized table, `opcode == 3`; invoke returned shift function | [x] |
| 10 | `execute_operation` | non-null multiply function and non-null operation name | [x] |
| 11 | `execute_operation` | non-null add function and non-null operation name | [x] |
| 12 | `execute_operation` | non-null XOR function and non-null operation name | [x] |
| 13 | `execute_operation` | non-null shift function and non-null operation name | [x] |
| 14 | `compute_checksum` | non-null values, `count == 1` | [x] |
| 15 | `compute_checksum` | non-null values, `count == 2` | [x] |
| 16 | `compute_checksum` | non-null values, `count == 3` | [x] |
| 17 | `compute_checksum` | non-null values, `count == 4` | [x] |
| 18 | `compute_checksum` | non-null values, `count > 4`; only the first four integers are read | [x] |
| 19 | `init_state` | non-null state and arbitrary initial C `int` | [x] |
| 20 | `apply_operation` | non-null state and multiply function | [x] |
| 21 | `apply_operation` | non-null state and add function | [x] |
| 22 | `apply_operation` | non-null state and XOR function | [x] |
| 23 | `apply_operation` | non-null state and shift function | [x] |
| 24 | `init_state`, `get_operation`, `apply_operation`, `execute_operation`, `compute_checksum` | full low-level pipeline with four arbitrary C `int` parameters | [x] |
| 25 | `checkshift` | one-shot full operation with four arbitrary C `int` parameters | [x] |
