# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. The library has **no** error enum,
no `RETURN_ERROR` macro, no `assert`, no `return -1` and no `return NULL`
(`grep -nE 'assert|return -1|return NULL|errno|RETURN_ERROR' c_src/src/lib.c`
→ no matches). Every rejection is therefore an *in-band* guard: an early
`return` of a sentinel value, a clamp, or a loop that declines to execute.
Each distinct guard/branch below is one row.

Guards found by `grep -nE 'if *\(|\?|return' c_src/src/lib.c`:
`lib.c:71`, `lib.c:76`, `lib.c:79`, `lib.c:82`, `lib.c:94`, `lib.c:101`,
`lib.c:103`, `lib.c:110`, `lib.c:112`, `lib.c:144`, `lib.c:163`, `lib.c:176`.

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| E1 | `modulo_operation` | `b == 0` (guard `lib.c:71`) — division by zero avoided | returns `0`, no trap | `err_e1_modulo_zero_divisor` | [x] |
| E2 | `modulo_operation` | `a == INT32_MIN, b == -1` — `INT_MIN % -1`, *not* guarded by C (only `b == 0` is) | **process-fatal**: x86-64 `idivl` raises `#DE` → `SIGFPE` (verified: exit status 136). Moved to the "not tested" section below — it returns no value, so there is no error code to compare. | documented, see below | [x] |
| E3 | `modulo_operation` | negative `a` and/or negative `b` (C `%` truncates toward zero, sign follows dividend) | `a % b` with dividend sign | `err_e3_modulo_signs` | [x] |
| E4 | `safe_double_to_int` | `d >= (double)INT32_MAX` (guard `lib.c:76`), incl. `d == 2147483647.0` exactly and `+INFINITY` | returns `INT32_MAX` (2147483647) | `err_e4_sdti_upper_clamp` | [x] |
| E5 | `safe_double_to_int` | `d <= (double)INT32_MIN` (guard `lib.c:79`), incl. `d == -2147483648.0` exactly and `-INFINITY` | returns `INT32_MIN` (-2147483648) | `err_e5_sdti_lower_clamp` | [x] |
| E6 | `safe_double_to_int` | `d != d`, i.e. NaN (guard `lib.c:82`) — reached only because both relational tests are false for NaN. Both quiet and signalling / negative NaN bit patterns. | returns `0` | `err_e6_sdti_nan` | [x] |
| E7 | `safe_double_to_int` | subnormal / `±0.0` / values in `(-1,1)` — truncation toward zero, incl. `-0.5 -> 0` and `-0.0 -> 0` | truncated `(int)d` | `err_e7_sdti_truncation` | [x] |
| E8 | `compute_scaled_value` | `base * scale_factor` overflows int range, or `scale_factor` is NaN/±INF | delegates to `safe_double_to_int` → clamp / `0` | `err_e8_scaled_overflow_nan` | [x] |
| E9 | `compare_results_in_array` | `idx1 >= arr->count` (guard `lib.c:94`, first disjunct) | returns `0` | `err_e9_cmp_idx1_too_big` | [x] |
| E10 | `compare_results_in_array` | `idx2 >= arr->count` (guard `lib.c:94`, second disjunct) | returns `0` | `err_e10_cmp_idx2_too_big` | [x] |
| E11 | `compare_results_in_array` | **negative** index (`idx < 0`): C only bounds-checks the *upper* limit, so a negative index is accepted and its out-of-object address is compared | no rejection — returns `-1`/`0`/`1` by address order (i.e. by index order) | `err_e11_cmp_negative_index` | [x] |
| E12 | `compare_results_in_array` | `idx1 == idx2` (both in range) — falls past both `<`/`>` tests (`lib.c:101`,`lib.c:103`) | returns `0` | `err_e12_cmp_equal_index` | [x] |
| E13 | `compare_results_in_array` | `arr->count <= 0` (empty array) with any indices ≥ 0 | returns `0` (guard trips) | `err_e13_cmp_empty_array` | [x] |
| E14 | `compare_results_in_array` | `INT32_MIN` / `INT32_MAX` indices across the FFI boundary | `INT32_MAX` → `0` (guard); `INT32_MIN` → address-ordered result | `err_e14_cmp_extreme_index` | [x] |
| E15 | `init_result_array` | `count > 10` (oversized length) — clamp `count < 10 ? count : 10` (`lib.c:110`) | `arr->count` set to `10`, only 10 elements written, no overflow of `data[10]` | `err_e15_init_oversized_count` | [x] |
| E16 | `init_result_array` | `count == 10` exactly — the `<` makes the ternary pick the `10` branch (same value) | `arr->count == 10` | `err_e16_init_count_exactly_10` | [x] |
| E17 | `init_result_array` | `count == 0` (zero length) | `arr->count == 0`, `data` left untouched | `err_e17_init_zero_count` | [x] |
| E18 | `init_result_array` | `count < 0` (negative length): the clamp accepts it, then `i < arr->count` is false immediately | `arr->count` set to the **negative** value, `data` untouched, `values` never dereferenced | `err_e18_init_negative_count` | [x] |
| E19 | `init_result_array` | `count == INT32_MIN` (one step past every valid range, extreme) | `arr->count == INT32_MIN`, no writes | `err_e19_init_intmin_count` | [x] |
| E20 | `process_with_foreach` | `arr->count == 0` — `FOREACH` size is 0, `count_iter != size` false on entry | returns `0`, `op` never called, `data` untouched | `err_e20_foreach_zero_count` | [x] |
| E21 | `process_with_foreach` | `op` returns values whose `*0.75` leaves int range (e.g. `multiply_operation` overflow) | `item->value` clamped via `safe_double_to_int`; `total` wraps two's-complement | `err_e21_foreach_op_overflow` | [x] |
| E22 | `process_with_foreach` | `op` is an arbitrary caller-supplied function pointer (including one that returns `INT32_MIN`/`INT32_MAX` every call, driving `total` to wrap) | wrapping `int` accumulation, no trap | `err_e22_foreach_extreme_op` | [x] |
| E23 | `compute_weighted_sum` | `arr->count <= 0` (zero or negative) — `i < arr->count` false on entry | returns `0` | `err_e23_weighted_nonpositive_count` | [x] |
| E24 | `compute_weighted_sum` | `i == 0`: `current > base` is false (pointers equal), so the ternary (`lib.c:144`) yields weight **1**, not the pointer difference `0` | element 0 weighted by 1 | `err_e24_weighted_index0_weight1` | [x] |
| E25 | `compute_weighted_sum` | `value * weight * 0.8` outside int range | per-term clamp by `safe_double_to_int`, then wrapping `int` sum | `err_e25_weighted_overflow` | [x] |
| E26 | `arrayfunc` | `param3 == INT32_MIN` (so `param3 * 2` overflows) and `param1 + param2` / `param2 - param3` overflow | two's-complement wrap; no trap | `err_e26_arrayfunc_overflow_inputs` | [x] |
| E27 | `arrayfunc` | `param4 == INT32_MIN` — `param4 / 2` (divisor is the constant `2`, so the `INT_MIN / -1` trap cannot occur) then `+ 1` | `-1073741824 + 1 == -1073741823` | `err_e27_arrayfunc_param4_intmin` | [x] |
| E28 | `arrayfunc` | `param4` negative odd (e.g. `-3`) — C `/` truncates toward zero, not floor | `-3/2 == -1`, so value is `0` | `err_e28_arrayfunc_param4_neg_odd` | [x] |
| E29 | all 11 entry points | out-of-range "enum" values across FFI: the library has **no** `enum`, so the analogous case is any `int` bit pattern with no distinguished meaning (`INT32_MIN`, `INT32_MAX`, `-1`, `0`) fed to every `int` parameter | never rejected — plain arithmetic on every value | `err_e29_no_enum_all_int_patterns` | [x] |

## Deliberately NOT tested (undefined behaviour in C — both sides crash identically)

Per the task rules these are real inputs, but they abort the process rather than
returning a value, so a differential test cannot observe a "same error code".
They are documented and the Rust mirrors the C construct 1:1:

| condition | C behaviour | Rust behaviour |
|-----------|-------------|----------------|
| `modulo_operation(INT32_MIN, -1, _, _)` (row **E2**) | `a % b` compiles to `idivl`; `INT_MIN / -1` overflows the quotient so the CPU raises `#DE` → `SIGFPE`, killing the process before any value is returned. Confirmed empirically with a C driver linked against the C `.so`: `exit=136` (`128 + SIGFPE`). | `wrapping_rem` returns `0`. A differential *value* comparison is impossible (C produces no value), and deliberately making Rust abort would be strictly worse for callers, so this single UB input is excluded from the value-comparison tests and documented here instead. Note it is **unreachable through every public path that composes `modulo_operation`**: `process_with_foreach` / `arrayfunc` always pass `item->rank` (which `init_result_array` sets to `0..=9`, never `-1`) as `b`. |
| `arr == NULL` in `compare_results_in_array` / `init_result_array` / `process_with_foreach` / `compute_weighted_sum` | dereferences NULL → `SIGSEGV` (no null check anywhere in `lib.c`) | `(*arr)` on a null `*mut` → same fault |
| `values == NULL` with `count > 0` in `init_result_array` | reads `values[i]` → `SIGSEGV` | `*values.wrapping_offset(i)` → same fault |
| `op == NULL` in `process_with_foreach` | calls through a NULL function pointer → `SIGSEGV` | `Option::unwrap_unchecked` on `None` → same fault |
| `arr->count < 0` in `process_with_foreach` | `FOREACH` terminates on `count_iter != size`; `count_iter` counts up from 0 and never reaches a negative `size` → runs off the end of `data[10]`, writes out of bounds, does not terminate | `while count_iter != size` with `wrapping_offset` writes → identical runaway |
| `arr->count > 10` in `process_with_foreach` / `compute_weighted_sum` (state forged by the caller, not reachable via `init_result_array`) | reads/writes past `data[10]` | identical out-of-bounds access |

`err_e29_no_enum_all_int_patterns` covers the "out-of-range enum value" clause
for every function that takes only scalars.
