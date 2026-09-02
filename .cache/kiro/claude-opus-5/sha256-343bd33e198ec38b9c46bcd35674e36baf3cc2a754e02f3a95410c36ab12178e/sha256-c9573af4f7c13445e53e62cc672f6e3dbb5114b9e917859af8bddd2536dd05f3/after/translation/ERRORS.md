# ERRORS.md — Error-surface table (Phase A → gates Phase C)

Mechanically derived from `c_src/src/lib.c`. Every `return`-of-a-negative-code,
every `return NULL`, every explicit range/null check, and every min/max constant
in the C source is one row below. There are **no** `assert`s, no `errno` use, no
error enums, and no pointer parameters in the public API (`gotomach` takes four
`int`s), so the surface is exactly the checks listed here.

Constants the C code compares against: `UINT16_MAX` (65535) for `iterations`
and `seed`; `UINT16_MAX` again as the `state->count` ceiling; `1000` as the
`current_value` modulus; `0` as the implicit lower bound for `iterations`/`seed`.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| E1 | `gotomach` | `iterations < 0` (`if (iterations < 0 \|\| iterations > UINT16_MAX)`) | returns `-1`; prints `[ERROR] Invalid iteration count`; nothing allocated | [x] |
| E2 | `gotomach` | `iterations > UINT16_MAX` i.e. `>= 65536` (same `if`) | returns `-1`; prints `[ERROR] Invalid iteration count` | [x] |
| E3 | `gotomach` | `seed < 0` (`if (seed < 0 \|\| seed > UINT16_MAX)`), `iterations` valid | returns `-2`; prints `[ERROR] Invalid seed value` | [x] |
| E4 | `gotomach` | `seed > UINT16_MAX` i.e. `>= 65536` (same `if`), `iterations` valid | returns `-2`; prints `[ERROR] Invalid seed value` | [x] |
| E5 | `gotomach` | `init_processor` returns `NULL` because `malloc(sizeof(ProcessorState))` failed | returns `-3`; prints `[ERROR] Failed to initialize processor` | [x] |
| E6 | `gotomach` | `init_processor` returns `NULL` because `malloc(capacity * sizeof(int))` failed (state `malloc` succeeded and is `free`d again) | returns `-3`; prints `[ERROR] Failed to initialize processor` | [x] |
| E7 | `gotomach` | `temp_buffer = malloc(iterations * sizeof(int))` returned `NULL` | returns `-4`; prints `[ERROR] Failed to allocate temporary buffer` | [x] |
| E8 | `gotomach` | `!check_char_flag(state->status)` — `state->status == 0`. **Statically unreachable**: `init_processor` always sets `status = 1`. Verified unreachable in both C and Rust; must remain a *present, never-taken* branch, not a behaviour change. | would return `-5` and print `[ERROR] Invalid state status`; in practice never observed | [x] |
| E9 | `gotomach` | `!is_valid_state(state)` inside the loop — needs `state->count >= state->capacity` (or `status == 0`). **Statically unreachable**: `count` is incremented at most once per iteration, so `count <= i < iterations == capacity` always. | would return `-6` and print `[ERROR] State became invalid during processing`; in practice never observed | [x] |
| E10 | `gotomach` | `mode` not in `{0,1,2}` — the `switch` `default:`. This is a *soft* rejection, not an error return: it logs and falls back to `process_value`. Includes out-of-range "enum" ints crossing the FFI boundary: `-1`, `3`, `INT_MIN`, `INT_MAX`. | prints `[WARNING] Invalid mode, using default`, then behaves exactly like `mode == 0`; return value is the normal sum, **not** an error code | [x] |
| E11 | `gotomach` | `state->count >= UINT16_MAX` after an append — the in-loop early `break`. Reachable only with `iterations == 65535` and a `threshold` that admits every value. | prints `[WARNING] Reached maximum count`, breaks the loop, then returns the normal sum of the 65535 stored values | [x] |
| E12 | `process_value` / `double_value` / `triple_value` | `unused_context` is a garbage / null pointer, `unused_param` arbitrary | both are `(void)`-cast and ignored; result depends only on `value` — never an error, never a dereference | [x] |

## Boundary cases covered in addition to the table

Required by Phase C even though the C code has no explicit check for them:

| # | boundary | note | test |
|---|----------|------|------|
| B1 | `iterations` exactly `0` | passes validation; `malloc(0)` twice (glibc returns unique non-`NULL`); loop body never runs; `count == 0`; returns `0` | [x] |
| B2 | `iterations` exactly `65535` (last valid) vs `65536` (first invalid) | one-past-the-range pair | [x] |
| B3 | `seed` exactly `0` / `65535` / `65536` | one-past-the-range pair | [x] |
| B4 | `iterations == INT_MIN` / `INT_MAX`, `seed == INT_MIN` / `INT_MAX` | extreme out-of-range | [x] |
| B5 | `mode == INT_MIN` / `INT_MAX` / `-1` / `3` / `2147483647` | out-of-range enum ints over FFI (C enums/`int`s accept any value) | [x] |
| B6 | `threshold == INT_MIN` (nothing is `< INT_MIN` → `count == 0`) and `threshold == INT_MAX` (everything stored) | both ends of the accept/reject predicate | [x] |
| B7 | null `unused_context` and non-null garbage `unused_context` for the three op functions | pointer parameter that must stay unread | [x] |
| B8 | `value == INT_MIN` / `INT_MAX` for `double_value` / `triple_value` | C signed-overflow on `value * 2` / `value * 3`; must wrap identically | [x] |
| B9 | `process_value` with `value == INT_MAX` / `INT_MAX - 9` | `value + 10` overflow | [x] |

## Notes on E5–E7 (allocation-failure rows)

`gotomach` clamps `iterations` to `[0, 65535]`, so the largest request is
262 140 bytes; these rows cannot be reached by choosing input values alone.
They are tested by **interposing a failing `malloc`** with `LD_PRELOAD`
(`tests/failmalloc.c` → `libfailmalloc.so`), which makes the Nth `malloc`
return `NULL`. The C and Rust libraries are each run in a child process under
the same interposer and their exit codes compared, which also proves the two
libraries issue the **same number of `malloc` calls in the same order**
(3 per successful `gotomach`: `ProcessorState`, `results`, `temp_buffer`).

## Row → test mapping (Phase C)

| rows | test | file |
|---|---|---|
| E1 | `e1_iterations_negative_returns_minus_1` | `tests/phase_c_errors.rs` |
| E2 | `e2_iterations_above_uint16_max_returns_minus_1` | `tests/phase_c_errors.rs` |
| E3 | `e3_seed_negative_returns_minus_2` | `tests/phase_c_errors.rs` |
| E4 | `e4_seed_above_uint16_max_returns_minus_2` | `tests/phase_c_errors.rs` |
| E1+E3 ordering | `e1_e3_precedence_iterations_checked_before_seed` | `tests/phase_c_errors.rs` |
| E5 | `e5_processorstate_malloc_failure_returns_minus_3` | `tests/phase_c_alloc.rs` |
| E6 | `e6_results_malloc_failure_returns_minus_3` | `tests/phase_c_alloc.rs` |
| E7 | `e7_temp_buffer_malloc_failure_returns_minus_4` | `tests/phase_c_alloc.rs` |
| E5–E7 allocation count | `e5_e6_e7_baseline_three_allocations_per_successful_call`, `e5_e6_e7_no_fourth_allocation` | `tests/phase_c_alloc.rs` |
| E1–E4 allocate nothing | `e1_e4_reject_before_allocating` | `tests/phase_c_alloc.rs` |
| E8, E9 | `e8_e9_minus5_and_minus6_never_observed_in_either_library` (behaviour) + `d4_log_string_sets_are_identical` (branch still present in both `.so`s) | `tests/phase_c_errors.rs`, `tests/phase_d_symbols.rs` |
| E10 | `e10_out_of_range_mode_falls_back_to_process_value` | `tests/phase_c_errors.rs` |
| E11 | `e11_count_ceiling_break_matches` | `tests/phase_c_errors.rs` |
| E12, B7 | `e12_ops_ignore_unused_param_and_context` | `tests/phase_c_errors.rs` |
| B1 | `b1_iterations_zero_is_accepted_and_returns_zero` | `tests/phase_c_errors.rs` |
| B2, B3 | `b2_b3_one_past_range_pairs` | `tests/phase_c_errors.rs` |
| B4, B5, B6 | `b4_b5_b6_extreme_arguments` (full 9⁴ cross-product of extremes) | `tests/phase_c_errors.rs` |
| B8, B9 | `b8_b9_op_overflow_boundaries` | `tests/phase_c_errors.rs` |

**Status: 12/12 `ERRORS.md` rows and 9/9 boundary rows have a passing
differential test that asserts the identical error code / sentinel.**

## Divergence found and fixed

Rows E7 (and the E5/E6 allocation *ordering*) initially **failed**. LLVM had
deleted the entire `temp_buffer` allocation from the release `cdylib`, because
the Rust translation stored into `temp_buffer[i]` but then read the value back
from a local instead of from memory, making every store dead. Measured with the
interposer, on `gotomach(4, 7, 0, INT_MAX)` with the 3rd `malloc` forced to fail:

```
C        : RESULT=-4  MALLOCS=3
Rust (before): RESULT=128 MALLOCS=1     <-- wrong code, wrong allocation count
Rust (after) : RESULT=-4  MALLOCS=3
```

Two further branches had been optimised out of the Rust `.so` entirely — the
`-4` path above and the `-5` / `[ERROR] Invalid state status` path (LLVM
constant-folded `check_char_flag(1)` after inlining). The fix, in
`src/lib.rs`, was to make the Rust perform the same observable work the
unoptimised C does: `write_volatile`/`read_volatile` on `temp_buffer[i]`, and
`#[inline(never)]` on the four `static` helpers so their allocations and checks
survive. All ten log literals and all three allocations now match the C `.so`.
