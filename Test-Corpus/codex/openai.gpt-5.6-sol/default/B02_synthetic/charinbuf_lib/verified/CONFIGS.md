# Configuration Surface

Mechanically derived from all externally visible C entry points and each
option, mode, state, range, pointer, size, target, and allocation branch in
`../c_src/src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `reset_counter` | arbitrary `int` value, including `INT_MIN`, zero, and `INT_MAX` | [x] |
| 2 | `increment_counter` | arbitrary counter/value pair whose mathematical sum is representable as `int` | [x] |
| 3 | `decrement_counter` | arbitrary counter/value pair whose mathematical difference is representable as `int` | [x] |
| 4 | `multiply_counter` | arbitrary counter/value pair whose mathematical product is representable as `int` | [x] |
| 5 | counter entry points | mixed reset/increment/decrement/multiply stateful sequences with representable intermediate results | [x] |
| 6 | `is_string_empty` | valid pointer whose first byte is NUL (empty string) | [x] |
| 7 | `is_string_empty` | valid pointer whose first byte is non-NUL | [x] |
| 8 | `find_char_in_buffer` | valid non-null buffer with `size == 0` | [x] |
| 9 | `find_char_in_buffer` | target occurs within searched bytes; first occurrence at start, middle, or end | [x] |
| 10 | `find_char_in_buffer` | target absent from searched bytes, including an occurrence just beyond `size` | [x] |
| 11 | `find_char_in_buffer` | arbitrary byte targets, including NUL and values with the high bit set | [x] |
| 12 | `create_buffer` | valid empty NUL-terminated string | [x] |
| 13 | `create_buffer` | valid non-empty NUL-terminated string of varied lengths | [x] |
| 14 | `validate_uint16_range` | lower boundary `value == 0` | [x] |
| 15 | `validate_uint16_range` | interior `0 < value < UINT16_MAX` | [x] |
| 16 | `validate_uint16_range` | upper boundary `value == UINT16_MAX` | [x] |
| 17 | `apply_operation` | non-null external callback and arbitrary `int` argument | [x] |
| 18 | `charinbuf` | mode 0 with `value == 0`; `opt1`/`opt2` ignored | [x] |
| 19 | `charinbuf` | mode 0 with `0 < value < UINT16_MAX`; `opt1`/`opt2` ignored | [x] |
| 20 | `charinbuf` | mode 0 with `value == UINT16_MAX`; `opt1`/`opt2` ignored | [x] |
| 21 | `charinbuf` | mode 1; all remaining arguments ignored; fixed empty/non-empty checks | [x] |
| 22 | `charinbuf` | mode 2 with successful fixed-string allocation; remaining arguments ignored | [x] |
| 23 | `charinbuf` | mode 3 with reset/increment/multiply/decrement pipeline and representable intermediates | [x] |
| 24 | `charinbuf` | mode 4 with successful allocation and target present in fixed buffer | [x] |
| 25 | `charinbuf` | mode 4 with fixed-string allocation failure; initialized result remains `0` | [x] |

Cargo declares no features, so the complete feature matrix consists of the
single no-feature configuration (default and `--no-default-features` are
equivalent).
