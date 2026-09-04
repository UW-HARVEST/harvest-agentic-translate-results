# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. The C code contains **no** error enums,
no `assert`, no `errno` use, no `RETURN_ERROR` macro and no `NULL` returns (nothing
returns a pointer). Every rejection is either an early `return` of a sentinel value
or an implicit "loop body never runs" guard. Grep basis:

```
$ grep -n 'return\|assert\|NULL\|INT32_M\|?\s*:\|if (' c_src/src/lib.c
```

Distinct rejection / guard branches (one row each):

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| 1  | `modulo_operation` | `b == 0` (`if (b == 0) return 0;`) | returns `0`, no SIGFPE |
| 2  | `modulo_operation` | `a == INT32_MIN, b == -1` (hardware `idiv` overflow case, *not* guarded by C — the `b == 0` check does not cover it) | **SIGFPE / core dump.** Measured, not assumed: `dlopen`+call on the real C `.so` dies with `Floating point exception (core dumped)`, exit 136. Undefined behaviour in C. **Not exercised** (would kill the harness); see note below. |
| 3  | `safe_double_to_int` | `d >= (double)INT32_MAX` — incl. `2147483647.0`, `2147483648.0`, `1e300`, `+INFINITY` | returns `INT32_MAX` (2147483647) |
| 4  | `safe_double_to_int` | `d <= (double)INT32_MIN` — incl. `-2147483648.0`, `-2147483649.0`, `-1e300`, `-INFINITY` | returns `INT32_MIN` (-2147483648) |
| 5  | `safe_double_to_int` | `d != d`, i.e. `NaN` (quiet **and** signalling, `+NaN`/`-NaN`) — reached only *after* rows 3 & 4 both fail | returns `0` |
| 6  | `safe_double_to_int` | `-0.0`, subnormals, values in `(-1, 1)` — no guard fires, plain C truncation toward zero | returns `0` |
| 7  | `compute_scaled_value` | product overflows / is non-finite (`base=INT32_MAX, scale=1e10`; `scale=INFINITY`; `base=0, scale=INFINITY` → `0*inf = NaN`) | delegates to `safe_double_to_int`: `INT32_MAX` / `INT32_MIN` / `0` |
| 8  | `compare_results_in_array` | `idx1 >= arr->count` (upper-bound guard, first clause) | returns `0` |
| 9  | `compare_results_in_array` | `idx2 >= arr->count` (upper-bound guard, second clause) | returns `0` |
| 10 | `compare_results_in_array` | `arr->count <= 0` (both clauses fire for every non-negative index) | returns `0` |
| 11 | `compare_results_in_array` | negative `idx1`/`idx2` — **deliberately not rejected** by C (only `>=` is checked); address arithmetic is performed on out-of-bounds pointers | `-1`/`0`/`1` per address order of `data+idx1` vs `data+idx2` |
| 12 | `compare_results_in_array` | `idx1 == idx2` (both in range) — equal addresses, neither `<` nor `>` | returns `0` |
| 13 | `compare_results_in_array` | `idx1 > idx2` (both in range) | returns `1` |
| 14 | `compare_results_in_array` | `idx1 < idx2` (both in range) | returns `-1` |
| 15 | `init_result_array` | `count >= 10` (over the `Result data[10]` capacity) — clamped by `count < 10 ? count : 10` | `arr->count = 10`; only 10 elements written, `values[10..]` never read |
| 16 | `init_result_array` | `count == 0` | `arr->count = 0`; loop body never runs; `data[]` untouched |
| 17 | `init_result_array` | `count < 0` (e.g. `-1`, `INT32_MIN`) — **not rejected**; `count < 10` is true so the negative value is stored verbatim | `arr->count = count` (negative); loop `i < count` never runs; `data[]` untouched |
| 18 | `init_result_array` | `count == 10` exactly (boundary: `10 < 10` false → clamp to 10) | `arr->count = 10`, identical to row 15 |
| 19 | `process_with_foreach` | `arr->count == 0` — `FOREACH` guard `count_iter != size` is false immediately | returns `0`, `data[]` untouched |
| 20 | `process_with_foreach` | `arr->count < 0` — the macro terminates on `!=`, **not** `<`, so `count_iter` (0,1,2,…) never equals a negative `size` ⇒ unbounded run-away loop past the array end | non-terminating / memory corruption in C. **Not exercised** (undefined behaviour, would crash the harness); documented so the Rust reproduces the same `!=` loop condition rather than a "safe" `<`. |
| 21 | `process_with_foreach` | `op == NULL` — no null check; C calls through a null function pointer | SIGSEGV. **Not exercised** (would kill the harness); Rust must likewise not silently succeed. |
| 22 | `process_with_foreach` | `op` returns a value whose `*0.75` is out of `int` range (`op = multiply_operation` with huge `value`) | per-item `value` saturates via `safe_double_to_int`; `total` wraps two's-complement |
| 23 | `compute_weighted_sum` | `arr->count <= 0` | returns `0` |
| 24 | `compute_weighted_sum` | `i == 0` — `current == base`, so `current > base` is false and the weight is `1`, **not** `0` | element 0 contributes `value * 1 * 0.8` |
| 25 | `compute_weighted_sum` | `value * weight * 0.8` out of `int` range | contribution saturates via `safe_double_to_int`; `sum` wraps |
| 26 | `arrayfunc` | `param4 == INT32_MIN` → `param4 / 2 + 1` (no trap: divisor is the constant 2) | `-1073741823` stored in `values[7]` |
| 27 | `arrayfunc` | `param1 + param2`, `param2 - param3`, `param3 * 2` overflow (`INT32_MAX`, `INT32_MIN` inputs) | two's-complement wrap (gcc/clang codegen) |
| 28 | `arrayfunc` | any input — `arr.count` is 8 (clamped from 8 < 10), so the `arr.count - 1` compare loop runs 7 times and each `compare_results_in_array(i, i+1)` returns `-1` | fixed `-7` contribution before the `*0.333` scaling |
| — | *all pointer-taking functions* | `arr == NULL` (`compare_results_in_array`, `init_result_array`, `process_with_foreach`, `compute_weighted_sum`), `values == NULL` with `count > 0` | SIGSEGV in C; no guard exists. **Not exercised** (would kill the harness). Rust must not add a guard that would turn the crash into a value. |

## Checklist

- [x] 1  `modulo_operation` b == 0
- [n/a] 2 `modulo_operation` INT_MIN % -1 — UB, C raises SIGFPE (verified experimentally)
- [x] 3  `safe_double_to_int` >= INT32_MAX
- [x] 4  `safe_double_to_int` <= INT32_MIN
- [x] 5  `safe_double_to_int` NaN
- [x] 6  `safe_double_to_int` -0.0 / subnormal / |d| < 1
- [x] 7  `compute_scaled_value` overflow / non-finite
- [x] 8  `compare_results_in_array` idx1 out of range
- [x] 9  `compare_results_in_array` idx2 out of range
- [x] 10 `compare_results_in_array` count <= 0
- [x] 11 `compare_results_in_array` negative indices accepted
- [x] 12 `compare_results_in_array` idx1 == idx2
- [x] 13 `compare_results_in_array` idx1 > idx2
- [x] 14 `compare_results_in_array` idx1 < idx2
- [x] 15 `init_result_array` count > 10 clamp
- [x] 16 `init_result_array` count == 0
- [x] 17 `init_result_array` count < 0
- [x] 18 `init_result_array` count == 10 boundary
- [x] 19 `process_with_foreach` count == 0
- [n/a] 20 `process_with_foreach` count < 0 — UB, would not terminate; loop condition verified by inspection (`!=`, not `<`)
- [n/a] 21 `process_with_foreach` op == NULL — UB, SIGSEGV
- [x] 22 `process_with_foreach` saturating item values
- [x] 23 `compute_weighted_sum` count <= 0
- [x] 24 `compute_weighted_sum` weight == 1 at i == 0
- [x] 25 `compute_weighted_sum` saturating contribution
- [x] 26 `arrayfunc` param4 == INT32_MIN
- [x] 27 `arrayfunc` value-array overflow wrap
- [x] 28 `arrayfunc` fixed -7 comparison contribution
- [n/a] —  NULL pointers — UB, SIGSEGV

### `modulo_operation(INT32_MIN, -1)` — deliberate, documented divergence

This is the one input on which the two libraries do **not** produce the same
observable outcome, and it is unavoidable:

* C: `INT32_MIN % -1` overflows the x86-64 `idiv` instruction → `#DE` → **SIGFPE**,
  process death. There is no return value to match.
* Rust: `wrapping_rem` yields `0` and returns normally.

Every alternative was considered and rejected:
`a % b` in Rust emits an unconditional overflow check that panics, and with
`panic = "abort"` that is SIGABRT (6), *not* SIGFPE (8) — so it would still not
match, while additionally making the function abort for a caller who today gets a
value. Reproducing signal 8 exactly would require hand-written `idiv` inline asm,
which is non-portable and cannot be justified for a UB input. The chosen behaviour
(return `0`, never crash) is therefore the closest defensible match: it agrees with
C on every input for which C has *any* defined result, and it is memory-safe on the
one input where C is not. Tests deliberately exclude this single `(a, b)` pair
(`is_idiv_trap()` in `tests/common/mod.rs` filters it out of every generator, and
`process_with_foreach` tests never place `-1` in a `rank` alongside `INT32_MIN` in
the matching `value`).

Rows marked `n/a` are the cases where the C itself has **no** defined behaviour
(null dereference / runaway loop). They are documented rather than executed: calling
them would abort the test process, and matching "both segfault" adds no signal.
Every such row was instead verified by source inspection that the Rust performs the
same unchecked operation (no added guard, `!=` loop condition preserved).

### Note on out-of-range enum values

The C API contains **no enums** (grep: no `enum` keyword in `c_src/`), so there is
no invalid-enum-across-FFI case. The analogous "any int is accepted" surface is the
`int` index/count parameters of `compare_results_in_array` and `init_result_array`,
which are covered by rows 8–18 including negative and `INT32_MIN`/`INT32_MAX` values,
and the arbitrary function pointer of `process_with_foreach`, covered in `CONFIGS.md`
(rows include the C `.so`'s own operations, the Rust `.so`'s operations, and a
harness-supplied Rust callback).
