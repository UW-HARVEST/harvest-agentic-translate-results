# ERRORS.md — Error-surface table (Phase A)

Mechanically derived from **every** rejection site in `c_src/src/lib.c`
(`grep -n 'return |result = -|goto |if (!|NULL|MAX'`). There are no `assert`s
and no `errno` use in the C source; every rejection is an integer return value.

Constants: `UINT16_MAX == 65535`. `sizeof(int) == 4`.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `gotomach` | `iterations < 0` (`lib.c:114`, first disjunct). e.g. `iterations = -1`, `-100`, `INT_MIN`. Logs `[ERROR] Invalid iteration count`. | returns `-1` |
| 2  | `gotomach` | `iterations > UINT16_MAX` (`lib.c:114`, second disjunct). e.g. `65536`, `70000`, `INT_MAX`. Logs `[ERROR] Invalid iteration count`. | returns `-1` |
| 3  | `gotomach` | `seed < 0` (`lib.c:120`, first disjunct) **with `iterations` already valid**. e.g. `seed = -1`, `INT_MIN`. Logs `[ERROR] Invalid seed value`. | returns `-2` |
| 4  | `gotomach` | `seed > UINT16_MAX` (`lib.c:120`, second disjunct) with valid `iterations`. e.g. `65536`, `INT_MAX`. Logs `[ERROR] Invalid seed value`. | returns `-2` |
| 5  | `gotomach` | **Check ordering**: both `iterations` and `seed` invalid at once. `iterations` is checked first, so the seed check is never reached. | returns `-1` (not `-2`) |
| 6  | `gotomach` | `init_processor` returned `NULL` because `malloc(sizeof(ProcessorState))` failed (`lib.c:78-81`). Logs `[ERROR] Failed to initialize processor`. | returns `-3` |
| 7  | `gotomach` | `init_processor` returned `NULL` because `malloc(capacity * sizeof(int))` failed (`lib.c:83-87`); the already-allocated state is `free`d first. Logs `[ERROR] Failed to initialize processor`. | returns `-3` |
| 8  | `gotomach` | `malloc(iterations * sizeof(int))` for `temp_buffer` failed (`lib.c:149-154`). Logs `[ERROR] Failed to allocate temporary buffer`. | returns `-4` |
| 9  | `gotomach` | `check_char_flag(state->status)` is false, i.e. `state->status == 0` (`lib.c:156`). **Statically unreachable**: `init_processor` unconditionally sets `status = 1` (`lib.c:92`) and nothing mutates it before the check. Logs `[ERROR] Invalid state status`. | returns `-5` (unreachable) |
| 10 | `gotomach` | `is_valid_state(state)` is false inside the loop (`lib.c:164`), i.e. `status == 0` **or** `count >= capacity`. **Statically unreachable**: `capacity == iterations`, `count` is incremented at most once per iteration so `count <= i < iterations == capacity`. Logs `[ERROR] State became invalid during processing`. | returns `-6` (unreachable) |
| 11 | `init_processor` | `malloc` for the struct fails → early `return NULL` (`lib.c:80`). | returns `NULL` (→ row 6) |
| 12 | `init_processor` | `malloc` for `results` fails → `free(state); return NULL` (`lib.c:85-86`). | returns `NULL` (→ row 7) |
| 13 | `cleanup_processor` | `state == NULL` → the `if (state)` guard (`lib.c:98`) makes it a no-op (this is the state on every `goto cleanup` from rows 1–5). | no crash, no output |
| 14 | `cleanup_processor` | `state->results == NULL` → the `if (state->results)` guard (`lib.c:99`) skips the inner `free`. | no crash, no output |
| 15 | `gotomach` | `temp_buffer == NULL` at `cleanup:` (`lib.c:192`) → the inner `free` is skipped (rows 1–7 all reach cleanup with `temp_buffer == NULL`). | no crash; returns the already-set `result` |

## Non-error rejections / warnings on the same surface

These are *not* error returns but they are branches the C takes on unusual
input, so they must match too (both the return value **and** the logged bytes).

| #  | function | trigger | expected C result |
|----|----------|---------|-------------------|
| 16 | `gotomach` | `mode` is **not** `0`, `1` or `2` (`switch` `default:`, `lib.c:136-139`). Any other `int` — `3`, `-1`, `INT_MIN`, `INT_MAX`. Logs `[WARNING] Invalid mode, using default` and falls back to `process_value`. | same numeric result as `mode == 0`, plus the extra warning line |
| 17 | `gotomach` | `state->count >= UINT16_MAX` at the end of a loop iteration (`lib.c:178-181`). Requires `iterations == 65535` **and** every produced value `< threshold`. Logs `[WARNING] Reached maximum count`, `break`s, then still sums. | the full sum (**not** an error); extra warning line |
| 18 | `gotomach` | `iterations == 0` → `malloc(0)`. glibc returns a **non-NULL** pointer, so this is *not* an error; loop body never runs, sum over 0 elements. | returns `0`, logs `[INFO] Processing completed successfully` |
| 19 | `gotomach` | `threshold <= min produced value` (e.g. `INT_MIN`, or `0` since all produced values are `>= 0`) → nothing is ever appended, `count == 0`. | returns `0` (success path) |

## Generic FFI boundary cases (required by Phase C even though not in the table)

| #  | entry point | trigger | expected C result |
|----|-------------|---------|-------------------|
| 20 | `gotomach` | boundary values **one step past / at** the documented range on every axis: `iterations ∈ {-1, 0, 65535, 65536}`, `seed ∈ {-1, 0, 65535, 65536}`. | `-1` / ok / ok / `-1` and `-2` / ok / ok / `-2` |
| 21 | `gotomach` | out-of-range "enum" value for `mode` crossing FFI: `mode ∈ {INT_MIN, -2147483647, -1, 3, 4, 99, INT_MAX}`. C `switch` on `int` accepts any value → `default:` branch. | `default:` behaviour (row 16) |
| 22 | `gotomach` | extreme `threshold`: `INT_MIN`, `INT_MAX`, `0`, `-1`, `1`. `temp_buffer[i] < threshold` is a plain signed compare, no clamping. | plain signed-compare behaviour |
| 23 | `process_value` / `double_value` / `triple_value` | `unused_context == NULL` (this is exactly how `gotomach` calls them, `lib.c:170`) and also a bogus non-null pointer. Both args are `(void)`-cast away. | value unaffected by args 2 and 3 |
| 24 | `process_value` | `value == INT_MAX` / `INT_MAX-9` / `INT_MIN` → `value + 10` signed overflow (C UB; gcc wraps). | wrapping add |
| 25 | `double_value` | `value == INT_MAX` / `INT_MIN` → `value * 2` signed overflow (C UB; gcc wraps). | wrapping multiply |
| 26 | `triple_value` | `value == INT_MAX` / `INT_MIN` / `INT_MAX/3+1` → `value * 3` signed overflow (C UB; gcc wraps). | wrapping multiply |

## Notes on rows 6–8, 11, 12 (allocation failure)

The public API takes only `int`s, so the largest allocation `gotomach` can ever
request is `65535 * 4 = 262140` bytes plus `sizeof(ProcessorState)`. These
`malloc`s cannot be made to fail through the FFI surface without an allocator
interposer. The tests therefore assert the *reachable* half of the contract:
that the maximum in-range request (`iterations == 65535`) **succeeds** in both
implementations (never `-3`/`-4`), and that `iterations == 0` (`malloc(0)`,
which glibc answers non-NULL) is likewise **not** treated as a failure — i.e.
the Rust `try_reserve_exact` path agrees with glibc `malloc` on both ends of
the range. Rows 9 and 10 are asserted the same way: both implementations must
*never* return `-5` or `-6` for any input.

## Status

| row | test | status |
|-----|------|--------|
| 1  | `err_01_iterations_negative`            | [x] pass |
| 2  | `err_02_iterations_above_uint16_max`    | [x] pass |
| 3  | `err_03_seed_negative`                  | [x] pass |
| 4  | `err_04_seed_above_uint16_max`          | [x] pass |
| 5  | `err_05_check_ordering_iterations_first`| [x] pass |
| 6  | `err_06_07_08_alloc_never_fails_in_range` | [x] pass |
| 7  | `err_06_07_08_alloc_never_fails_in_range` | [x] pass |
| 8  | `err_06_07_08_alloc_never_fails_in_range` | [x] pass |
| 9  | `err_09_10_status_and_state_never_invalid` | [x] pass |
| 10 | `err_09_10_status_and_state_never_invalid` | [x] pass |
| 11 | `err_06_07_08_alloc_never_fails_in_range` | [x] pass |
| 12 | `err_06_07_08_alloc_never_fails_in_range` | [x] pass |
| 13 | `err_13_14_15_cleanup_null_guards`      | [x] pass |
| 14 | `err_13_14_15_cleanup_null_guards`      | [x] pass |
| 15 | `err_13_14_15_cleanup_null_guards`      | [x] pass |
| 16 | `err_16_invalid_mode_default_branch`    | [x] pass |
| 17 | `err_17_reached_maximum_count`          | [x] pass |
| 18 | `err_18_zero_iterations_malloc_zero`    | [x] pass |
| 19 | `err_19_threshold_rejects_everything`   | [x] pass |
| 20 | `err_20_range_boundaries`               | [x] pass |
| 21 | `err_21_out_of_range_mode_enum`         | [x] pass |
| 22 | `err_22_extreme_thresholds`             | [x] pass |
| 23 | `err_23_op_ignores_extra_args`          | [x] pass |
| 24 | `err_24_process_value_overflow`          | [x] pass |
| 25 | `err_25_double_value_overflow`           | [x] pass |
| 26 | `err_26_triple_value_overflow`           | [x] pass |
