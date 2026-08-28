# ERRORS.md — Phase C error-surface table

Mechanically derived by grepping `c_src/src/lib.c` for **every** guard, early
return, clamp, range check, null-sensitive dereference and sentinel value. The C
code uses no `errno`, no error enum, no `RETURN_ERROR` macro and no `assert`; it
rejects bad input exclusively via **early `return` of a sentinel** (`0`, `1`,
`INT32_MAX`, `INT32_MIN`), via **silent clamping**, or by **faulting**.

Status: **36 / 36 rows verified.** Every row has a differential test that
constructs the exact condition, calls both `.so`s, and asserts the *same*
specific sentinel (or the same termination status) — never merely "both failed".

Source inventory of rejection sites (line numbers in `c_src/src/lib.c`):

```
 71  if (b == 0) return 0;                                    modulo_operation
 72  return a % b;                        (idiv can raise #DE) modulo_operation
 76  if (d >= (double)INT32_MAX) return INT32_MAX;             safe_double_to_int
 79  if (d <= (double)INT32_MIN) return INT32_MIN;             safe_double_to_int
 82  if (d != d) return 0;                                     safe_double_to_int
 85  return (int)d;                                            safe_double_to_int
 90  return safe_double_to_int(scaled);                        compute_scaled_value
 94  if (idx1 >= arr->count || idx2 >= arr->count) return 0;   compare_results_in_array
101  if (ptr1 < ptr2) return -1;                               compare_results_in_array
103  else if (ptr1 > ptr2) return 1;                           compare_results_in_array
106  return 0;                                                 compare_results_in_array
110  arr->count = count < 10 ? count : 10;                     init_result_array (clamp)
112  for (i = 0; i < arr->count; i++)                          init_result_array (neg => no writes)
125  FOREACH(... arr->count)   -> `keep && count_iter != size` process_with_foreach
126  op(...)                              (NULL op -> jump 0)  process_with_foreach
131  item->value = safe_double_to_int(temp);                    process_with_foreach
140  for (i = 0; i < arr->count; i++)                          compute_weighted_sum
144  weight = (current > base) ? (int)(current-base) : 1;      compute_weighted_sum (i==0)
147  sum += safe_double_to_int(weighted);                      compute_weighted_sum
163  param4 / 2                          (divisor is const 2)  arrayfunc
182  result = safe_double_to_int(final_scale);                  arrayfunc
```

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] | verifying test |
|----|----------|----------------------------------------------|-------------------|-----|----------------|
| E1  | `modulo_operation` | `b == 0` (division by zero) | returns `0`; no SIGFPE | [x] | `e1_modulo_by_zero_returns_zero` |
| E2  | `modulo_operation` | `a == INT32_MIN, b == -1` — the single `idivl` computes an implicit quotient of `2147483648`, which does not fit in `eax` | CPU raises `#DE` → **killed by SIGFPE (signal 8)** | [x] | `crash_probes::e2_intmin_remainder_raises_sigfpe_in_both`, `…::e2b_…_through_pipeline_…` |
| E3  | `safe_double_to_int` | `d >= 2147483647.0` (at or above `(double)INT32_MAX`), incl. `+INFINITY`, `DBL_MAX` | returns `INT32_MAX` | [x] | `e3_safe_double_to_int_at_or_above_intmax_clamps` |
| E4  | `safe_double_to_int` | `d == 2147483647.0` exactly (boundary; `>=`, so it *is* rejected) | returns `INT32_MAX` | [x] | `e4_safe_double_to_int_exactly_intmax_is_rejected` |
| E5  | `safe_double_to_int` | `d <= -2147483648.0` (at or below `(double)INT32_MIN`), incl. `-INFINITY`, `-DBL_MAX` | returns `INT32_MIN` | [x] | `e5_safe_double_to_int_at_or_below_intmin_clamps` |
| E6  | `safe_double_to_int` | `d == -2147483648.0` exactly (boundary; `<=`, so it *is* rejected) | returns `INT32_MIN` | [x] | `e6_safe_double_to_int_exactly_intmin_is_rejected` |
| E7  | `safe_double_to_int` | `d` is NaN (quiet, signalling, negative-sign, payload-carrying) — both relational tests are false, so `d != d` catches it | returns `0` | [x] | `e7_safe_double_to_int_nan_returns_zero` (20 000 random NaN payloads) |
| E8  | `compute_scaled_value` | `base * scale_factor` leaves `int` range | saturates via `safe_double_to_int` → `INT32_MAX` / `INT32_MIN` | [x] | `e8_compute_scaled_value_overflow_saturates` |
| E9  | `compute_scaled_value` | `scale_factor` is NaN, or `base == 0 && scale == ±INFINITY` (→ NaN) | returns `0` | [x] | `e9_compute_scaled_value_nan_returns_zero` |
| E10 | `compute_scaled_value` | `scale_factor` is `±INFINITY` with `base != 0` | `INT32_MAX` / `INT32_MIN` by sign | [x] | `e10_compute_scaled_value_infinite_scale` |
| E11 | `compare_results_in_array` | `idx1 >= arr->count` (e.g. `idx1 == count`) | returns `0` (guard hit) | [x] | `e11_compare_idx1_at_or_past_count_returns_zero` |
| E12 | `compare_results_in_array` | `idx2 >= arr->count` | returns `0` (guard hit) | [x] | `e12_compare_idx2_at_or_past_count_returns_zero` |
| E13 | `compare_results_in_array` | **both** indices out of range | returns `0` | [x] | `e13_compare_both_indices_out_of_range_returns_zero` |
| E14 | `compare_results_in_array` | `arr->count == 0` with any non-negative indices | returns `0` | [x] | `e14_compare_empty_array_always_returns_zero` |
| E15 | `compare_results_in_array` | negative index — the guard checks **only** the upper bound, so `idx = -1` is *accepted* and address arithmetic proceeds | returns `-1`/`0`/`1` by address order, **not** the `0` sentinel | [x] | `e15_compare_negative_indices_are_accepted_not_rejected` |
| E16 | `compare_results_in_array` | `arr->count` negative: every non-negative index is `>= count` | returns `0` | [x] | `e16_compare_negative_count_rejects_all_nonnegative_indices` |
| E17 | `compare_results_in_array` | `idx1 == idx2` (both in range) | returns `0` (equal addresses) | [x] | `e17_compare_equal_indices_returns_zero` |
| E18 | `compare_results_in_array` | `count` hand-set `> 10`, indices past the `data[10]` capacity — no bounds check on `data[]` | returns by address order | [x] | `e18_compare_count_beyond_capacity_has_no_bounds_check`, `e24_deterministic_out_of_bounds_marching_init_and_compare` |
| E19 | `init_result_array` | `count > 10` (oversized) | silently clamps: `arr->count = 10`; only 10 elements written | [x] | `e19_init_oversized_count_clamps_to_ten` |
| E20 | `init_result_array` | `count == 10` (boundary; `< 10` is false) | clamps to `10` — same value, all 10 written | [x] | `e20_init_count_exactly_ten_is_the_boundary` |
| E21 | `init_result_array` | `count < 0` | `arr->count` left **negative**; loop body never runs; `values` never dereferenced (so `values == NULL` is safe) | [x] | `e21_init_negative_count_writes_nothing_and_ignores_null_values` |
| E22 | `init_result_array` | `count == 0` | `arr->count = 0`; no writes; `values == NULL` safe | [x] | `e22_init_zero_count_writes_nothing_and_ignores_null_values` |
| E23 | `process_with_foreach` | `arr->count == 0` | `FOREACH` runs 0 iterations; returns `0`; **`op` is never dereferenced**, so a NULL `op` is harmless | [x] | `e23_process_with_null_op_and_zero_count_is_safe`, `e23b_…`, `crash_probes::controls_…` |
| E24 | `process_with_foreach` | `arr->count < 0` — `FOREACH` terminates on `count_iter != size`, never true for negative `size`, so the loop marches forward writing 24 bytes at a time | process is killed. The *exact* signal is nondeterministic **in both implementations** (SIGSEGV on an unmapped page, or SIGABRT once glibc heap metadata is corrupted), so the assertions are: (a) both die by signal, never return; (b) the identical out-of-bounds address arithmetic and writes, checked deterministically on a large mapped buffer | [x] | `crash_probes::e24_negative_count_kills_both_processes` + `e24_deterministic_out_of_bounds_marching_process` / `…_weighted_sum` / `…_init_and_compare` |
| E25 | `process_with_foreach` | `op` is NULL **and** `arr->count > 0` | calls through a NULL function pointer → **SIGSEGV** | [x] | `crash_probes::e25_null_op_with_nonzero_count_faults_in_both` |
| E26 | `process_with_foreach` | `op` returns an extreme value, so `result * 0.75` needs clamping | `item->value` saturates (`INT32_MAX → 1610612735`, `INT32_MIN → -1610612736`) | [x] | `e26_process_with_extreme_op_results_saturates` |
| E27 | `compute_weighted_sum` | `arr->count <= 0` | loop never runs; returns `0` | [x] | `e27_weighted_sum_nonpositive_count_returns_zero` |
| E28 | `compute_weighted_sum` | `i == 0` — `current > base` is false, so the pointer-difference weight would be `0` | falls back to `weight = 1` (**not** 0) | [x] | `e28_weighted_sum_index_zero_uses_weight_one_not_zero` |
| E29 | `compute_weighted_sum` | `value * weight * 0.8` leaves `int` range | each term saturates via `safe_double_to_int`; `sum` accumulates with wraparound | [x] | `e29_weighted_sum_saturating_terms` (cross-checked against a model built from the C's own primitives) |
| E30 | `arrayfunc` | `param3 * 2` overflows `int` | two's-complement wraparound, no trap | [x] | `e30_arrayfunc_param3_times_two_overflow` |
| E31 | `arrayfunc` | `param1 + param2` / `param2 - param3` overflow `int` | two's-complement wraparound | [x] | `e31_arrayfunc_add_sub_overflow` |
| E32 | `arrayfunc` | `param4 == INT32_MIN` → `param4 / 2` (divisor is the constant `2`, so `INT_MIN / -1` cannot occur); also odd negatives truncate toward zero | `-1073741824`, then `+1` → `-1073741823` | [x] | `e32_arrayfunc_param4_intmin_division` |
| E33 | `arrayfunc` | accumulated `result` overflows `int` before the final `* 0.333` | wraparound, then the final `safe_double_to_int` clamp | [x] | `e33_arrayfunc_accumulator_overflow_then_final_clamp` (120 000 randomized quadruples) |
| E34 | `compare_results_in_array`, `init_result_array`, `process_with_foreach`, `compute_weighted_sum` | `arr == NULL` → unconditional `arr->count` dereference | **killed by SIGSEGV (signal 11)** in all four | [x] | `crash_probes::e34_null_result_array_faults_in_both` |
| E35 | `init_result_array` | `values == NULL` with `count > 0` | **killed by SIGSEGV (signal 11)** | [x] | `crash_probes::e35_null_values_with_nonzero_count_faults_in_both` |
| E36 | `process_with_foreach` | the "out-of-range enum" analogue: an arbitrary caller-supplied `operation_func` that is neither NULL nor one of the four built-ins (the FFI accepts *any* pointer value, exactly as a C enum accepts any `int`) | called verbatim; the result is folded into `total` / `scaled` / `value` identically. Also pins down that the library passes literal `0, 0` for `unused1`/`unused2` | [x] | `e36_arbitrary_function_pointer_is_called_verbatim`, `c16_process_with_foreach_external_callback` |

## Generic FFI-boundary coverage (beyond the table)

| condition | [x] | verifying test |
|-----------|-----|----------------|
| NULL pointers on every pointer-taking entry point | [x] | `crash_probes::e34_…`, `e35_…` |
| zero and oversized "lengths" (`count == 0`, `10`, `11`, `INT32_MAX`) end to end | [x] | `generic_zero_and_oversized_lengths_agree` |
| one step past every documented range (`count ± 1`, `idx == count`, `idx == count-1`) | [x] | rows E11–E14 and E18–E20 above, plus `generic_extreme_index_values_do_not_diverge` |
| absolute extremes (`INT32_MIN`, `INT32_MIN+1`, `INT32_MAX-1`, `INT32_MAX`) as counts and indices | [x] | `generic_extreme_index_values_do_not_diverge`, `generic_init_with_extreme_counts` |
| out-of-range "enum" values across the FFI — here the `operation_func` pointer, the only type in this ABI that accepts arbitrary caller values | [x] | `E36`, `c16` |
| `double` bit patterns with no meaningful value (random `u64` reinterpreted) | [x] | `c5_safe_double_to_int_all_classes`, `c7_compute_scaled_value` |

## Bug found and fixed during Phase C

**`modulo_operation(INT32_MIN, -1)` (row E2).** The original Rust used
`a.wrapping_rem(b)`, which quietly returns `0`. The compiled C executes a single
`idivl` whose implicit quotient overflows, so the process dies with **SIGFPE**.
Neither Rust operator reproduces this (`a % b` panics → SIGABRT under
`panic = "abort"`; `wrapping_rem` returns `0`), so the remainder is now computed
with an explicit `cdq; idiv` inline-asm sequence — the same instruction the C
compiler emits. Verified: both libraries now exit with signal 8.
