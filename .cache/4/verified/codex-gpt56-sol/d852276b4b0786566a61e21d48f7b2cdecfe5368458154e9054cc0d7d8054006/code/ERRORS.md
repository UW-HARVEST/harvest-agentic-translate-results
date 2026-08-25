# Error Surface

The rows below come from every allocation failure, null check, default
selection, and explicit rejected operation in `c_src/src/lib.c`. Conditions
that dereference an unchecked null pointer are listed separately as generic
FFI boundaries after the table.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `create_buffer` | allocation of the `StringBuffer` object returns `NULL` (`!buffer`) | returns `NULL` | [x] |
| 2 | `create_buffer` | object allocation succeeds, then `malloc(initial_capacity)` returns `NULL` (`!buffer->data`) | frees the object and returns `NULL` | [x] |
| 3 | `append_to_buffer` | `required_capacity > buffer->capacity` and `realloc` returns `NULL` (`!new_data`) | returns `-1`; original allocation, capacity, length, and bytes remain intact | [x] |
| 4 | `destroy_buffer` | `buffer == NULL` | returns normally without freeing anything | [x] |
| 5 | `destroy_buffer` | `buffer != NULL` and `buffer->data == NULL` | skips the data free, frees the object, and returns normally | [x] |
| 6 | `get_operation_name` | `op_code` is outside `0..=3` (the `default` switch arm) | returns a pointer to `"unknown\0"` | [x] |
| 7 | `perform_operation` | operation is `"divide"` and `b == 0` | returns `0` | [x] |
| 8 | `perform_operation` | operation is not exactly `"add"`, `"subtract"`, `"multiply"`, or `"divide"` | returns `0` | [x] |

Generic unchecked FFI boundaries that must also match:

| # | function | boundary | expected observed C behavior | status |
|---|----------|----------|------------------------------|--------|
| G1 | `append_to_buffer` | `buffer == NULL` with a valid string | child process terminates with `SIGSEGV` | [x] |
| G2 | `append_to_buffer` | valid buffer with `str == NULL` | child process terminates with `SIGSEGV` | [x] |
| G3 | `perform_operation` | `operation == NULL` | child process terminates with `SIGSEGV` | [x] |
| G4 | `create_buffer` | `initial_capacity == 0` | C and Rust return the same null/non-null classification | [x] |
| G5 | `create_buffer` | `initial_capacity == INT_MIN` (oversized after conversion to `size_t`) | returns `NULL` | [x] |
| G6 | `get_operation_name` | one past the documented range (`op_code == 4`) | returns `"unknown\0"` | [x] |
| G7 | `perform_operation` | `"divide"` with `a == INT_MIN` and `b == -1` | child process terminates with `SIGFPE` | [x] |

There are no C enum parameters and no explicit length parameters in this API.
