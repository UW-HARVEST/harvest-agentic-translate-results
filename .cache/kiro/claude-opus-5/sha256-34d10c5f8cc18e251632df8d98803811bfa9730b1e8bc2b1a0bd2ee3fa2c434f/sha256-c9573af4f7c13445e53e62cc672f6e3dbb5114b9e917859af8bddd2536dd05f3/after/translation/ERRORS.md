# ERRORS.md — Phase C error / rejection surface

Mechanically derived from `c_src/src/lib.c`. The library has **no** error enum,
no `RETURN_ERROR` macro, no `assert`, and no function returning a pointer (so no
`return NULL`). Every rejection is either an early `return` of a sentinel value,
a clamp, or a guard-free path whose "error" behaviour is a hardware trap.
Each `if` that rejects/clamps/short-circuits input gets its own row, plus the
generic FFI boundaries (null pointers, zero/oversized lengths, one-past-range
values, out-of-range "enum"-like selectors).

Grep audit of every `return`/branch that is a rejection or clamp:

```
lib.c:71   if (b == 0) return 0;                       modulo_operation
lib.c:76   if (d >= (double)INT32_MAX) return INT32_MAX; safe_double_to_int
lib.c:79   if (d <= (double)INT32_MIN) return INT32_MIN; safe_double_to_int
lib.c:82   if (d != d) return 0;                        safe_double_to_int
lib.c:94   if (idx1 >= arr->count || idx2 >= arr->count) return 0;  compare_results_in_array
lib.c:108  arr->count = count < 10 ? count : 10;        init_result_array  (clamp)
lib.c:133  keep && count_iter != size                   process_with_foreach (FOREACH guard)
lib.c:151  (current > base) ? (int)(current - base) : 1  compute_weighted_sum (i==0 special case)
```

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ok |
|---|----------|---------------------------------------------|-------------------|------|----|
| E1 | `modulo_operation` | `b == 0` (any `a`, incl. `INT_MIN`, `0`, `INT_MAX`) | returns `0`, no trap | `err_e1_modulo_zero_divisor` | [x] |
| E2 | `modulo_operation` | `b == -1 && a == INT_MIN` → `idiv` overflow | **SIGFPE (signal 8)**, process dies; no value returned | `err_e2_modulo_intmin_by_neg1_traps` (out-of-process, compares signal) | [x] |
| E3 | `safe_double_to_int` | `d >= (double)INT32_MAX`, i.e. `d >= 2147483647.0` — incl. exactly `2147483647.0`, `2147483647.5`, `1e300`, `+INFINITY` | returns `INT32_MAX` (`2147483647`) | `err_e3_sdti_upper_clamp` | [x] |
| E4 | `safe_double_to_int` | `d <= (double)INT32_MIN`, i.e. `d <= -2147483648.0` — incl. exactly `-2147483648.0`, `-1e300`, `-INFINITY` | returns `INT32_MIN` (`-2147483648`) | `err_e4_sdti_lower_clamp` | [x] |
| E5 | `safe_double_to_int` | `d != d` (`NaN`, quiet or signalling, +/- sign, any payload) — reached only because NaN fails **both** relational tests above | returns `0` | `err_e5_sdti_nan` | [x] |
| E6 | `safe_double_to_int` | one step *inside* the clamp: `nextafter(2147483647.0, 0)` and `nextafter(-2147483648.0, 0)` | falls through to `(int)d` truncation, i.e. `2147483646` / `-2147483647` | `err_e6_sdti_one_step_inside` | [x] |
| E7 | `compute_scaled_value` | `base * scale_factor` overflows the int range (`base=INT_MAX, scale=1e10`), underflows (`base=INT_MIN, scale=1e10`), or is `NaN` (`base=0, scale=INFINITY` → `0*inf = NaN`) | delegates to `safe_double_to_int`: `INT32_MAX` / `INT32_MIN` / `0` | `err_e7_csv_overflow_underflow_nan` | [x] |
| E8 | `compare_results_in_array` | `idx1 >= arr->count` (e.g. `count=3, idx1=3`) | returns `0` (no address compare) | `err_e8_cmp_idx1_out_of_range` | [x] |
| E9 | `compare_results_in_array` | `idx2 >= arr->count` (e.g. `count=3, idx2=3`) | returns `0` | `err_e9_cmp_idx2_out_of_range` | [x] |
| E10 | `compare_results_in_array` | `arr->count == 0`, any indices `>= 0` | returns `0` (both guards fire) | `err_e10_cmp_count_zero` | [x] |
| E11 | `compare_results_in_array` | **negative** index — there is *no* lower-bound check, so `idx1 = -1` passes the guard and an out-of-bounds address is formed and compared | address arithmetic only (no deref): `-1 vs 0` → `-1`; `0 vs -1` → `1`; `-4 vs -4` → `0` | `err_e11_cmp_negative_index_unchecked` | [x] |
| E12 | `compare_results_in_array` | `idx1 == idx2` and in range | returns `0` (third branch) | `err_e12_cmp_equal_index` | [x] |
| E13 | `compare_results_in_array` | `arr->count` larger than the real array (`count = INT_MAX`), indices far past `data[10]` (`idx=1000`) | guard passes, OOB addresses compared, still `-1/0/1` by index order | `err_e13_cmp_count_lies` | [x] |
| E14 | `init_result_array` | `count > 10` (oversized length: `11`, `1000`, `INT_MAX`) | clamps `arr->count = 10`, reads only `values[0..10)` | `err_e14_init_count_clamped` | [x] |
| E15 | `init_result_array` | `count == 0` (zero length) | `arr->count = 0`, **`values` is never dereferenced** — even a null `values` is safe | `err_e15_init_count_zero_null_values_ok` | [x] |
| E16 | `init_result_array` | `count < 0` (negative length) | `count < 10` is true, so `arr->count` is set to the **negative** value; the `for` loop body never runs. Poisons the struct for later calls. | `err_e16_init_negative_count_poisons` | [x] |
| E17 | `process_with_foreach` | `arr->count == 0` | `count_iter != size` false immediately → returns `0`, array untouched | `err_e17_foreach_count_zero` | [x] |
| E18 | `process_with_foreach` | `arr->count < 0` (from E16). FOREACH tests `count_iter != size`, **not** `<`, so the loop runs away past `data[10]` | undefined behaviour: runs off the struct and traps/scribbles. Not a defined "error result" — excluded from in-process differential assertion; equivalence is established structurally (`while count_iter != size`) and by an out-of-process both-die check | `err_e18_foreach_negative_count_runs_away` | [x] |
| E19 | `compute_weighted_sum` | `arr->count == 0` | loop never runs → returns `0` | `err_e19_weighted_count_zero` | [x] |
| E20 | `compute_weighted_sum` | `arr->count < 0` | `i < count` is false immediately → returns `0` (differs from E18: this loop uses `<`) | `err_e20_weighted_negative_count` | [x] |
| E21 | `compute_weighted_sum` | element 0: `current > base` is false, so `weight = 1`, **not** `0` | `data[0]` contributes `sdti(value*1*0.8)`, not `0` | `err_e21_weighted_index0_weight_is_one` | [x] |
| E22 | `compute_weighted_sum` | `value * weight * 0.8` leaves int range (`value=INT_MAX`, `count=10` → weight up to 9) | per-element `safe_double_to_int` clamp to `INT32_MAX`; `sum` then wraps (2's complement) | `err_e22_weighted_saturates_then_wraps` | [x] |
| E23 | `arrayfunc` | `param4 = INT_MIN` → `param4 / 2` (division by the literal `2`; no trap, but the only division in `arrayfunc`) | `-1073741824`, `+1` → `-1073741823`; full pipeline result must match | `err_e23_arrayfunc_intmin_params` | [x] |
| E24 | `arrayfunc` | signed overflow in the `values[]` initialiser: `param1+param2`, `param2-param3`, `param3*2` at `INT_MAX`/`INT_MIN` | C UB; gcc emits 2's-complement wraparound. Rust must produce the identical wrapped value | `err_e24_arrayfunc_overflow_in_values` | [x] |
| E25 | all `ResultArray*` entry points (`compare_results_in_array`, `init_result_array`, `process_with_foreach`, `compute_weighted_sum`) | **null `arr` pointer** | immediate null deref → **SIGSEGV (signal 11)** in every case (`arr->count` is read first in all four) | `err_e25_null_arr_segv_all_entry_points` (out-of-process, compares signal per function) | [x] |
| E26 | `init_result_array` | **null `values`** with `count > 0` | null deref reading `values[0]` → **SIGSEGV** | `err_e26_null_values_segv` (out-of-process) | [x] |
| E27 | `process_with_foreach` | **null `op`** function pointer with `arr->count > 0` | call through null → **SIGSEGV** | `err_e27_null_op_segv` (out-of-process) | [x] |
| E28 | `process_with_foreach` | `op` is an *out-of-range selector*: the C API takes a raw `operation_func`, so a caller can pass any callable. Passing a callback that itself returns extreme values (`INT_MIN`, `INT_MAX`) or a non-`operations[]` function is a legal input the C handles | `result*0.75` then `safe_double_to_int` clamp; `total` wraps | `err_e28_foreach_arbitrary_callback` | [x] |
| E29 | operation selector as an out-of-range enum-like value | `arrayfunc` picks `operations[i]` for `i` in `0..4` only; the index is **not** caller-controlled, so no out-of-range enum value can reach it. The equivalent FFI hazard is the raw `operation_func` in E27/E28 (null / arbitrary), which is covered. Documented so the row is not silently skipped. | n/a — not reachable from the public API | `err_e29_no_caller_controlled_enum` (documents + asserts the 4 selectors are the only ones) | [x] |
| E30 | `modulo_operation` | negative operands: C `%` truncates toward zero, so the result takes the sign of `a` (`-7 % 3 == -1`, `7 % -3 == 1`) — a classic divergence point vs. floor-mod languages | sign follows the dividend | `err_e30_modulo_sign_follows_dividend` | [x] |
