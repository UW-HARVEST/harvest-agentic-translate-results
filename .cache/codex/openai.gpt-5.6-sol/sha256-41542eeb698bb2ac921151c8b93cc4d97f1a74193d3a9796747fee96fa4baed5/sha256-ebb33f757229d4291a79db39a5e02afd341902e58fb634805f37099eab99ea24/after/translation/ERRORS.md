# Error-surface table

Mechanical scan covered every `return`, `if`, loop bound, `NULL`, `assert`,
range expression, and special-value constant in `c_src/src/lib.c` and
`c_src/include/lib.h`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `find_value_in_buffer` | `memchr(buffer, (char)search_val, size) == NULL`: the target byte is absent from the first `size` bytes (including `size == 0`) | returns `-1` | [x] |

No `assert`, error enum, explicit null-pointer rejection, explicit range
rejection, or other error-return statement exists. In particular, `b == 0` in
`calculate_with_doubles` returns a numeric zero, and `size <= 0` in
`create_numeric_buffer` performs zero iterations; neither is an error.

Generic FFI boundaries were also compared: null pointers with zero/negative
lengths, null pointers with positive lengths (same terminating signal in
isolated child processes), a 1,048,577-byte length, and floating-point values
one step beyond the `int` range. The API has no enum parameters.
