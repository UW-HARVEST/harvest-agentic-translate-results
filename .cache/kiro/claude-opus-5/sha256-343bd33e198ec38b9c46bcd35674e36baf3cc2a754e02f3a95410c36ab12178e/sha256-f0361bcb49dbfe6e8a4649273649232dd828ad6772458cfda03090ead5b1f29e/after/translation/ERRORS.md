# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return`,
`result = -N`, `goto cleanup`, `NULL` check, range check and min/max constant.
There are **no** `assert`s and **no** error enums in the C source; every
rejection is either a negative `int` return code from `gotomach` or a `NULL`
return from the static allocator helper.

Constants that bound the input domain: `UINT16_MAX` = 65535 (from
`<stdint.h>`), used for both the `iterations` and the `seed` range checks and
again as the loop's max-count cutoff.

## Rows

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `gotomach` | `iterations < 0` (line 114, left disjunct) | returns `-1`; stdout `[INFO] Starting…` + `[ERROR] Invalid iteration count` | `err_row1_iterations_negative` | [x] |
| 2 | `gotomach` | `iterations > UINT16_MAX`, i.e. `iterations >= 65536` (line 114, right disjunct) | returns `-1`; same stdout as row 1 | `err_row2_iterations_too_large` | [x] |
| 3 | `gotomach` | `seed < 0` (line 120, left disjunct), with `iterations` valid | returns `-2`; stdout `[INFO] …` + `[ERROR] Invalid seed value` | `err_row3_seed_negative` | [x] |
| 4 | `gotomach` | `seed > UINT16_MAX`, i.e. `seed >= 65536` (line 120, right disjunct), with `iterations` valid | returns `-2`; same stdout as row 3 | `err_row4_seed_too_large` | [x] |
| 5 | `gotomach` | `init_processor(...) == NULL` (line 143) — reached only if `malloc(sizeof(ProcessorState))` **or** `malloc(capacity*4)` fails inside `init_processor` | returns `-3`; stdout `[INFO] …` + `[ERROR] Failed to initialize processor` | `err_row5_row6_alloc_failure_unreachable` (unreachable-by-construction; see note A) | [x] |
| 6 | `gotomach` | `malloc(iterations*sizeof(int)) == NULL` for `temp_buffer` (line 150) | returns `-4`; stdout `[INFO] …` + `[ERROR] Failed to allocate temporary buffer` | `err_row5_row6_alloc_failure_unreachable` (unreachable-by-construction; see note A) | [x] |
| 7 | `gotomach` | `check_char_flag(state->status)` is false, i.e. `state->status == 0` (line 156) | returns `-5`; stdout `[INFO] …` + `[ERROR] Invalid state status` | `err_row7_status_flag_unreachable` (unreachable-by-construction; see note B) | [x] |
| 8 | `gotomach` | `!is_valid_state(state)` inside the loop (line 164), i.e. `state->status == 0` **or** `state->count >= state->capacity` | returns `-6`; stdout `[INFO] …` + `[ERROR] State became invalid during processing` | `err_row8_state_invalid_unreachable` (unreachable-by-construction; see note C) | [x] |
| 9 | `init_processor` (static) | `malloc(sizeof(ProcessorState))` returns `NULL` (line 79) | returns `NULL`, no leak | folded into row 5 | [x] |
| 10 | `init_processor` (static) | `malloc(capacity * sizeof(int))` returns `NULL` (line 84) | `free(state)`, returns `NULL` | folded into row 5 | [x] |

### Non-error rejection-adjacent branches (verified in `CONFIGS.md`, listed here for completeness)

| # | function | trigger | C behaviour |
|---|----------|---------|-------------|
| W1 | `gotomach` | `mode` not in {0,1,2} (`default:` at line 136) — this is *not* an error; it logs a warning and silently falls back to `process_value` | `[WARNING] Invalid mode, using default`, then proceeds normally |
| W2 | `gotomach` | `state->count >= UINT16_MAX` at end of a loop iteration (line 178) | `[WARNING] Reached maximum count`, `break` out of the loop **without** running `i++`, then falls through to the summation (NOT an error return) |

### Notes on the unreachable rows

Rows 5–8 are real branches in the C source, so they get rows, but no input to
the public API can trigger them on a normally-functioning allocator. Proving
that is part of verification, so each has a test that (a) documents the
argument and (b) pins the closest reachable configuration to identical
behaviour in C and Rust:

* **Note A (rows 5, 6, 9, 10).** After the row-1/row-2 range check,
  `0 <= iterations <= 65535`. The three allocations are therefore at most
  `sizeof(ProcessorState)` = 40 bytes and `65535 * 4` = 262 140 bytes. These
  cannot fail on the test host. The test drives the largest allocation the API
  permits (`iterations == 65535`) and asserts C and Rust agree, and also covers
  `iterations == 0`, where glibc `malloc(0)` returns a unique non-`NULL`
  pointer — the case where a naive translation could wrongly take the `-3`/`-4`
  branch. The Rust code calls the same `malloc`, so it inherits the identical
  outcome.
* **Note B (row 7).** `init_processor` unconditionally sets `state->status = 1`
  right before returning, and nothing mutates it between there and line 156, so
  `check_char_flag` is always true. The test asserts neither library ever
  returns `-5` across the whole reachable input space sampled in Phase B/C.
* **Note C (row 8).** `state->capacity == iterations` and `state->count` is
  incremented at most once per loop iteration, so on entry to iteration `i` we
  have `count <= i < iterations == capacity`; combined with note B,
  `is_valid_state` is always true. The test asserts neither library ever returns
  `-6`, including the worst case (`threshold` large enough that *every*
  iteration stores a result, at `iterations == 65535`).

## Generic FFI boundary cases (required even though not in the table above)

| case | covered by |
|------|-----------|
| out-of-range "enum" value for `mode` (C `switch` on an `int` accepts any value: `-1`, `3`, `INT_MIN`, `INT_MAX`, random) | `err_generic_mode_out_of_range` |
| one step past each documented range: `iterations` ∈ {-1, 0, 65535, 65536}, `seed` ∈ {-1, 0, 65535, 65536} | `err_generic_off_by_one_boundaries` |
| both range checks violated at once (short-circuit order: `iterations` is checked first, so `-1` wins over `-2`) | `err_generic_both_ranges_invalid` |
| `INT_MIN` / `INT_MAX` in every one of the four `int` parameters | `err_generic_extreme_ints` |
| `threshold` extremes (`INT_MIN` ⇒ nothing ever stored ⇒ sum 0; `INT_MAX` ⇒ everything stored) and `threshold` exactly equal to a produced value (comparison is strict `<`) | `err_generic_threshold_extremes` |
| `NULL` pointer passed as the `void *unused_context` of the three operation callbacks (the only pointer parameter in the exported ABI), plus non-`NULL` garbage | `err_generic_op_null_and_garbage_context` |
| signed-overflow inputs to the operation callbacks (`INT_MAX`, `INT_MIN`) | `err_generic_op_overflow` |

There are no pointer or length parameters on `gotomach`, so "null pointer" and
"oversized length" reduce to the `void *unused_context` callback argument and
the `iterations` range check respectively.

## Final result

All 10 rows and all 7 generic-boundary cases have a passing differential test in
`tests/errors.rs` (14 tests). Every test pins the **exact** sentinel from the C
source (`-1`, `-2`, `0`, …) rather than "both failed somehow", and also compares
the byte-exact stdout, so a divergence in either the code or the log text fails.

Confirmation that these assertions can actually fail: `mutation_check.sh`
injected `result = -1 → -7`, `result = -2 → -1`, `>` → `>=` on both range
checks, `< 0` → `<= 0`, a swapped range-check order, and a corrupted error
message string. All six were caught by this file's tests.
