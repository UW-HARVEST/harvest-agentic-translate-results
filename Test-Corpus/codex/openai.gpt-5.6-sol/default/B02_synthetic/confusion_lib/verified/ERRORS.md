# Error Surface

Derived from every allocation check, null check, and error-returning statement
in `c_src/src/lib.c`. The C source contains no assertions, error enums, or
explicit numeric range checks.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `create_state` | `malloc(sizeof(ProcessState)) == NULL` | prints the state-allocation error and returns `NULL` | [x] |
| 2 | `create_state` | state allocation succeeds and `malloc(capacity) == NULL` | prints the buffer-allocation error, frees the state, and returns `NULL` | [x] |
| 3 | `create_state` | `capacity < 0`, converted by `malloc`/`snprintf` to an oversized `size_t` | buffer allocation fails and returns `NULL` as in row 2 | [x] |
| 4 | `create_state` | oversized positive capacity whose allocation fails | buffer allocation fails and returns `NULL` as in row 2 | [x] |
| 5 | `destroy_state` | `state == NULL` | returns normally without action | [x] |
| 6 | `destroy_state` | `state != NULL && state->buffer == NULL` | skips the buffer free, frees the state, and returns | [x] |
| 7 | `process_buffer` | `state == NULL` | prints the null-pointer error and returns `-1` | [x] |
| 8 | `process_buffer` | `state != NULL && state->buffer == NULL` | prints the null-pointer error and returns `-1` | [x] |
| 9 | `update_flags` | `state == NULL` | returns normally without action | [x] |
| 10 | `confuse_types` | `state == NULL`, for any operation value including out-of-range values | returns `0` | [x] |
| 11 | `confuse_types` | `state != NULL && operation` is outside `0..=3` (the `switch` has no matching case) | leaves state unchanged and returns `0` | [x] |
| 12 | `confusion` | its internal `create_state(param1, 128)` returns `NULL` | returns `-1` | [x] |

Generic FFI boundaries are represented explicitly above. There are no public
pointer-plus-length APIs and no C enum types. `capacity == 0` is not explicitly
rejected by C and is therefore covered as a valid configuration in
`CONFIGS.md`; negative and oversized capacities are rows 3 and 4.
