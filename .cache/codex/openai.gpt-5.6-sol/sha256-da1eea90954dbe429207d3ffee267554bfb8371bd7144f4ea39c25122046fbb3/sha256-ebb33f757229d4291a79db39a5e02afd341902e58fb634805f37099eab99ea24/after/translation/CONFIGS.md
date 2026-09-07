# Configuration-Surface Table

Mechanically derived from all externally visible functions in
`c_src/src/lib.c`, the `switch (mode)` in `charinbuf`, and each data-shape
branch (`NULL` shapes are tracked in `ERRORS.md`).

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `reset_counter` | arbitrary negative, zero, positive, and `int` boundary value | [x] |
| 2 | `increment_counter` | counter initialized by `reset_counter`; arbitrary signed base and delta, including machine-boundary/wrapping cases | [x] |
| 3 | `decrement_counter` | counter initialized by `reset_counter`; arbitrary signed base and delta, including machine-boundary/wrapping cases | [x] |
| 4 | `multiply_counter` | counter initialized by `reset_counter`; arbitrary signed base and multiplier, including machine-boundary/wrapping cases | [x] |
| 5 | `apply_operation` + `reset_counter` | non-null operation pointer selects reset; arbitrary signed value | [x] |
| 6 | `apply_operation` + `increment_counter` | non-null operation pointer selects increment after initialized state | [x] |
| 7 | `apply_operation` + `decrement_counter` | non-null operation pointer selects decrement after initialized state | [x] |
| 8 | `apply_operation` + `multiply_counter` | non-null operation pointer selects multiply after initialized state | [x] |
| 9 | `is_string_empty` | valid pointer to an empty C string (`*str == '\0'`) | [x] |
| 10 | `is_string_empty` | valid pointer to a non-empty C string (`*str != '\0'`) | [x] |
| 11 | `find_char_in_buffer` | valid buffer with `size == 0` | [x] |
| 12 | `find_char_in_buffer` | target occurs inside scanned prefix (start, middle, and final scanned byte; arbitrary byte including NUL/high-bit bytes) | [x] |
| 13 | `find_char_in_buffer` | target occurs only after the scanned prefix | [x] |
| 14 | `find_char_in_buffer` | target absent from the scanned prefix | [x] |
| 15 | `create_buffer` | valid empty C string | [x] |
| 16 | `create_buffer` | valid non-empty C string of varied lengths/content | [x] |
| 17 | `validate_uint16_range` | `0 <= value <= UINT16_MAX`, including `0` and `65535` | [x] |
| 18 | `charinbuf` | mode `0`, valid uint16 value; all `opt1`/`opt2` values are ignored | [x] |
| 19 | `charinbuf` | mode `1`; fixed empty and non-empty strings; other arguments ignored | [x] |
| 20 | `charinbuf` | mode `2`; allocation succeeds; other arguments ignored | [x] |
| 21 | `charinbuf` | mode `3`; reset → increment → multiply → decrement pipeline with arbitrary signed operands, including machine-boundary/wrapping cases | [x] |
| 22 | `charinbuf` | mode `4`; fixed buffer contains `X` inside `strlen` bytes; other arguments ignored | [x] |

Build-time configuration surface: `Cargo.toml` declares no features, and the C
build has no feature options or conditional-compilation branches. Verification
is still run once with Cargo defaults and once with `--no-default-features`.
