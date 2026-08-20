# ERRORS.md — Error-surface table (Phase A → tested in Phase C)

Mechanically derived from every rejection / error-return / early-exit site in
`c_src/src/lib.c`. `grep -n 'return\|goto\|if (!' c_src/src/lib.c` was used as
the starting point; one row per **distinct** rejection branch.

`UINT16_MAX` == 65535. The `gotomach` guards are evaluated **in source order**,
so `iterations` is validated before `seed` (row 9 pins that precedence).

| #  | function | trigger (exact invalid input/condition) | expected C result | test |
|----|----------|------------------------------------------|-------------------|------|
| 1  | `gotomach` | `iterations < 0` (line 114 `iterations < 0`) → `LOG_MSG(ERROR,"Invalid iteration count")` | returns `-1`, stdout `[INFO]…` + `[ERROR] Invalid iteration count` | `err_01_iterations_negative` |
| 2  | `gotomach` | `iterations > UINT16_MAX` (line 114) i.e. `>= 65536` | returns `-1`, same log | `err_02_iterations_too_large` |
| 3  | `gotomach` | `seed < 0` (line 120) → `LOG_MSG(ERROR,"Invalid seed value")` | returns `-2` | `err_03_seed_negative` |
| 4  | `gotomach` | `seed > UINT16_MAX` (line 120) i.e. `>= 65536` | returns `-2` | `err_04_seed_too_large` |
| 5  | `gotomach` | `init_processor()` returned `NULL` (line 143) → `result = -3` | returns `-3` | `err_05_to_08_unreachable_sentinels_never_observed` (unreachable: see notes) |
| 6  | `gotomach` | `malloc(iterations*4)` for `temp_buffer` returned `NULL` (line 150) → `result = -4` | returns `-4` | `err_05_to_08_unreachable_sentinels_never_observed` (unreachable: see notes) |
| 7  | `gotomach` | `check_char_flag(state->status)` false (line 156) → `result = -5` | returns `-5` | `err_05_to_08_unreachable_sentinels_never_observed` (unreachable: `status` is set to 1 by `init_processor`) |
| 8  | `gotomach` | `is_valid_state(state)` false inside the loop (line 164) → `result = -6` | returns `-6` | `err_05_to_08_unreachable_sentinels_never_observed` (unreachable: `count <= i < capacity`) |
| 9  | `gotomach` | BOTH `iterations` and `seed` out of range → first guard wins | returns `-1` (never `-2`) | `err_09_precedence_iterations_before_seed` |
| 10 | `gotomach` | `mode` matches no `case` (`switch` `default:`, line 136) — NOT an error: warns and falls back to `process_value` | returns the normal sum; stdout contains `[WARNING] Invalid mode, using default` | `err_10_invalid_mode_is_not_an_error` |
| 11 | `gotomach` | `state->count >= UINT16_MAX` inside loop (line 178) → `LOG_MSG(WARNING,"Reached maximum count")` + `break` (early loop termination, not an error) | returns sum of first 65535 accepted values; stdout contains `[WARNING] Reached maximum count` | `err_11_reached_maximum_count` |
| 12 | `init_processor` | `malloc(sizeof(ProcessorState))` == NULL (line 79) | `return NULL` → propagates to row 5 | covered by row 5 |
| 13 | `init_processor` | `state->results = malloc(...)` == NULL (line 84) → `free(state)` | `return NULL` → propagates to row 5 | covered by row 5 |
| 14 | `cleanup_processor` | `state == NULL` (line 98) | no-op, no free, no crash | `err_14_cleanup_null_state` (exercised by rows 1–4 which reach `cleanup:` with `state == NULL`) |
| 15 | `cleanup_processor` | `state->results == NULL` (line 99) | skips `free(results)`, still frees `state` | covered by row 13 (only reachable when `init_processor` bails out) |
| 16 | `gotomach` cleanup | `temp_buffer == NULL` at `cleanup:` (line 192) | skips `free(temp_buffer)` | `err_14_cleanup_null_state` (rows 1–5 all reach cleanup with a NULL `temp_buffer`) |
| 17 | `is_valid_state` | `state->status == 0` (line 49 falsy branch) | `return false` | covered by row 8 / row 7 (same unreachable state) |

## Generic FFI boundary cases (mandated even though not in the C table)

| #  | case | expected |
|----|------|----------|
| G1 | `mode` = out-of-range "enum" values across FFI: `-1`, `3`, `INT_MIN`, `INT_MAX`, random large ±ints. C `switch(int)` accepts any `int`; `default:` branch must be taken identically by Rust. | identical return + identical log |
| G2 | `iterations` = boundary ladder `-1, 0, 1, 65534, 65535, 65536`, and `INT_MIN`, `INT_MAX` | `-1` outside `[0,65535]`, normal otherwise |
| G3 | `seed` = boundary ladder `-1, 0, 1, 65534, 65535, 65536`, and `INT_MIN`, `INT_MAX` | `-2` outside `[0,65535]`, normal otherwise |
| G4 | `threshold` = `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX`, and values exactly equal to a produced element (strict `<` boundary) | identical sums |
| G5 | `iterations == 0` → `malloc(0)`; glibc returns a non-NULL unique block so the `-3`/`-4` paths are **not** taken and the empty sum `0` is returned | `0` from both |
| G6 | `process_value` / `double_value` / `triple_value` called directly with `INT_MIN`, `INT_MAX`, `INT_MAX-9`, `0`, `±1` (signed-overflow wrap-around region) and with a **non-NULL garbage** `unused_context` pointer and arbitrary `unused_param` | identical `int` result; the unused params must be ignored by both |
| G7 | NULL pointers: the public API takes **no pointer parameters**, so the only pointer input is `void *unused_context` of the three `operation_fn`s. Both NULL and non-NULL must be accepted and ignored. | identical results |

## Notes on the "unreachable" rows (5, 6, 7, 8, 12, 13, 15, 17)

These branches are guarded by `malloc` failure or by internal invariants that
`init_processor` establishes (`status = 1`, `count = 0`, `capacity = iterations`)
and that the loop preserves (`count <= i < iterations == capacity`). They cannot
be reached through the public API with any `int` argument tuple. The Phase C
tests therefore assert the **observable consequence**: for every input in a
saturating sweep of the whole valid domain plus the boundary ladders, neither
implementation ever returns `-3`, `-4`, `-5` or `-6`, and both agree on the
value they *do* return. That is the strongest differential statement available
without an allocator-fault injector, and it pins the Rust code to the same
unreachability (a Rust translation that erroneously produced `-5`/`-6` would be
caught).

## Phase C result — every row has a passing differential test

| row(s) | test in `tests/phase_c_errors.rs` | status |
|--------|-----------------------------------|--------|
| 1  | `err_01_iterations_negative` (207 inputs incl. `INT_MIN`) | [x] |
| 2  | `err_02_iterations_too_large` (206 inputs incl. `INT_MAX`) | [x] |
| 3  | `err_03_seed_negative` (207 inputs) | [x] |
| 4  | `err_04_seed_too_large` (206 inputs) | [x] |
| 5, 6, 7, 8, 12, 13, 15, 17 | `err_05_to_08_unreachable_sentinels_never_observed` (~8 900 inputs; asserts both impls agree AND that neither ever yields `-3/-4/-5/-6`) | [x] |
| 9  | `err_09_precedence_iterations_before_seed` | [x] |
| 10 | `err_10_invalid_mode_is_not_an_error` | [x] |
| 11 | `err_11_reached_maximum_count` (+ negative control at `iterations = 65534`) | [x] |
| 14, 16 | `err_14_cleanup_null_state` (2 000 calls that reach `cleanup:` with `state == NULL` and `temp_buffer == NULL`) | [x] |
| G1 | `g1_mode_out_of_range_enum_values` (517 mode values × 4 thresholds — out-of-range enum values crossing FFI) | [x] |
| G2, G3 | `g2_g3_iterations_and_seed_ladders` (15 × 15 ladder cross product) | [x] |
| G4 | `g4_threshold_ladder` (27 thresholds × 4 modes × 9 seeds × 6 iteration counts) | [x] |
| G5 | `g5_zero_iterations_malloc_zero` (+ 3 000-call bulk pass) | [x] |
| G6, G7 | `g6_g7_ops_ignore_unused_params_and_pointers` (319 values × 7 pointers incl. `NULL`, `0x1`, `usize::MAX`, for all three ops) | [x] |

```
running 14 tests
...
test result: ok. 14 passed; 0 failed
```

Every test asserts the **same sentinel value** (`-1`, `-2`, `0`, …), not merely
"both failed", and additionally asserts the two implementations produced
byte-identical log output.

### Mutation evidence that these tests are not vacuous

| injected bug in the Rust | Phase C result |
|--------------------------|----------------|
| `-1` changed to `-11` (bad `iterations`) | 5 tests FAILED |
| `-2` changed to `-1` (bad `seed`) | 4 tests FAILED |
| `mode == 3` routed to `double_value` | 4 tests FAILED |
| `[INFO] Starting gotomach function` removed | 12 tests FAILED |
| `malloc(0)` treated as failure (`-3`) | caught by Phase B/`g5` |
| guard order swapped | caught (`err_09` / row 51) |
