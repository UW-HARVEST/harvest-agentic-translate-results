# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return`, every
`if`/`else if`, every `switch`/`case`/`default`, every `assert`, every `NULL`
check, and every limit constant (`INT_MAX`, `INT_MIN`, `sizeof`, `isnan`).

Mechanical findings:

* `assert` occurrences: **0**
* `NULL` occurrences: **0** — no function validates its pointer arguments
* error-return macros (`RETURN_ERROR`, error enums, `errno` sets): **0**
* `return` sites: 7 (lines 41, 43, 45, 47, 74, 89, 156)

This library has **no error codes and no error enum**. Its entire rejection
surface consists of *saturating / sentinel* returns: the three guard branches
in `safe_double_to_int`, and the `default:` arm of `process_with_fallthrough`
which returns the sentinel `-1`. Those are the rows below, one row per distinct
rejecting branch. Rows 8–14 are the generic FFI-boundary boundaries required
regardless of the table (null pointers, out-of-range "enum-like" ints, values
one step past a documented range).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `safe_double_to_int` | `d > (double)INT_MAX` — line 40; e.g. `2147483648.0`, `1e15`, `+INFINITY`, `DBL_MAX` | returns `INT_MAX` (`2147483647`) |
| 2 | `safe_double_to_int` | `d < (double)INT_MIN` — line 42; e.g. `-2147483649.0`, `-1e15`, `-INFINITY`, `-DBL_MAX` | returns `INT_MIN` (`-2147483648`) |
| 3 | `safe_double_to_int` | `isnan(d)` — line 44. Reached only because both range compares are false for NaN; the NaN test deliberately comes *after* them | returns `0` |
| 4 | `safe_double_to_int` | boundary one step *inside* the high guard: `d == 2147483647.0` (`> INT_MAX` is false) | returns `2147483647` via `(int)d`, **not** the guard |
| 5 | `safe_double_to_int` | boundary one step *inside* the low guard: `d == -2147483648.0` (`< INT_MIN` is false) | returns `-2147483648` via `(int)d`, **not** the guard |
| 6 | `safe_double_to_int` | boundary one step *past* the high guard: `nextafter(2147483647.0, +inf)` = `2147483647.0000002` | returns `INT_MAX` (guard taken) |
| 7 | `safe_double_to_int` | boundary one step *past* the low guard: `nextafter(-2147483648.0, -inf)` | returns `INT_MIN` (guard taken) |
| 8 | `process_with_fallthrough` | `default:` arm — line 69; `code` has no matching `case`, i.e. any `code < 0` or `code > 5` (e.g. `-1`, `6`, `7`, `INT_MAX`, `INT_MIN`) | returns the sentinel `-1`, ignoring `base_value` |
| 9 | `process_with_fallthrough` | out-of-range "enum-like" int across FFI: `code` = `INT_MAX`, `INT_MIN`, `0x7fffffff`, `-2147483648` | returns `-1` (`default:`) |
| 10 | `process_with_fallthrough` | `case 0:` — the value-discarding arm; `base_value` is overwritten regardless of magnitude (even `INT_MAX`) | returns `0` |
| 11 | `process_with_fallthrough` | signed-overflow-adjacent input: `code` in `1..5` with `base_value` near `INT_MAX` (e.g. `INT_MAX`, `INT_MAX-25`) — C performs unchecked `result += N` with **no** range check | returns the two's-complement wrapped sum (reference build wraps) |
| 12 | `copy_data_block` | `dest == NULL` or `src == NULL` — **no null check exists** (0 `NULL` occurrences); C dereferences unconditionally via `memcpy` | undefined behaviour / SIGSEGV. Not differentially testable; asserted only that **neither** implementation contains a validating branch (both crash identically). Excluded from the executed suite by design — see `NOTE-null` below |
| 13 | `overunder` | no argument validation at all; `d*d + a*a` (line 108) overflows `int` for large \|a\|,\|d\| and can go **negative**, making `sqrt` receive a negative operand | `sqrt(negative)` = `NaN`, which then flows into row 3 and yields `conv4 == 0`; `overunder` still returns a value (no rejection) |
| 14 | `overunder` | `a` negative ⇒ `a % 6` is negative (C `%` truncates toward zero) ⇒ `process_with_fallthrough` takes `default:` ⇒ `switch_result == -1` | contributes `-1`; combined with row 8 |

## NOTE-null

Rows for null pointers are required by the generic-boundary rule, so they are
enumerated above. `copy_data_block` is the only pointer-taking export and it
performs **zero** validation, so passing `NULL` is undefined behaviour in the C
and would abort the test process for *both* libraries rather than produce a
comparable value. The differential test therefore verifies the *observable*
consequence of "no null check" — that both libraries copy all
`sizeof(DataBlock)` bytes unconditionally, including padding, for every
non-null aliasing/offset shape — and documents that neither side added a
defensive check that the other lacks. Deliberately dereferencing `NULL` is not
executed.

## Test mapping

| ERRORS.md row | test in `tests/phase_c_errors.rs` | result |
|---|---|---|
| 1 | `err01_sdti_above_int_max_saturates` | [x] |
| 2 | `err02_sdti_below_int_min_saturates` | [x] |
| 3 | `err03_sdti_nan_returns_zero` | [x] |
| 4 | `err04_sdti_exactly_int_max_uses_cast_not_guard` | [x] |
| 5 | `err05_sdti_exactly_int_min_uses_cast_not_guard` | [x] |
| 6 | `err06_sdti_one_ulp_past_high_guard` | [x] |
| 7 | `err07_sdti_one_ulp_past_low_guard` | [x] |
| 8 | `err08_pwf_default_returns_minus_one` | [x] |
| 9 | `err09_pwf_out_of_range_enum_values` | [x] |
| 10 | `err10_pwf_case_zero_discards_base` | [x] |
| 11 | `err11_pwf_unchecked_add_wraps_identically` | [x] |
| 12 | `err12_copy_data_block_has_no_null_check` (forked children, termination signal compared) | [x] |
| 13 | `err13_overunder_negative_sqrt_operand_yields_zero_conv4` | [x] |
| 14 | `err14_overunder_negative_a_hits_default_arm` | [x] |
| generic boundaries | `err15_generic_scalar_extremes_every_entry_point` | [x] |

Every test asserts the *specific* sentinel the C source mandates (`INT_MAX`,
`INT_MIN`, `0`, `-1`, or an exact wrapped value), never merely that both
implementations "failed somehow".

### Note on zero / oversized lengths

The API has **no** length, size, or count parameter: `copy_data_block`'s length
is the compile-time constant `sizeof(DataBlock)`. That generic boundary is
therefore discharged by proving both implementations write exactly
`sizeof(DataBlock)` = 40 bytes and touch nothing beyond it
(`row18_copy_poisoned_destination_bounds`), plus hammering the integer extremes
of every scalar parameter of every entry point (`err15`).

### Fix applied

Row 12 initially **failed** in the `dev` profile: the Rust
`ptr::copy_nonoverlapping` raised a precondition panic (SIGABRT) on a NULL
argument while the C `memcpy` faulted with SIGSEGV. Fixed by calling libc
`memcpy` directly in the translation, as the C does. Both profiles now agree.
