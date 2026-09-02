# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return`,
`if`, `switch`/`default:`, comparison, `NULL` check, `assert`, and every
implicit-rejection constant.

Facts about this library, established by that grep:

- There is **no** `RETURN_ERROR`-style macro, **no** `assert`, **no**
  `errno` use, and **no** argument null-check anywhere in the file.
- The `StatusCode` enum declares `STATUS_ERROR = -1` and `STATUS_WARNING = 1`,
  but **neither is ever assigned**; the only value written to
  `ComputationResult.status` is `STATUS_SUCCESS` (0). Rows 14/15 record this.
- The only hard-coded limit constants are `10` (history capacity, appears
  twice: `allocate_results(10)` and `*history_count < 10`), `128`
  (`param1 % 128`), `5` (`% 5` for both operation selections), `29` (timestamp
  shift), `100` (timestamp modifier), and the `'1'`..`'5'` char range.
- Rejection in this library is **always silent**: a sentinel `0`, a fallback
  function pointer, a fallback character, a skipped write, or a `NULL` from
  `calloc`. No path returns a negative error code.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `is_valid_operation` | `op_char == 0` (the `op_char &&` short-circuit; `'\0'` is also `< '1'`) | returns `false` |
| 2 | `is_valid_operation` | `op_char < '1'` (0x31) and non-zero, e.g. `'0'`, `' '`, `-1`, `-128` | returns `false` |
| 3 | `is_valid_operation` | `op_char > '5'` (0x35), e.g. `'6'`, `'A'`, `0x7f` | returns `false` |
| 4 | `divide_operation` | `b == 0` (`if (b == 0) return 0;`, line 75-77) | returns `0`, no trap |
| 5 | `modulo_operation` | `b == 0` (`if (b == 0) return 0;`, line 82-84) | returns `0`, no trap |
| 6 | `divide_operation` | `a == INT_MIN && b == -1` — reaches `a / b`; `idiv` overflow | process dies on `SIGFPE` (signal 8) |
| 7 | `modulo_operation` | `a == INT_MIN && b == -1` — reaches `a % b`; `idiv` overflow | process dies on `SIGFPE` (signal 8) |
| 8 | `select_operation` | `op` not in `1..=5` — hits `default:` (line 100). Covers `0`, `6`, `-1`, `-2`, `INT_MIN`, `INT_MAX`, and any out-of-range enum int passed over FFI | returns `add_operation` (**not** `NULL`) |
| 9 | `allocate_results` | `count < 0` → sign-extended to a huge `size_t` in `calloc(count, 24)` | `calloc` fails → returns `NULL` (unchecked by C) |
| 10 | `allocate_results` | `count` so large that `count * 24` exceeds available memory (e.g. `INT_MAX`) | `calloc` fails → returns `NULL` (unchecked by C) |
| 11 | `allocate_results` | `count == 0` → `calloc(0, 24)` | glibc returns a **non-NULL** unique pointer |
| 12 | `perform_computation_with_history` | `*history == NULL` (line 122) — the "uninitialised state" condition | allocates 10 slots, resets `*history_count = 0`, then proceeds |
| 13 | `perform_computation_with_history` | `*history_count >= 10` (line 127 guard fails) — history full | result still returned, but **no** slot written and `*history_count` **not** incremented |
| 14 | `perform_computation_with_history` | any successful record | `status` field is always `STATUS_SUCCESS` (0); `STATUS_ERROR`/`STATUS_WARNING` are unreachable |
| 15 | `perform_computation_with_history` | `op` out of `1..=5` (delegates to row 8) | silently performs **addition** |
| 16 | `perform_computation_with_history` | `history == NULL` or `history_count == NULL` (the `**`/`*` out-params) | unchecked → dereferences NULL → `SIGSEGV` (signal 11) |
| 17 | `mathop` | `!is_valid_operation((char)(param1 % 128))` (line 144) | `validation_char` is silently replaced with `'1'`; the variable is then **dead** — it cannot affect the return value |
| 18 | `mathop` | `param3 < 0` → `param3 % 5` truncates toward zero → `selected_op` in `-3..=0`, out of the enum range | `select_operation` falls to `default:` (addition) and `get_operation_priority` returns a **negative/zero** priority (`op * 10`) |
| 19 | `mathop` | `param4 == INT_MAX` → `param4 + 1` signed overflow (wraps to `INT_MIN` as built) → `second_op` = `-2` | out-of-range op → addition; no diagnostic |
| 20 | `mathop` | `param1 == INT_MIN` → `param1 % 128` (no `idiv` overflow since `128 != -1`) | `0` → `validation_char = '\0'` → invalid (row 1) → replaced by `'1'` |
| 21 | `mathop` | called more than 5 times in one process (each call records 2 entries into the 10-slot static history) | from the 6th call on, row 13 applies: `history_count` sticks at `10`; **the return value is unaffected** because `perform_computation_with_history` returns `math_func(...)` regardless |
| 22 | `mathop` (via `divide_operation`) | `param3 % 5 + 1 == OP_DIVIDE` and `param2 == 0` | inner divide returns `0` sentinel (row 4), computation continues |
| 23 | `mathop` (via `modulo_operation`) | `(param4+1) % 5 + 1 == OP_MODULO` and `param4 == 0` | inner modulo returns `0` sentinel (row 5), computation continues |
| 24 | `mathop` (via `divide_operation`) | `selected_op == OP_DIVIDE`, `param1 == INT_MIN`, `param2 == -1` | `SIGFPE` (row 6) propagates out of `mathop` |
| 25 | `add_operation` / `subtract_operation` / `multiply_operation` / `get_operation_priority` | signed integer overflow (`INT_MAX + 1`, `INT_MIN - 1`, `INT_MAX * INT_MAX`, `INT_MAX * 10`) — C UB, but the emitted code wraps two's-complement | wrapped `int` result, no rejection |

## Status

| # | test | passing |
|---|------|---------|
| 1 | `err_01_03_is_valid_operation_rejections` | [x] |
| 2 | `err_01_03_is_valid_operation_rejections` | [x] |
| 3 | `err_01_03_is_valid_operation_rejections` | [x] |
| 4 | `err_04_divide_by_zero` | [x] |
| 5 | `err_05_modulo_by_zero` | [x] |
| 6 | `err_06_07_intmin_div_neg1_sigfpe` | [x] |
| 7 | `err_06_07_intmin_div_neg1_sigfpe` | [x] |
| 8 | `err_08_select_operation_out_of_range` | [x] |
| 9 | `err_09_allocate_results_negative` | [x] |
| 10 | `err_10_allocate_results_oversized` | [x] |
| 11 | `err_11_allocate_results_zero` | [x] |
| 12 | `err_12_history_null_bootstraps` | [x] |
| 13 | `err_13_history_full_no_write` | [x] |
| 14 | `err_14_status_always_success` | [x] |
| 15 | `err_15_pcwh_out_of_range_op_adds` | [x] |
| 16 | `err_16_null_outparams_segv` | [x] |
| 17 | `err_17_mathop_invalid_validation_char_is_dead` | [x] |
| 18 | `err_18_mathop_negative_param3` | [x] |
| 19 | `err_19_mathop_param4_intmax_overflow` | [x] |
| 20 | `err_20_mathop_param1_intmin` | [x] |
| 21 | `err_21_history_saturates_after_five_calls` | [x] |
| 22 | `err_22_mathop_divide_by_zero_path` | [x] |
| 23 | `err_23_mathop_modulo_by_zero_path` | [x] |
| 24 | `err_24_mathop_sigfpe_propagates` | [x] |
| 25 | `err_25_signed_overflow_wraps` | [x] |

## Note on rows 19 / 24 — the second computation can never trap

`mathop`'s second computation uses `op = (param4 + 1) % 5 + 1` with operands
`(intermediate_result, param4)`. It only divides when `(param4 + 1) % 5` is 3
or 4, whereas trapping additionally needs `param4 == -1`, which gives
`(0) % 5 + 1 == OP_ADD`. So the **only** trapping `mathop` tuple is a
divide/modulo *first* computation with `param1 == INT_MIN && param2 == -1`
(row 24). The differential tests use exactly this predicate rather than a
conservative over-filter, so `param4 == -1` is fully covered as a valid input.

## Divergence found and fixed

One real divergence was found by row 16, and only in `dev` builds:

| | C | Rust before | Rust after |
|---|---|---|---|
| `perform_computation_with_history` with a NULL out-param | `SIGSEGV` (11) | `SIGABRT` (6) in `dev`, `SIGSEGV` in `release` | `SIGSEGV` (11) in both |

Cause: the original translation dereferenced the out-parameters with `*history`
/ `*history_count` place projections. Those carry a `debug_assertions`-only
null/alignment UB check that *panics*, and a panic escaping an `extern "C"`
function aborts. The C load simply faults.

Fix: `src/lib.rs` now performs those accesses (and the `ComputationResult`
field stores) through `raw_load32/64` / `raw_store32/64`, which emit the load
or store directly, so the fault is identical in every build profile. The same
change also fixes the alignment half of the class, covered by
`err_generic_misaligned_pointers`: C happily performs unaligned 4/8-byte
accesses on x86-64 and the Rust now does too.
