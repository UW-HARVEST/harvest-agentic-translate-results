# Error Surface

Mechanically derived from the `return NULL`, `return false`, range checks, and
negative result assignments in `c_src/src/lib.c`. Static helper failures are
observed through the exported `gotomach` entry point.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `is_valid_state` via `gotomach` | `state->status == 0` | `false`; `gotomach` rejects an initially false status as `-5` | [x] |
| 2 | `is_valid_state` via `gotomach` | `state->status != 0 && state->count >= state->capacity` during the loop | `false`; `gotomach` returns `-6` | [x] |
| 3 | `init_processor` via `gotomach` | allocation of `ProcessorState` returns `NULL` | `NULL`; `gotomach` returns `-3` | [x] |
| 4 | `init_processor` via `gotomach` | allocation of `capacity * sizeof(int)` for `results` returns `NULL` | frees `state`, returns `NULL`; `gotomach` returns `-3` | [x] |
| 5 | `gotomach` | `iterations < 0` | `-1` | [x] |
| 6 | `gotomach` | `iterations > UINT16_MAX` (`65535`) | `-1` | [x] |
| 7 | `gotomach` | `seed < 0` | `-2` | [x] |
| 8 | `gotomach` | `seed > UINT16_MAX` (`65535`) | `-2` | [x] |
| 9 | `gotomach` | allocation of `iterations * sizeof(int)` for `temp_buffer` returns `NULL` | `-4` | [x] |
| 10 | `gotomach` | `check_char_flag(state->status)` is false | `-5` | [x] |
| 11 | `gotomach` | `is_valid_state(state)` is false during processing | `-6` | [x] |

There are no assertions, pointer parameters, enums, or independent public
length parameters in the C API. `mode` is an `int`; every out-of-range value is
accepted and selects the default operation rather than returning an error.
