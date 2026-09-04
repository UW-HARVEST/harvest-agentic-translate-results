# Configuration surface

Mechanically derived axes from `../c_src/include/lib.h` and
`../c_src/src/lib.c`:

- no compile-time Cargo features and no C preprocessor feature flags;
- callback identity: multiply, add, XOR, or shift;
- callback lookup state: first call initializes the static table, later calls
  reuse it;
- checksum shape: 1, 2, 3, or 4 integers, versus more than 4 (truncated to 4);
- state setup/mutation and the fixed four-stage composed operation.

Randomized integer cases avoid C signed-overflow and invalid-left-shift
undefined behavior. Bitwise-only paths still exercise the full signed bit
patterns.

| # | entry point(s) | configuration (options set + input shape) | Verified |
|---|----------------|--------------------------------------------|-----|
| 1 | `multiply_with_static` | randomized safe signed integer pairs; includes negative, zero, and positive operands | [x] |
| 2 | `add_with_static` | randomized safe signed integer pairs; includes negative, zero, and positive operands | [x] |
| 3 | `xor_operation` | randomized full-width signed integer pairs | [x] |
| 4 | `shift_with_static` | safe non-negative left operand; positive and negative right operands | [x] |
| 5 | `get_operation` | cold lookup with `opcode == 0`, initializing the static callback table, then invoke returned callback | [x] |
| 6 | `get_operation` | warm lookup with `opcode == 1`, then invoke returned callback | [x] |
| 7 | `get_operation` | warm lookup with `opcode == 2`, then invoke returned callback | [x] |
| 8 | `get_operation` | warm lookup with `opcode == 3`, then invoke returned callback | [x] |
| 9 | `execute_operation` | multiply callback and valid NUL-terminated operation name | [x] |
| 10 | `execute_operation` | add callback and valid NUL-terminated operation name | [x] |
| 11 | `execute_operation` | XOR callback and valid NUL-terminated operation name | [x] |
| 12 | `execute_operation` | shift callback and valid NUL-terminated operation name | [x] |
| 13 | `compute_checksum` | `count == 1`, one initialized integer | [x] |
| 14 | `compute_checksum` | `count == 2`, two initialized integers | [x] |
| 15 | `compute_checksum` | `count == 3`, three initialized integers | [x] |
| 16 | `compute_checksum` | `count == 4`, four initialized integers | [x] |
| 17 | `compute_checksum` | `count > 4`, extra integers ignored after the fourth | [x] |
| 18 | `init_state` | non-null state and randomized full-width initial accumulator | [x] |
| 19 | `init_state`, `get_operation`, `apply_operation` | initialized state plus multiply callback; checksum must remain unchanged | [x] |
| 20 | `init_state`, `get_operation`, `apply_operation` | initialized state plus add callback; checksum must remain unchanged | [x] |
| 21 | `init_state`, `get_operation`, `apply_operation` | initialized state plus XOR callback; checksum must remain unchanged | [x] |
| 22 | `init_state`, `get_operation`, `apply_operation` | initialized state plus shift callback; checksum must remain unchanged | [x] |
| 23 | all public entry points through `checkshift` | full multiply → add → XOR → shift → checksum pipeline with randomized safe four-integer inputs | [x] |

Feature combinations: the manifest declares no `[features]` table, so the
single semantic configuration is the default/no-default feature set. Phase D
runs the suite both normally and with `--no-default-features`; both pass.
