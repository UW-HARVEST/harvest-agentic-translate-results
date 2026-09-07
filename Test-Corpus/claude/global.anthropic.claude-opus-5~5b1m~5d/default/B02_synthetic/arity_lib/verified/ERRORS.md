# ERRORS.md — Phase A error-surface table

Mechanically derived from `c_src/src/lib.c`. Every `return -1`, every
guard whose false branch skips work, every `default:` label, every
`NULL` check, every implicit range/bounds assumption is one row.

This library has **no error enum, no `RETURN_ERROR` macro, no `assert`,
and no `errno` use**. Its entire rejection surface consists of:
* one sentinel `-1` from `arity` (`len < 2`),
* one sentinel `-1` from `compare_allocations` (allocation failure),
* several *silent* rejections where a guard is false and the function
  simply does nothing / returns the input unchanged.

Silent rejections are included because they are observable rejections of
invalid input and are exactly the paths a happy-path test misses.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `arity` | `len` truncates to `0` (e.g. `0`, `256`, `512`, `0x100`) | returns `-1`, `params` never dereferenced | `err_01_arity_len_trunc_zero` | [x] |
| 2 | `arity` | `len` truncates to `1` (e.g. `1`, `257`, `0x101`) | returns `-1`, `params` never dereferenced | `err_02_arity_len_trunc_one` | [x] |
| 3 | `arity` | `len < 2` guard is reached with `params == NULL` and `len` in `{0,1,256,257}` | returns `-1` without a null-deref (guard runs first) | `err_03_arity_null_params_is_safe` | [x] |
| 4 | `arity` | `len` is negative as an `int` (e.g. `-1 -> 255`, `-256 -> 0`, `-255 -> 1`) — `mov %al` + unsigned `cmpb` | `-1` iff low byte `< 2`, else dispatches (`255 -> arity4`) | `err_04_arity_negative_len` | [x] |
| 5 | `arity` | `len` low byte `>= 4` (e.g. `4..255`, `1000 -> 232`) — falls into the `else` | dispatches to `arity4`, reads `params[0..4]` (**never** more, whatever `len` says) | `err_05_arity_large_len_reads_only_4` | [x] |
| 6 | `compare_allocations` | `ptr1 == NULL \|\| ptr2 == NULL` (`malloc` failure) | `free(ptr1); free(ptr2); return -1` | `err_06_compare_alloc_oom_unreachable` (documented; not inducible) | [x] |
| 7 | `compare_allocations` | `*ptr1 <= 0`, i.e. `val1 <= 0` — ternary `(*uninit_ptr > 0)` is false | adds `0` instead of `10` (result `1`/`2`/`3`, not `11`/`12`/`13`) | `err_07_compare_alloc_nonpositive_val1` | [x] |
| 8 | `compare_allocations` | `ptr1 == ptr2` — the `else { result = 3; }` arm | `3` (or `13`); unreachable with a real allocator, but the arm must exist | `err_08_compare_alloc_equal_arm` (documented; not inducible) | [x] |
| 9 | `process_string` | `*str == 0` (empty string) — `if (*str)` false | returns `0` (does **not** call `strlen`) | `err_09_process_string_empty` | [x] |
| 10 | `process_string` | `str == NULL` | **unconditional deref of `*str` -> SIGSEGV in both** (no null check in C) | `err_10_process_string_null_segv` (subprocess, both must die identically) | [x] |
| 11 | `apply_bitmask` | `operation` has no matching `case` — the `default:` label. Includes `4`, `5`, `255`, `-1`, `INT_MIN`, `INT_MAX` (out-of-range "enum" values across FFI) | returns `value` **unchanged** | `err_11_apply_bitmask_default` | [x] |
| 12 | `apply_bitmask` | `operation` in `{-1,-2,-3}` — reachable from `arity4` because C `param1 % 4` is **negative** for negative `param1` | `default:` -> `value` unchanged (NOT the `case 1/2/3` a "fixed" `rem_euclid` would pick) | `err_12_negative_modulo_hits_default` | [x] |
| 13 | `shift_array` | `positions <= 0` (`0`, `-1`, `INT_MIN`) — first half of the guard | array left **completely unmodified**, no memmove, no zero-fill | `err_13_shift_positions_nonpositive` | [x] |
| 14 | `shift_array` | `positions >= size` (`positions == size`, `positions > size`) — second half of the guard | array left **completely unmodified** | `err_14_shift_positions_ge_size` | [x] |
| 15 | `shift_array` | `size <= 0` (`0`, `-1`) with any `positions` | guard false (`positions < size` cannot hold for `positions > 0`) -> unmodified | `err_15_shift_nonpositive_size` | [x] |
| 16 | `shift_array` | `size` larger than the real buffer (caller lies) | C memmoves out of bounds; **not tested** (true UB / heap corruption, would corrupt the harness) | n/a — deliberately excluded | [x] |
| 17 | `arity4` | `param3 == 0` — `if (param3 != 0)` false | the `* param3 / 100` step is **skipped entirely** (no divide-by-zero, no zeroing) | `err_17_arity4_param3_zero` | [x] |
| 18 | `arity4` | `param4 == 0` — `if (param4 != 0)` false | the `+= param4` step is skipped (no observable difference, but the branch is covered) | `err_18_arity4_param4_zero` | [x] |
| 19 | `arity4` | `result * param3` overflows `int` (e.g. `param1=INT_MAX`, `param3=INT_MAX`) — signed-overflow UB in C | must match C's actual emitted behaviour: 2's-complement wraparound (`imul`), then `idiv` by 100 | `err_19_arity4_mul_overflow` | [x] |
| 20 | `arity4` | `result + param4` overflows `int` (`param4 = INT_MAX/INT_MIN`) | 2's-complement wraparound (`add`) | `err_20_arity4_add_overflow` | [x] |
| 21 | `arity4` | negative `(result * param3)` divided by `100` | C `/` truncates **toward zero** (`idiv`), not floor | `err_21_arity4_negative_division` | [x] |
| 22 | `arity4` | `param1 = INT_MIN` -> `param1 % 4` | `0` (the UB case is `% -1`, not `% 4`); must not panic | `err_22_arity4_param1_int_min` | [x] |
| 23 | `arity2` / `arity3` | pass the implicit `param3 = 0` / `param4 = 0` defaults | must equal `arity4(p1,p2,0,0)` / `arity4(p1,p2,p3,0)` exactly | `err_23_arity2_arity3_defaults` | [x] |
| 24 | `init_matrix` | `matrix == NULL` | unconditional store -> SIGSEGV in both (no null check in C) | `err_24_init_matrix_null_segv` (subprocess) | [x] |
| 25 | `shift_array` | `arr == NULL` with a *guard-passing* `positions`/`size` (e.g. `size=4, positions=1`) | memmove on NULL -> SIGSEGV in both | `err_25_shift_array_null_segv` (subprocess) | [x] |
| 26 | `shift_array` | `arr == NULL` with a *guard-failing* `positions` (e.g. `positions=0`) | guard short-circuits first -> **returns safely**, no deref | `err_26_shift_array_null_guarded_safe` | [x] |

## Deliberate exclusions

* Row 16 (`size` beyond the buffer) and any "lie about the `params`
  length" variant are genuine out-of-bounds writes/reads. Both
  implementations would corrupt the same way; exercising them in-process
  would corrupt the test harness rather than measure a difference.
  Row 5 covers the *safe* half of this (`arity` ignoring an oversized
  `len`) because there `arity4` only ever touches `params[0..4]`.
* Rows 6 and 8 are unreachable with a working glibc allocator
  (`malloc(4)` twice never returns NULL here, and never returns the same
  address twice). They are recorded because they are real branches; the
  tests assert the *reachable* invariant instead (result is never `-1`,
  and never `3`/`13`) for both libraries identically.
