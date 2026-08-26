# Error Surface

Derived mechanically from every allocation check, null check, early error
return, and unsupported `switch` value in `c_src/src/lib.c`. The source has no
assertions, error enums, or explicit input min/max rejection checks. Bit masks
`0x1f` and `0x7` constrain stored fields rather than reject inputs.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| [x] 1 | `create_state` | `malloc(sizeof(ProcessState)) == NULL` | prints `Error: Failed to allocate memory for state\n`; returns `NULL` |
| [x] 2 | `create_state` | state allocation succeeds and `malloc(capacity) == NULL` | prints `Error: Failed to allocate buffer\n`; frees the state; returns `NULL` |
| [x] 3 | `destroy_state` | `state == NULL` | returns without accessing or freeing memory |
| [x] 4 | `destroy_state` | `state != NULL` and `state->buffer == NULL` | skips the buffer free, frees the state, and returns |
| [x] 5 | `process_buffer` | `state == NULL` | prints `Error: Null pointer in process_buffer\n`; returns `-1` |
| [x] 6 | `process_buffer` | `state != NULL` and `state->buffer == NULL` | prints `Error: Null pointer in process_buffer\n`; returns `-1` |
| [x] 7 | `update_flags` | `state == NULL` | returns without mutation |
| [x] 8 | `confuse_types` | `state == NULL` | returns `0` |
| [x] 9 | `confuse_types` | `operation < 0` or `operation > 3` | no `switch` arm runs; returns `0` without mutating the state |
| [x] 10 | `confusion` | its `create_state(param1, 128)` call returns `NULL` | returns `-1` before flag, buffer, or type processing |

Generic FFI boundaries to exercise in addition to the rows above:

- [x] null pointers for every pointer-taking entry point;
- [x] zero, one-byte, truncating, exact-fit, oversized, and allocation-failing
  capacities;
- [x] signed `char` boundary targets and zero/one/many buffer lengths;
- [x] `INT_MIN`, `INT_MAX`, and values around every masked/remainder range;
- [x] invalid `confuse_types` operation values immediately below and above
  `0..=3`.
