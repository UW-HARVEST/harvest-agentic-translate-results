# Error surface

Rows 1-3 are the complete set of explicit error returns found with
`rg -n 'return NULL|return -1' ../c_src/src/lib.c`. Rows 4-9 record explicit
invalid-input and generic FFI boundary behavior required for Phase C. The C
source has no assertions, error enums, named min/max constants, or explicit
input range checks.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `create_buffer` | `malloc(sizeof(StringBuffer))` returns `NULL` | returns `NULL` | [x] |
| 2 | `create_buffer` | struct allocation succeeds, then `malloc(initial_capacity)` returns `NULL` | frees the struct and returns `NULL` | [x] |
| 3 | `append_to_buffer` | growth is required and `realloc(buffer->data, required_capacity * 2)` returns `NULL` | returns `-1`; buffer fields and existing bytes remain unchanged | [x] |
| 4 | `perform_operation` | operation is `"divide"` and `b == 0` | returns `0` | [x] |
| 5 | `perform_operation` | operation string is not one of `add`, `subtract`, `multiply`, `divide` | returns `0` | [x] |
| 6 | `get_operation_name` | operation code is outside `0..=3`, including one-step-past and arbitrary `int` values | returns bytes `"unknown\0"` | [x] |
| 7 | `append_to_buffer` | `buffer == NULL` with a valid string | process terminates with the same signal for C and Rust after dereferencing `buffer` | [x] |
| 8 | `append_to_buffer` | `str == NULL` with a valid buffer | process terminates with the same signal for C and Rust in `strlen` | [x] |
| 9 | `perform_operation` | `operation == NULL` | process terminates with the same signal for C and Rust in `strcmp` | [x] |

`destroy_buffer(NULL)` is an explicitly accepted no-op and is listed on the
valid configuration surface. A zero initial capacity and oversized/negative
capacity are also exercised as boundary configurations; the latter induces
row 2 on the target libc.
