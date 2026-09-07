# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. The C code contains **no**
`RETURN_ERROR`-style macros, **no** `assert`, **no** error enums, and **no**
`NULL` checks. Every rejection is either an early `return` of a sentinel value,
a clamp, or an unchecked dereference. Enumerated by grepping every `if`,
ternary, and `return` in the file:

```sh
grep -nE 'return|assert|\?|if *\(|==|>=|<=' c_src/src/lib.c
```

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| 1 | `modulo_operation` | `b == 0` (line 71 guard), any `a` | returns `0` (does **not** trap) | `err_modulo_zero_divisor` | [x] |
| 2 | `modulo_operation` | `a == INT32_MIN && b == -1` — passes the `b==0` guard, then `idiv` overflows | process dies on **SIGFPE (8)**; no value returned | `err_modulo_intmin_neg1_sigfpe` + `err_fatal_traps_and_indirect_call` (subprocess) | [x] |
| 3 | `safe_double_to_int` | `d >= (double)INT32_MAX` — incl. `2147483647.0`, `2147483648.0`, `+INFINITY`, `1e300` | returns `INT32_MAX` (2147483647) | `err_sdti_upper_clamp` | [x] |
| 4 | `safe_double_to_int` | `d <= (double)INT32_MIN` — incl. `-2147483648.0`, `-2147483649.0`, `-INFINITY`, `-1e300` | returns `INT32_MIN` (-2147483648) | `err_sdti_lower_clamp` | [x] |
| 5 | `safe_double_to_int` | `d != d` (NaN — quiet, signalling, negative, arbitrary payload) | returns `0` | `err_sdti_nan` | [x] |
| 6 | `compute_scaled_value` | `base * scale_factor` overflows `int` (e.g. `base=INT32_MAX, scale=2.0`) | saturates via #3/#4 → `INT32_MAX` / `INT32_MIN` | `err_compute_scaled_value_saturation` | [x] |
| 7 | `compute_scaled_value` | `scale_factor` is NaN, or `0 * INFINITY` | `0` (NaN path, row #5) | `err_compute_scaled_value_nan` | [x] |
| 8 | `compare_results_in_array` | `idx1 >= arr->count` (line 94, first disjunct) | returns `0` | `err_compare_idx1_too_large` | [x] |
| 9 | `compare_results_in_array` | `idx2 >= arr->count` (line 94, second disjunct) | returns `0` | `err_compare_idx2_too_large` | [x] |
| 10 | `compare_results_in_array` | `arr->count <= 0` — **every** index fails `>= count`, including `0` | returns `0` for all index pairs | `err_compare_empty_array` | [x] |
| 11 | `compare_results_in_array` | **negative** `idx1`/`idx2` — there is *no* lower-bound check, so the guard passes and out-of-bounds addresses are formed (never dereferenced) and compared | `-1` / `0` / `1` by address order, i.e. `sign(idx1 - idx2)` | `err_compare_negative_indices` | [x] |
| 12 | `init_result_array` | `count > 10` (oversized length) — clamp at line 110 | `arr->count` set to `10`; only 10 elements written | `err_init_count_oversized` | [x] |
| 13 | `init_result_array` | `count < 0` — the ternary keeps the negative value | `arr->count` set to the negative value; loop body never runs; `data` untouched | `err_init_count_negative` | [x] |
| 14 | `init_result_array` | `count == 0` (zero length) | `arr->count = 0`; `data` untouched; `values` never read (may be NULL) | `err_init_count_zero` | [x] |
| 15 | `process_with_foreach` | `arr->count == 0` — `FOREACH` condition `count_iter != size` is false immediately | returns `0`; `op` never called; array untouched | `err_foreach_empty` | [x] |
| 16 | `arrayfunc` | `param4 == INT32_MIN` — `param4 / 2` at the signed-division boundary (divisor is the literal `2`, so div-by-zero is impossible) | well-defined `-1073741824`, feeds slot 7 | `err_arrayfunc_extreme_params` | [x] |
| 17 | `arrayfunc` | signed overflow in the derived slots: `param1+param2`, `param2-param3`, `param3*2` at `INT32_MIN`/`INT32_MAX` | two's-complement wraparound (what gcc emits at `-O0`) | `err_arrayfunc_extreme_params` | [x] |
| 18 | `compare_results_in_array` | `arr == NULL` — no null check, `arr->count` is loaded | **SIGSEGV (11)** | `err_null_pointer_dereferences` (subprocess) | [x] |
| 19 | `init_result_array` | `arr == NULL` — no null check, `arr->count` is stored | **SIGSEGV (11)** | `err_null_pointer_dereferences` (subprocess) | [x] |
| 20 | `init_result_array` | `values == NULL` with `count > 0` — `values[i]` is loaded | **SIGSEGV (11)** | `err_null_pointer_dereferences` (subprocess) | [x] |
| 21 | `process_with_foreach` | `arr == NULL` | **SIGSEGV (11)** | `err_null_pointer_dereferences` (subprocess) | [x] |
| 22 | `process_with_foreach` | `op == NULL` with `count > 0` — indirect call through a null function pointer | **SIGSEGV (11)** | `err_fatal_traps_and_indirect_call` (subprocess) | [x] |
| 23 | `compute_weighted_sum` | `arr == NULL` | **SIGSEGV (11)** | `err_null_pointer_dereferences` (subprocess) | [x] |
| 24 | `compute_weighted_sum` | `arr->count` values that make every product saturate (`value=INT32_MAX`, weight up to 9) | each term clamps via row #3/#4; `sum` wraps two's-complement | `err_weighted_sum_saturation` | [x] |
| 25 | `add_operation` / `subtract_operation` / `multiply_operation` | signed overflow (`INT32_MAX + 1`, `INT32_MIN - 1`, `INT32_MIN * -1`, …) | two's-complement wraparound; no guard exists | `err_binops_overflow` | [x] |

## Out-of-range "enum" values across the FFI boundary

The C API declares **no enums**. The equivalent unchecked-integer inputs that a
caller can push across the boundary are covered above:

- `int count` outside `[0, 10]` → rows 12, 13, 14 (and the excluded case below).
- `int idx1`/`idx2` outside `[0, count)` → rows 8, 9, 10, 11.
- `operation_func op` — an arbitrary pointer-sized value; `NULL` is row 22.
  Rows 1–2 and `cfg_foreach_add` / `cfg_foreach_multiply` / `cfg_foreach_subtract` / `cfg_foreach_modulo` cover every *valid* variant, and
  `cfg_foreach_cross_abi` passes the C `.so`'s function pointers into the Rust
  `process_with_foreach` (and vice versa) to prove the callback ABI matches.
- `double d` outside the `int` range, plus `±Inf` / NaN → rows 3, 4, 5.

## Conditions deliberately NOT differential-tested (unbounded memory corruption)

These are reachable but produce non-deterministic out-of-bounds *writes*, so
"identical output" is not a meaningful assertion for either side. Both
implementations perform the same unchecked arithmetic; documented for
completeness.

| condition | why excluded |
|-----------|--------------|
| `process_with_foreach` with `arr->count < 0` | `FOREACH` compares `count_iter != size`; a negative `size` is never reached going upward, so it writes `item->scaled` / `item->value` for ~2^31 increasing out-of-bounds indices before wrapping. Both sides scribble over the heap and die at an implementation-defined address. |
| `compute_weighted_sum` / `process_with_foreach` with `arr->count > 10` | Reads/writes past `data[10]`. Only reachable if the caller writes `count` directly (`init_result_array` clamps it). Tested only up to the in-bounds maximum of 10. |
| `compare_results_in_array` with a wildly negative index (e.g. `INT32_MIN`) | `arr->data + INT32_MIN` overflows the pointer; no dereference occurs, but the address is not representable. Tested with small negative indices (row 11) where both sides agree. |

## Finding: fatal-signal parity depends on the Rust build profile

Rows 18–21 and 23 (the null-pointer *dereferences*) were the only place any
divergence appeared during verification, and it is a property of the Rust build
profile rather than of the translation. Measured terminating signals:

| case | C `.so` | Rust `.so` (release) | Rust `.so` (debug) |
|------|---------|----------------------|--------------------|
| `mod_intmin_neg1` (row 2)   | SIGFPE 8   | SIGFPE 8   | SIGFPE 8   |
| `foreach_null_op` (row 22)  | SIGSEGV 11 | SIGSEGV 11 | SIGSEGV 11 |
| `cmp_null_arr` (row 18)     | SIGSEGV 11 | SIGSEGV 11 | **SIGABRT 6** |
| `init_null_arr` (row 19)    | SIGSEGV 11 | SIGSEGV 11 | **SIGABRT 6** |
| `init_null_values` (row 20) | SIGSEGV 11 | SIGSEGV 11 | **SIGABRT 6** |
| `foreach_null_arr` (row 21) | SIGSEGV 11 | SIGSEGV 11 | **SIGABRT 6** |
| `wsum_null_arr` (row 23)    | SIGSEGV 11 | SIGSEGV 11 | **SIGABRT 6** |

Cause: with `debug-assertions` on, rustc inserts `ub_checks` instrumentation
that intercepts the raw-pointer dereference and raises a non-unwinding panic
(`null pointer dereference occurred` → `abort`) *before* the hardware fault
would occur. It is a compiler diagnostic with no C counterpart, and it cannot
be switched off on stable rustc (`profile.dev.ub-checks` is an unstable
manifest key — verified: cargo reports it as an unused key).

Resolution: the artifact this crate ships is the **release** cdylib — the
profile `Cargo.toml` explicitly configures (`panic = "abort"`), and the one a C
consumer would link. It matches the C on **all seven** fatal cases. The default
`cargo test` run therefore enforces full parity. `run_all.sh` additionally
re-runs the whole suite against the debug `.so` as an overflow-check stress
pass, skipping exactly one test (`err_null_pointer_dereferences`) and nothing
else; row 22 and row 2 are still asserted there because an indirect call
through a null function pointer and an `idiv` trap are not dereferences and are
not instrumented.

`foreach_null_op` (row 22) also confirms the translation does not add a null
check that the C lacks: both sides jump to address 0 and fault.

## Harness sensitivity (negative controls)

To prove the differential harness is not comparing one library against itself,
three deliberate bugs were injected into `src/lib.rs`, each built and run, then
reverted (`src/lib.rs` restored byte-identical):

| injected bug | tests that caught it |
|--------------|----------------------|
| `compute_weighted_sum`: `current > base` → `current >= base` | `cfg_weighted_counts_random`, `cfg_weighted_saturating`, `cfg_weighted_single`, `err_weighted_sum_saturation` |
| `arrayfunc`: `param4 / 2` → `param4 >> 1` | all 4 `cfg_arrayfunc_*`, `cfg_pipeline_arrayfunc_replica`, `err_arrayfunc_extreme_params` |
| `safe_double_to_int`: truncate → `d.round()` | 27 tests across every layer |

The first is worth noting: the `arrayfunc`- and pipeline-level tests did **not**
catch it, because `data[0].value` is always 0 by the time `arrayfunc` reaches
`compute_weighted_sum` (the `multiply_operation` pass multiplies slot 0 by its
own `rank`, which is 0), so slot 0's weight is unobservable through the
top-level API. Only the tests that call `compute_weighted_sum` **directly** at
low level expose it — which is exactly why `CONFIGS.md` enumerates the
lowest-level entry points rather than just the one function in `include/lib.h`.
