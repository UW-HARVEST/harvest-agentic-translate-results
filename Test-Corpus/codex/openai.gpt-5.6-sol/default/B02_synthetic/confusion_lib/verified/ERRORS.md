# Error-surface table

Mechanically derived from every `if (... == NULL)`, error return, and
out-of-range `switch` path in `c_src/src/lib.c`. There are no assertions,
error enums, explicit numeric range checks, or min/max constants in this
source.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] E01 | `create_state` | `malloc(sizeof(ProcessState)) == NULL` | prints `Error: Failed to allocate memory for state\n`; returns `NULL` |
| [x] E02 | `create_state` | state allocation succeeds, then `malloc(capacity) == NULL` | prints `Error: Failed to allocate buffer\n`; frees the state; returns `NULL` |
| [x] E03 | `destroy_state` | `state == NULL` | no output and no operation |
| [x] E04 | `destroy_state` | `state != NULL && state->buffer == NULL` | frees only the state; no output |
| [x] E05 | `process_buffer` | `state == NULL` | prints `Error: Null pointer in process_buffer\n`; returns `-1` |
| [x] E06 | `process_buffer` | `state != NULL && state->buffer == NULL` | prints `Error: Null pointer in process_buffer\n`; returns `-1` |
| [x] E07 | `update_flags` | `state == NULL` | returns immediately; no output |
| [x] E08 | `confuse_types` | `state == NULL` | returns `0`; no output |
| [x] E09 | `confuse_types` | `operation < 0 || operation > 3` (no matching `switch` case) | leaves state data unchanged; returns `0`; no output |
| [x] E10 | `confusion` | its internal `create_state(param1, 128)` returns `NULL` | returns `-1` after the four parameter debug lines and allocation error line |

Generic FFI boundary cases attached to these rows: all pointer-taking APIs are
called with null; `capacity` is tested at zero and with values too large for a
successful allocation; and `confuse_types` is tested with `-1` and `4`, the
two values immediately outside its handled operation range.
