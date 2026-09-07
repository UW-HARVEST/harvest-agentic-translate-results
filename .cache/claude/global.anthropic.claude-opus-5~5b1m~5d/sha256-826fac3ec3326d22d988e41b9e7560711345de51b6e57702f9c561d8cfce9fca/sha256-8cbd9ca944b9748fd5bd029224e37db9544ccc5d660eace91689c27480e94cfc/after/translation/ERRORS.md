# ERRORS.md — Error-surface table (Phase A, gates Phase C)

Mechanically derived by grepping `c_src/src/lib.c` for every `return`, every
`result = -N; goto cleanup;`, every `NULL` check, every range check and every
min/max constant. There are no `assert`s and no error enums in this library;
rejection is signalled purely by a negative `int` return from `gotomach`, and by
`NULL` from the `static` allocators.

## `gotomach` rejection branches

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `gotomach` | `iterations < 0` (e.g. `-1`, `INT_MIN`) | returns `-1`, prints `[ERROR] Invalid iteration count` |
| 2 | `gotomach` | `iterations > UINT16_MAX` i.e. `> 65535` (e.g. `65536`, `INT_MAX`) | returns `-1`, prints `[ERROR] Invalid iteration count` |
| 3 | `gotomach` | `seed < 0` (e.g. `-1`, `INT_MIN`), with `iterations` valid | returns `-2`, prints `[ERROR] Invalid seed value` |
| 4 | `gotomach` | `seed > UINT16_MAX` i.e. `> 65535` (e.g. `65536`, `INT_MAX`), with `iterations` valid | returns `-2`, prints `[ERROR] Invalid seed value` |
| 5 | `gotomach` | `init_processor` returns `NULL` (`malloc(sizeof(ProcessorState))` or `malloc(capacity*sizeof(int))` fails) | returns `-3`, prints `[ERROR] Failed to initialize processor` — UNREACHABLE for any accepted `iterations` (max allocation is `65535*4` = 256 KiB), so the observable contract is "never returns `-3`" |
| 6 | `gotomach` | `malloc(iterations * sizeof(int))` for `temp_buffer` returns `NULL` | returns `-4`, prints `[ERROR] Failed to allocate temporary buffer` — likewise UNREACHABLE; observable contract is "never returns `-4`". NOTE: `malloc(0)` returns non-`NULL` on glibc, so `iterations == 0` does **not** trigger this |
| 7 | `gotomach` | `check_char_flag(state->status)` false, i.e. `state->status == 0` | returns `-5`, prints `[ERROR] Invalid state status` — UNREACHABLE because `init_processor` unconditionally sets `status = 1`; observable contract is "never returns `-5`" |
| 8 | `gotomach` | `is_valid_state(state)` false inside the loop, i.e. `status == 0` **or** `count >= capacity` | returns `-6`, prints `[ERROR] State became invalid during processing` — UNREACHABLE because `count` grows by at most 1 per iteration and `capacity == iterations`, so `count <= i < capacity`; observable contract is "never returns `-6`" |

## Non-error (warning) branches that must still match byte-for-byte

| # | function | trigger | expected C behaviour |
|---|----------|---------|----------------------|
| 9  | `gotomach` | `mode` not in `{0,1,2}` — i.e. any out-of-range "enum" value crossing the FFI boundary (`-1`, `3`, `INT_MIN`, `INT_MAX`) | prints `[WARNING] Invalid mode, using default`, falls back to `process_value`, then proceeds normally (no error return) |
| 10 | `gotomach` | `state->count >= UINT16_MAX` at the end of an iteration | prints `[WARNING] Reached maximum count`, `break`s out of the loop **without** setting an error, then still sums `results` and returns the sum. Reachable only with `iterations == 65535` and a `threshold` high enough that every element is appended |

## Ordering / precedence facts that the tests must pin

* The `iterations` check runs **before** the `seed` check: for
  `iterations = -1, seed = -1` the C returns `-1` (not `-2`).
* Both range checks run **before** the `mode` switch: an invalid `mode`
  combined with an invalid `iterations`/`seed` prints **only** the
  `[ERROR]` line, never `[WARNING] Invalid mode, using default`.
* `threshold` is **never** validated — every `int` value is legal, including
  `INT_MIN` (nothing is ever appended, sum is `0`) and `INT_MAX` (everything is
  appended).
* On every path, `[INFO] Starting gotomach function` is printed first, and the
  `cleanup:` label is always reached (no leak, no early `return`).

## Generic FFI boundary cases (not distinct C branches, covered anyway)

| case | why it is safe / what is asserted |
|------|-----------------------------------|
| null pointers | `gotomach` takes no pointer arguments; `process_value`/`double_value`/`triple_value` take a `void *unused_context` that is `(void)`-cast away, so `NULL` and a dangling non-null pointer must both be accepted and ignored |
| zero length | `iterations == 0`: valid, loop body never runs, returns `0` |
| oversized length | `iterations == 65536` → `-1` (row 2); `iterations == 65535` → the largest accepted workload |
| one past valid range | `iterations`/`seed` at `-1`, `0`, `65535`, `65536` |
| out-of-range enum | `mode` ∈ {`INT_MIN`, `-1`, `3`, `4`, `INT_MAX`} (row 9) |
| signed overflow in the helpers | `process_value(INT_MAX,..)`, `double_value(INT_MAX,..)`, `triple_value(INT_MAX,..)` — C signed overflow; the Rust uses `wrapping_*`, which must match what the compiled C actually does |

## Phase C result

All 10 rows plus the precedence and generic-boundary cases PASS. Row → test
mapping (`translation/tests/phase_c_errors.rs`):

| row(s) | test |
|--------|------|
| 1  | `err_row01_iterations_negative_returns_minus1` |
| 2  | `err_row02_iterations_above_u16max_returns_minus1` |
| 3  | `err_row03_seed_negative_returns_minus2` |
| 4  | `err_row04_seed_above_u16max_returns_minus2` |
| 5, 6 | `err_row05_row06_allocation_failures_never_observed` |
| 7  | `err_row07_invalid_state_status_minus5_never_observed` |
| 8  | `err_row08_state_became_invalid_minus6_never_observed` |
| 9  | `err_row09_out_of_range_mode_falls_back_to_process_value` |
| 10 | `err_row10_count_cap_warns_and_breaks_without_error` |
| precedence | `err_precedence_iterations_check_before_seed_check`, `err_precedence_range_checks_before_mode_switch` |
| threshold never validated | `err_threshold_is_never_validated` |
| null / arbitrary pointers | `err_generic_helper_pointer_arguments` |
| zero & oversized lengths | `err_generic_zero_and_oversized_lengths` |
| one step past range | `err_generic_one_step_past_documented_ranges` |
| out-of-range enum values | `err_generic_out_of_range_enum_values_exhaustive_near_valid` |

Each test asserts the SAME specific code from both implementations (`-1`, `-2`,
or "never `-3`/`-4`/`-5`/`-6`"), not merely that both failed. Rows 5-8 are the
unreachable-by-construction branches; their tests pin the observable contract
(the code is never produced) across ~20 000 randomized accepted inputs each.
