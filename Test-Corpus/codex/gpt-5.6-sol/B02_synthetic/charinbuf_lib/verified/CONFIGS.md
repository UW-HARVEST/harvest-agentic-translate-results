# Configuration Surface

Build-time configuration has one state: CMake defaults and the empty Rust
feature set (`--no-default-features`). The runtime rows below are derived from
the ten exported entry points, the `charinbuf` mode switch, each explicit C
branch, and the string/buffer/count shapes consumed by the C library.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `reset_counter` | arbitrary negative, zero, positive, and boundary `int` values replace static state | [x] |
| 2 | `increment_counter` | reset state first; negative, zero, and positive increments without signed overflow | [x] |
| 3 | `decrement_counter` | reset state first; negative, zero, and positive decrements without signed overflow | [x] |
| 4 | `multiply_counter` | reset state first; negative, zero, one, and positive multipliers without signed overflow | [x] |
| 5 | all counter functions | many-operation stateful sequences, including actual compiled-C overflow boundaries | [x] |
| 6 | `is_string_empty` | valid empty C string (`*str == '\0'`) | [x] |
| 7 | `is_string_empty` | valid non-empty C string (`*str != '\0'`), including arbitrary first bytes | [x] |
| 8 | `find_char_in_buffer` | valid non-null buffer with `size == 0` | [x] |
| 9 | `find_char_in_buffer` | one-byte buffer, target present | [x] |
| 10 | `find_char_in_buffer` | one-byte buffer, target absent | [x] |
| 11 | `find_char_in_buffer` | many-byte binary buffer, target at first/middle/last position | [x] |
| 12 | `find_char_in_buffer` | many-byte binary buffer, target absent | [x] |
| 13 | `find_char_in_buffer` | binary buffer containing embedded NUL, including NUL as target | [x] |
| 14 | `find_char_in_buffer` | oversized `size == SIZE_MAX` with target in the first byte (generic length boundary without out-of-bounds traversal) | [x] |
| 15 | `create_buffer` | valid empty C string | [x] |
| 16 | `create_buffer` | valid non-empty C strings of varied lengths | [x] |
| 17 | `create_buffer` | input storage has bytes after an embedded NUL; C-string prefix only is copied | [x] |
| 18 | `validate_uint16_range` | minimum valid value `0` | [x] |
| 19 | `validate_uint16_range` | randomized interior values `1..65534` | [x] |
| 20 | `validate_uint16_range` | maximum valid value `UINT16_MAX` (`65535`) | [x] |
| 21 | `apply_operation` | valid external callback and randomized `int` values | [x] |
| 22 | `apply_operation` + counter functions | each exported counter operation supplied as the callback, with state initialized by `reset_counter` | [x] |
| 23 | `charinbuf` mode `0` | valid `value` at `0`, randomized interior points, and `65535`; `opt1`/`opt2` arbitrary and ignored | [x] |
| 24 | `charinbuf` mode `1` | fixed empty/non-empty string checks; all other arguments arbitrary and ignored | [x] |
| 25 | `charinbuf` mode `2` | successful fixed-string allocation/copy/free; all other arguments arbitrary and ignored | [x] |
| 26 | `charinbuf` mode `3` | reset/increment/multiply/decrement pipeline over randomized negative, zero, and positive operands without signed overflow | [x] |
| 27 | `charinbuf` mode `3` | boundary operands exercising actual compiled-C wrapping behavior | [x] |
| 28 | `charinbuf` mode `4` | successful fixed-buffer search for `X`; all other arguments arbitrary and ignored | [x] |
