# Error Surface

Mechanically derived from every `return NULL`, range check, allocation/null
check, status check, and invalid-state branch in `../c_src/src/lib.c`. There
are no assertions or error enums.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 [x] | `init_processor` via `gotomach` | First allocation, `malloc(sizeof(ProcessorState))`, returns `NULL` | Internal `NULL`; public result `-3` |
| 2 [x] | `init_processor` via `gotomach` | Second allocation, `malloc(capacity * sizeof(int))`, returns `NULL` | Frees state and returns internal `NULL`; public result `-3` |
| 3 [x] | `gotomach` | `iterations < 0` | `-1` |
| 4 [x] | `gotomach` | `iterations > UINT16_MAX` (`65535`) | `-1` |
| 5 [x] | `gotomach` | `seed < 0`, after a valid iteration count | `-2` |
| 6 [x] | `gotomach` | `seed > UINT16_MAX` (`65535`), after a valid iteration count | `-2` |
| 7 [x] | `gotomach` | `init_processor` returned `NULL` and `if (!state)` rejects it | `-3` |
| 8 [x] | `gotomach` | Temporary-buffer allocation returns `NULL` after state initialization | `-4` |
| 9 [x] | `gotomach` | `check_char_flag(state->status)` is false (`status == 0`) | `-5` |
| 10 [x] | `gotomach` | `is_valid_state(state)` is false in the loop (`status == 0` or `count >= capacity`) | `-6` |

`cleanup_processor` and the `cleanup` block also contain null checks, but those
checks guard optional deallocation and do not reject input or return an error.
