# Error Surface

This table is derived from every input range check, allocation null check, and
error-result assignment in `src/lib.c`. Private cleanup null checks are not
rejections and are therefore excluded. The two warning paths are accepted
configurations and appear in `CONFIGS.md`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E01 | `gotomach` | `iterations < 0 || iterations > UINT16_MAX` | [x] `-1`; logs `Invalid iteration count` |
| E02 | `gotomach` | valid `iterations`, then `seed < 0 || seed > UINT16_MAX` | [x] `-2`; logs `Invalid seed value` |
| E03 | `init_processor` → `gotomach` | first allocation, `malloc(sizeof(ProcessorState))`, returns `NULL` | [x] `init_processor` returns `NULL`; `gotomach` returns `-3` |
| E04 | `init_processor` → `gotomach` | state allocation succeeds, then `malloc(capacity * sizeof(int))` returns `NULL` | [x] frees state; `init_processor` returns `NULL`; `gotomach` returns `-3` |
| E05 | `gotomach` | processor initializes, then `malloc(iterations * sizeof(int))` for `temp_buffer` returns `NULL` | [x] `-4`; processor is cleaned up |
| E06 | `gotomach` | `state->status == 0` at the explicit pre-loop `check_char_flag` check | [x] `-5` |
| E07 | `gotomach` | during the loop, `state->status == 0`, so `is_valid_state(state)` is false | [x] `-6` |
| E08 | `gotomach` | during the loop, `state->count >= state->capacity`, so `is_valid_state(state)` is false | [x] `-6` |

Generic FFI boundaries mapped to rows/configurations:

- Null `unused_context` and non-null opaque `unused_context` are both valid for
  all three operation functions because C explicitly discards the pointer.
- Zero iterations is valid and covered by `CONFIGS.md`.
- `UINT16_MAX + 1`, negative lengths, and extreme `int` lengths map to E01.
- `UINT16_MAX + 1`, negative seeds, and extreme `int` seeds map to E02.
- Every integer outside modes `0`, `1`, and `2` is accepted and selects the
  default operation; these out-of-range enum-like values are covered by
  `CONFIGS.md`, including negative and extreme `int` values.
