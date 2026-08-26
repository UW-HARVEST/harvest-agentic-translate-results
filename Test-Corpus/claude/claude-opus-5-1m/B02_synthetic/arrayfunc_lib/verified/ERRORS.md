# ERRORS.md — Phase C error-surface table

Every distinct rejection / guard / saturation / clamp / early-out in
`c_src/src/lib.c`, obtained by grepping the source for every `if (…) return`,
every ternary clamp, every implicit loop guard, and every implicit
range/overflow boundary. `lib.c` has **no** `assert`, **no** `RETURN_ERROR`
macro, **no** error `enum` and **no** `NULL` checks — its whole error surface is
made of value clamps, guard `return`s and loop pre-conditions, listed below.

Source lines referenced against `c_src/src/lib.c`.

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| E1 | `modulo_operation` | `b == 0` (line 71) | returns `0` (no division performed) | [x] |
| E2 | `safe_double_to_int` | `d >= (double)INT32_MAX` (line 76) — incl. `+INF`, `2147483647.0` exactly, `1e300` | returns `INT32_MAX` (`2147483647`) | [x] |
| E3 | `safe_double_to_int` | `d <= (double)INT32_MIN` (line 79) — incl. `-INF`, `-2147483648.0` exactly, `-1e300` | returns `INT32_MIN` (`-2147483648`) | [x] |
| E4 | `safe_double_to_int` | `d != d`, i.e. NaN (line 82) — quiet **and** signalling NaN, both signs | returns `0` (NaN fails E2 and E3 first) | [x] |
| E5 | `compute_scaled_value` | product `base * scale_factor` overflows `int` upward | `INT32_MAX` (delegated to E2) | [x] |
| E6 | `compute_scaled_value` | product overflows `int` downward | `INT32_MIN` (delegated to E3) | [x] |
| E7 | `compute_scaled_value` | `scale_factor` NaN, or `0 * INF` → NaN | `0` (delegated to E4) | [x] |
| E8 | `compare_results_in_array` | `idx1 >= arr->count` (line 94) | returns `0` | [x] |
| E9 | `compare_results_in_array` | `idx2 >= arr->count` (line 94) | returns `0` | [x] |
| E10 | `compare_results_in_array` | **negative** `idx1`/`idx2` — *not* validated by C; `&arr->data[idx]` is formed out of bounds and only compared, never dereferenced | falls through to the pointer compare: `-1` / `0` / `+1` by index order | [x] |
| E11 | `compare_results_in_array` | `arr->count <= 0` (any index then satisfies `idx >= count` unless the index is negative) | returns `0` for all non-negative indices | [x] |
| E12 | `compare_results_in_array` | `idx1 == idx2` (both `< count`) — the `ptr1 < ptr2` / `ptr1 > ptr2` chain both fail | returns `0` (line 106) | [x] |
| E13 | `init_result_array` | `count >= 10` (clamp `count < 10 ? count : 10`, line 110) | `arr->count` set to `10`; only 10 elements filled; `values[10..]` never read | [x] |
| E14 | `init_result_array` | `count < 0` | `arr->count` stores the **negative** value verbatim; fill loop `i < arr->count` never runs; `data[]` untouched | [x] |
| E15 | `init_result_array` | `count == 0` | `arr->count = 0`, `data[]` untouched, `values` never dereferenced (safe with `values == NULL`) | [x] |
| E16 | `init_result_array` | `values[i] == INT32_MIN`/`INT32_MAX` → `(double)v * 1.5` is exact-ish but out of `int` range; `scaled` is a `double` so no clamp happens here | `scaled` = exact `v * 1.5`, no saturation | [x] |
| E17 | `process_with_foreach` | `arr->count == 0` — `FOREACH` pre-condition `count_iter != size` is false on entry | returns `0`, `op` never called (safe even with `op == NULL`) | [x] |
| E18 | `process_with_foreach` | `arr->count < 0` — `FOREACH` uses `count_iter != size` (**not** `<`), so `0 != -1` is true and the loop walks forward off the object | `SIGSEGV` once the walk leaves mapped memory. Verified: C and Rust die with the identical status. | [x] |
| E19 | `process_with_foreach` | value written back is `safe_double_to_int(result * 0.75)` — saturates for `|result * 0.75| >= INT32_MAX` | element `value` clamps via E2/E3 | [x] |
| E20 | `process_with_foreach` | `op == NULL` **and** `arr->count > 0` — C jumps to address 0 | `SIGSEGV` (C UB). Rust must behave identically. | [x] |
| E21 | `compute_weighted_sum` | `arr->count <= 0` (loop guard `i < arr->count`, line 140) | returns `0` | [x] |
| E22 | `compute_weighted_sum` | index 0: `current == base`, so `current > base` is false | `weight = 1`, **not** `0` (line 144) | [x] |
| E23 | `compute_weighted_sum` | `value * weight * 0.8` outside `int` range | per-term clamp via E2/E3, then `int` accumulate wraps | [x] |
| E24 | `arrayfunc` | `arr.count - 1` bound on the compare loop (line 176) — `count` is always 8 here, so `i` never reaches an invalid index | 7 iterations, each contributing `-1` | [x] |
| E25 | `arrayfunc` | `param4 == INT32_MIN` → `param4 / 2` (line 163). Divisor is the constant 2, so no `SIGFPE`; result is `-1073741824`, `+1` → `-1073741823` | no trap, value as above | [x] |
| E26 | `arrayfunc` | `param1 + param2`, `param2 - param3`, `param3 * 2` overflow `int` (line 162-163) — signed overflow is C UB, the compiled code wraps | two's-complement wrap | [x] |
| E27 | `arrayfunc` | final `safe_double_to_int(result * 0.333)` — `result * 0.333` can never leave `int` range, but NaN/clamp path is still shared | value via E2/E3/E4 rules | [x] |
| E28 | `compare_results_in_array` / `init_result_array` / `process_with_foreach` / `compute_weighted_sum` | `arr == NULL` — no null check anywhere in `lib.c`; the first member access dereferences it | `SIGSEGV` (C UB). Rust must behave identically. | [x] |
| E29 | `init_result_array` | `values == NULL` with `count > 0` | `SIGSEGV` (C UB). Rust must behave identically. | [x] |
| E30 | `modulo_operation` | `a == INT32_MIN && b == -1` — C computes `a % b` with `idiv`, which raises `#DE` | `SIGFPE` (C UB, x86-64). Rust returns `0` via `wrapping_rem`. **Known UB-only divergence**, see note below. | [x] (documented) |

## Out-of-range "enum" values across the FFI boundary

`lib.c` declares **no `enum` type**, so there is no integer-with-no-valid-variant
case. The closest analogue is the `operation_func` function-pointer parameter of
`process_with_foreach`, whose "out of range" values are:

* `NULL` → row **E20** (differential crash test);
* an arbitrary caller-supplied function pointer that is *not* one of the four
  built-in operations → a perfectly valid input the C code must call; covered by
  `CONFIGS.md` rows C21–C23 which pass callbacks defined in the **test** binary
  (including ones that return `INT32_MIN`/`INT32_MAX` and ones that mutate the
  array behind the library's back).

Every `int` parameter (`a`, `b`, `idx1`, `idx2`, `count`, `param1..4`) accepts the
full `int32` range and is exercised at `INT32_MIN`, `-1`, `0`, `1`, `INT32_MAX`
and one step past every documented bound (`count = 10 ± 1`, `idx = count ± 1`).

## Row -> test mapping

| rows | test |
|------|------|
| E1 | `phase_c_errors::phase_c_e1_modulo_by_zero` |
| E2, E3, E4 | `phase_c_e2_saturate_high`, `phase_c_e3_saturate_low`, `phase_c_e4_nan_sentinel` |
| E5, E6, E7 | `phase_c_e5_scaled_overflow_high`, `phase_c_e6_scaled_overflow_low`, `phase_c_e7_scaled_nan` |
| E8..E12 | `phase_c_e8_idx1_out_of_range`, `phase_c_e9_idx2_out_of_range`, `phase_c_e10_negative_indices_unvalidated`, `phase_c_e11_count_non_positive`, `phase_c_e12_equal_indices` |
| E13..E16 | `phase_c_e13_count_clamped_to_ten`, `phase_c_e14_negative_count_stored_verbatim`, `phase_c_e15_count_zero_never_reads_values`, `phase_c_e16_extreme_values_no_clamp_in_scaled` |
| E17, E19 | `phase_c_e17_count_zero_never_calls_op`, `phase_c_e19_writeback_saturates` |
| E18 | `phase_c_e18_negative_count_runaway` (child processes `child_{c,rust}_negative_count`) |
| E20 | `phase_c_e20_null_op_with_nonempty_array` (children `child_{c,rust}_null_op`) |
| E21, E22, E23 | `phase_c_e21_count_non_positive_returns_zero`, `phase_c_e22_index_zero_weight_is_one`, `phase_c_e23_per_term_clamp_then_wrapping_accumulate` |
| E24..E27 | `phase_c_e24_compare_loop_bound`, `phase_c_e25_param4_int_min_no_trap`, `phase_c_e26_overflowing_derived_values`, `phase_c_e27_final_scale_shared_path` |
| E28 | `phase_c_e28_null_arr_*` (4 tests, one per pointer-taking function) |
| E29 | `phase_c_e29_null_values_with_positive_count` |
| E30 | `phase_c_e30_modulo_int_min_by_neg_one` |

Observed crash statuses (x86-64 Linux, gcc 11.5, rustc 1.94, **both** the debug
and the release Rust profile):

| condition | C | Rust |
|-----------|---|------|
| `arr == NULL` (all 4 functions) | SIGSEGV (11) | SIGSEGV (11) |
| `values == NULL`, `count > 0` | SIGSEGV (11) | SIGSEGV (11) |
| `op == NULL`, `count > 0` | SIGSEGV (11) | SIGSEGV (11) |
| `op == NULL`, `count == 0` | returns 0 | returns 0 |
| `values == NULL`, `count <= 0` | returns, `count` stored | returns, `count` stored |
| `count < 0` into `process_with_foreach` | SIGSEGV (11) | SIGSEGV (11) |
| `INT32_MIN % -1` | SIGFPE (8) | returns 0 (see E30) |

> Reproducing C's `SIGSEGV` on NULL required a change to the translation: written
> as Rust place expressions, `(*arr).count` and friends make `rustc` emit a
> null-pointer check whenever `-C debug-assertions` is on, which turned C's
> silent `SIGSEGV` into a Rust `SIGABRT` plus a panic message. `src/lib.rs` now
> performs those accesses with `wrapping_byte_add` + `core::ptr::{read, write}`,
> which are as unchecked as C's `->` in every profile.

## Notes on the one remaining UB-only row

* **E30** (`INT32_MIN % -1`): the C behaviour is a hardware trap, i.e. there is
  no value a Rust translation could return that would "match". The Rust code
  deliberately uses `wrapping_rem` (returns `0`) so that the Rust library is not
  strictly worse than C; the input is unreachable from `arrayfunc` (the `b`
  argument there is always a rank `0..7`, and `b == 0` is intercepted by E1).
  `tests/phase_c_errors.rs::phase_c_e30_modulo_int_min_by_neg_one` asserts the
  documented behaviour of each side rather than equality.
