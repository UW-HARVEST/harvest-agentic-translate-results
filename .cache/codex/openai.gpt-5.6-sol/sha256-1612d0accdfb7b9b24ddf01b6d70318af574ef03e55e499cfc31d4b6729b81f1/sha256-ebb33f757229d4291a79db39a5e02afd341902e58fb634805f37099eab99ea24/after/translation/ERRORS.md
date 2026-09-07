# Error Surface

This table is derived from every `if`, `switch` default, null check, and
error/sentinel return in `c_src/src/lib.c`. The source contains no assertions,
error enums, named minimum/maximum constants, or explicit input range checks.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `create_buffer` | allocation of the `StringBuffer` object returns `NULL` | returns `NULL` | [x] |
| 2 | `create_buffer` | allocation of `data` for `initial_capacity` bytes returns `NULL` | frees the object and returns `NULL` | [x] |
| 3 | `append_to_buffer` | growth is required and `realloc(buffer->data, required_capacity * 2)` returns `NULL` | returns `-1`; existing buffer pointer, capacity, length, and bytes remain unchanged | [x] |
| 4 | `destroy_buffer` | `buffer == NULL` | no-op; returns normally | [x] |
| 5 | `destroy_buffer` | `buffer != NULL` and `buffer->data == NULL` | skips data free, frees the object, and returns normally | [x] |
| 6 | `get_operation_name` | `op_code` is not `0`, `1`, `2`, or `3` | returns pointer to `"unknown"` | [x] |
| 7 | `perform_operation` | `operation == "divide"` and `b == 0` | returns `0` | [x] |
| 8 | `perform_operation` | `operation` is not `"add"`, `"subtract"`, `"multiply"`, or `"divide"` | returns `0` | [x] |
| 9 | `append_to_buffer` | `buffer == NULL` | process faults while dereferencing the null pointer on the test platform | [x] |
| 10 | `append_to_buffer` | `str == NULL` | process faults in `strlen` on the test platform | [x] |
| 11 | `perform_operation` | `operation == NULL` | process faults in `strcmp` on the test platform | [x] |
| 12 | `perform_operation` | `operation == "divide"`, `a == INT_MIN`, and `b == -1` | process terminates with `SIGFPE` on the test platform | [x] |
| 13 | `buffapp` | the wrapped intermediate sum is `INT_MIN` and the wrapped intermediate product is `-1` (for example `buffapp(0, 1073741825, 0, 1073741823)`) | final division terminates with `SIGFPE` on the test platform | [x] |

Rows 9-13 are generic FFI boundary cases required by Phase C. The C source
does not reject them explicitly; the platform-observed termination signal is
therefore compared exactly in an isolated child process.
