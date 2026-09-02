# ERRORS.md — error / rejection surface table (Phase C gate)

Mechanically derived from `c_src/src/lib.c`. There are **no** `assert`s, no error
enums, no `RETURN_ERROR`-style macros and no `errno` use in this library, so the
complete set of rejection mechanisms is:

* `return -1;` — two sites (`arity` short length, `compare_allocations` OOM)
* guard-`if` whose false branch silently does nothing (`shift_array`)
* `if (*str)` false → `return 0` (`process_string`)
* `switch` `default:` → identity return (`apply_bitmask`)
* implicit rejections: absent NULL checks, absent range checks

Greps used:

```sh
grep -n 'return -1\|return 0\|return NULL\|assert\|default:\|if (' c_src/src/lib.c
```

Every row below has a differential test in `tests/errors.rs` (or, where noted,
`tests/valid_paths.rs`) that constructs the exact condition, calls BOTH the C and
the Rust `.so`, and asserts the same sentinel/value is returned.

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| 1 | `arity` | `len == 0` (`len < 2`, `params` may be anything) | `-1`, `params` never dereferenced | `err_arity_len_below_two` | [x] |
| 2 | `arity` | `len == 1` (`len < 2`) | `-1`, `params` never dereferenced | `err_arity_len_below_two` | [x] |
| 3 | `arity` | `len == 0` **and** `params == NULL` — no NULL check exists, but the short-length branch returns before any load | `-1`, no crash | `err_arity_null_params_short_len` | [x] |
| 4 | `arity` | `len == 1` **and** `params == NULL` | `-1`, no crash | `err_arity_null_params_short_len` | [x] |
| 5 | `arity` | `len == 256` passed through the header's `int` parameter — the definition takes `unsigned char`, so the low byte is `0` | `-1` (aliases row 1) | `err_arity_int_truncation` | [x] |
| 6 | `arity` | `len == 257` (low byte `1`) | `-1` (aliases row 2) | `err_arity_int_truncation` | [x] |
| 7 | `arity` | `len == -256` / `0xFFFFFF00` (low byte `0`) | `-1` | `err_arity_int_truncation` | [x] |
| 8 | `arity` | `len == -1` (low byte `255`, i.e. **not** `< 2` under the unsigned byte compare) → falls to the `else` branch and reads 4 ints | `arity4(p[0..4])`, **not** an error | `err_arity_negative_len_is_not_rejected` | [x] |
| 9 | `arity` | `len == 258` (low byte `2`) | `arity2(p[0], p[1])`; `p[2]`, `p[3]` untouched | `err_arity_int_truncation` | [x] |
| 10 | `arity` | `len` one step past each dispatch boundary: `4`, `5`, `127`, `128`, `255` | all take the `arity4` branch (`len` is not `2` or `3` and not `< 2`) | `err_arity_len_sweep_all_256` | [x] |
| 11 | `arity` | every `len` in `0..=255` plus every `int` whose low byte is that value (`len`, `len+256`, `len-256`, `len+65536`) | result depends **only** on the low byte | `err_arity_len_sweep_all_256` | [x] |
| 12 | `compare_allocations` | `ptr1 == NULL \|\| ptr2 == NULL` (allocation failure) → `free(ptr1); free(ptr2); return -1;` | `-1` | `err_compare_allocations_oom_branch_unreachable` (documents that `malloc(4)` cannot fail here; the branch is dead in both impls and both `free(NULL)` calls would be no-ops) | [x] |
| 13 | `compare_allocations` | `ptr1 == ptr2` → `result = 3` | unreachable: two simultaneously-live allocations cannot share an address, so `3` is dead code in C and in Rust alike | `err_compare_allocations_never_returns_three` | [x] |
| 14 | `compare_allocations` | `val1 <= 0` — the `(*uninit_ptr > 0)` ternary rejects the `+10` bonus (`val1 == 0`, `val1 == -1`, `val1 == INT_MIN`) | `1` or `2` (address ordering only), never `11`/`12` | `err_compare_allocations_nonpositive_val1` | [x] |
| 15 | `shift_array` | `positions == 0` → guard `positions > 0` false | no-op, array unchanged | `err_shift_array_noop_guards` | [x] |
| 16 | `shift_array` | `positions < 0` (`-1`, `-size`, `INT_MIN`) → guard `positions > 0` false | no-op | `err_shift_array_noop_guards` | [x] |
| 17 | `shift_array` | `positions == size` → guard `positions < size` false | no-op | `err_shift_array_noop_guards` | [x] |
| 18 | `shift_array` | `positions > size` (incl. `INT_MAX`) → guard `positions < size` false | no-op, **no** out-of-bounds write | `err_shift_array_noop_guards` | [x] |
| 19 | `shift_array` | `size == 0` with any `positions` → `positions < 0 == false` for `positions > 0` | no-op; safe with a zero-length buffer | `err_shift_array_zero_and_one_size` | [x] |
| 20 | `shift_array` | `size < 0` (`-1`, `INT_MIN`) with `positions > 0` → `positions < size` false | no-op | `err_shift_array_negative_size` | [x] |
| 21 | `shift_array` | `size == 1`, `positions == 1` → `positions < size` false | no-op | `err_shift_array_zero_and_one_size` | [x] |
| 22 | `shift_array` | `size == 0` **and** `arr == NULL` — no NULL check exists, but the guard short-circuits before any load | no-op, no crash | `err_shift_array_null_when_guard_fails` | [x] |
| 23 | `shift_array` | `positions <= 0` **and** `arr == NULL` | no-op, no crash | `err_shift_array_null_when_guard_fails` | [x] |
| 24 | `process_string` | `str` points at `'\0'` (empty string) → `if (*str)` false | `0` (the `strlen` call is skipped) | `err_process_string_empty` | [x] |
| 25 | `process_string` | string whose first byte is `'\0'` but which has non-NUL bytes after it | `0` — only the first byte is tested | `err_process_string_embedded_nul_first` | [x] |
| 26 | `apply_bitmask` | `operation` outside the 0–3 `switch` labels, i.e. an out-of-range "enum" value crossing the FFI boundary: `4`, `5`, `-1`, `-4`, `255`, `256`, `INT_MAX`, `INT_MIN` | `default:` → returns `value` unchanged | `err_apply_bitmask_out_of_range_operation` | [x] |
| 27 | `apply_bitmask` | `operation` one step past each valid label (`-1` below `0`, `4` above `3`) | identity | `err_apply_bitmask_out_of_range_operation` | [x] |
| 28 | `arity4` (via `param1 % 4`) | `param1 < 0` makes `param1 % 4` **negative** (C truncating remainder: `-1 % 4 == -1`), which is not a valid `switch` label → `apply_bitmask` `default:` path | mask left unapplied for every negative `param1` not divisible by 4 | `err_arity4_negative_modulo_hits_default` | [x] |
| 29 | `arity4` | `param3 == 0` → the `(result * param3) / 100` statement is skipped entirely (this is the only thing protecting against a `* 0` collapse) | `result` unscaled | `valid_arity4_param3_zero_vs_nonzero` | [x] |
| 30 | `arity4` | `param4 == 0` → the `result += param4` statement is skipped (indistinguishable from adding 0, but it is a distinct branch) | `result` unchanged | `valid_arity4_param4_zero_vs_nonzero` | [x] |
| 31 | `arity4` | signed-integer overflow in `result += ...`, `result * param3`, `result += param4` (e.g. `INT_MAX`, `INT_MIN`) — UB in C, wraps two's-complement as compiled at `-O0` | wrapped value | `err_arity4_overflow_wraps` | [x] |
| 32 | `arity4` | `(result * param3) / 100` with a negative numerator — C integer division truncates toward zero | truncation toward zero, not floor | `err_arity4_division_truncates_toward_zero` | [x] |
| 33 | `init_matrix` | no validation of any kind exists (no NULL check, no bounds check); the only requirement is a ≥ 3×4 `int` buffer | writes exactly 12 ints, never more | `err_init_matrix_writes_exactly_twelve` | [x] |
| 34 | `process_string` / `arity` / `init_matrix` | genuine NULL dereference (`process_string(NULL)`, `arity(2, NULL)`, `init_matrix(NULL)`) | both implementations dereference NULL and take `SIGSEGV`; no rejection exists to compare | `err_null_deref_is_symmetric_documented` (asserts the pointer-load *ordering* preconditions and documents why the crash itself is not asserted in-process) | [x] |

Notes on rows deliberately marked unreachable (12, 13): the instructions forbid
inventing behavior, and both branches are provably unreachable given
`malloc(4)`-sized requests, so the tests assert the *observable consequence*
(the sentinel never appears from either `.so` across the whole randomized sweep)
rather than faking an allocation failure.
