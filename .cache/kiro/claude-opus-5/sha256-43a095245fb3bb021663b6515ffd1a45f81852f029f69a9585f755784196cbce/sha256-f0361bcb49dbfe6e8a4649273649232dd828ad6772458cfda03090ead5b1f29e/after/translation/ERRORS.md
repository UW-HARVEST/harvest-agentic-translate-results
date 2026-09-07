# ERRORS.md — Error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. The library has no error enum and
no `RETURN_ERROR` macro; every rejection is either a **clamp/sentinel return**
or a **`default:` fallback**. Each distinct rejecting branch in the C source
gets one row. Grep basis:

```
grep -n 'return\|isnan\|default:\|INT_MAX\|INT_MIN\|sizeof\|assert' c_src/src/lib.c
```

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `safe_double_to_int` | `d > (double)INT_MAX` — e.g. `1e15`, `2147483648.0`, `+INFINITY`, `DBL_MAX`, `nextafter(2147483647.0, inf)` | returns `INT_MAX` (2147483647) | [x] |
| 2 | `safe_double_to_int` | `d < (double)INT_MIN` — e.g. `-1e15`, `-2147483649.0`, `-INFINITY`, `-DBL_MAX` | returns `INT_MIN` (-2147483648) | [x] |
| 3 | `safe_double_to_int` | `isnan(d)` (reached only after both range compares fail, since NaN compares false both ways) — quiet NaN, signalling NaN, `-NaN` | returns `0` | [x] |
| 4 | `safe_double_to_int` | in-range boundary `d == (double)INT_MAX` exactly (NOT `>`, so falls through to `(int)d`) | returns `2147483647` | [x] |
| 5 | `safe_double_to_int` | in-range boundary `d == (double)INT_MIN` exactly (NOT `<`, so falls through to `(int)d`) | returns `-2147483648` | [x] |
| 6 | `safe_double_to_int` | `-0.0`, and sub-1.0 magnitudes (`0.9`, `-0.9`) — truncation toward zero, not rounding | returns `0` (incl. `-0.0 -> 0`) | [x] |
| 7 | `process_with_fallthrough` | `code` outside the handled set `{0,1,2,3,4,5}` — i.e. any negative value, or `>= 6`, incl. `INT_MIN`, `INT_MAX`, `6`, `-1` | `result = -1` (the `default:` branch; `base_value` is discarded) | [x] |
| 8 | `process_with_fallthrough` | `code == 0` — input `base_value` is unconditionally discarded | returns `0` | [x] |
| 9 | `process_with_fallthrough` | `base_value` near `INT_MAX` with `code in {1..5}` — signed overflow of `result += N` | wraps (reference build is `-O0`, two's-complement wrap) | [x] |
| 10 | `overunder` | `d*d + a*a` overflows `int` to a **negative** value, so `sqrt()` of a negative double → `NaN` → feeds row 3 | `conv4 == 0`; total computed with `conv4 = 0` | [x] |
| 11 | `overunder` | `a % 6` is negative (any `a < 0` with `a % 6 != 0`) → `process_with_fallthrough` `default:` | `switch_result == -1` (row 7 reached indirectly) | [x] |
| 12 | `overunder` | `a`/`b`/`c`/`d` at `INT_MIN`/`INT_MAX` — signed overflow in `a*a`, `d*d`, `value*2`, `a+b`, and the `total` accumulation | all wrap (two's complement); no trap | [x] |
| 13 | `overunder` (`strncpy` guard) | `strncpy(label, "Source", sizeof(label)-1)` then `label[19] = '\0'` — bounded copy, zero-pads bytes 6..18 | `label` == `"Source"` + 13 NUL bytes + explicit NUL at [19]; `%s` prints `Source` | [x] |
| 14 | `handle_pointer_operations` | `value * 2` overflow (`value > INT_MAX/2` or `< INT_MIN/2`), then `+ 100` overflow | wraps; returns `value*2 + 100` mod 2^32 | [x] |
| 15 | `copy_data_block` | `dest` and/or `src` is `NULL` → `memcpy(NULL, ...)` | UB in C: dies with `SIGSEGV`. **Live-tested**: each call is made in a `fork()`ed child and the two implementations' wait-statuses are compared, so they must fail with the *same signal*, not merely "both failed". Covers `NULL/NULL`, `NULL/valid`, `valid/NULL`, plus a valid-pointer control that must exit 0 for both. | [x] |
| 16 | `copy_data_block` | fully overlapping `dest == src` (C `memcpy` has `restrict` parameters) | Both call the same glibc `memcpy`, so both are the identity for a 40-byte self-copy; asserted over 500 randomized buffers, and the full 40 bytes + 24 guard bytes are compared. | [x] |

Notes on rows deliberately **not** present: there are no `assert()` calls, no
`return NULL`, no error enums, and no negative-length or count parameters
anywhere in `c_src/src/lib.c`. `safe_double_to_int` and
`process_with_fallthrough` are total functions over their input domain — the
"out-of-range enum value across the FFI boundary" class is covered by row 7,
which is exactly a C `switch` receiving an `int` with no matching case label.

## Row → test mapping

All rows live in `tests/phase_c_errors.rs` (rows 4 and 5 share one test):

| rows | test |
|------|------|
| 1 | `err01_sdti_above_int_max_clamps_to_int_max` |
| 2 | `err02_sdti_below_int_min_clamps_to_int_min` |
| 3 | `err03_sdti_nan_returns_zero` |
| 4, 5 | `err04_err05_sdti_exact_limits_fall_through_to_cast` |
| 6 | `err06_sdti_truncates_toward_zero` |
| 7 | `err07_pwf_out_of_range_code_returns_minus_one` |
| 8 | `err08_pwf_code_zero_discards_base_value` |
| 9 | `err09_pwf_accumulator_overflow_matches` |
| 10 | `err10_overunder_sqrt_of_negative_yields_conv4_zero` |
| 11 | `err11_overunder_negative_modulo_hits_default_branch` |
| 12 | `err12_overunder_extreme_arguments` |
| 13 | `err13_label_bounded_and_nul_terminated` |
| 14 | `err14_hpo_overflow_matches` |
| 15 | `err15_null_pointers_fail_identically` |
| 16 | `err16_self_copy_is_identity_for_both` |

Each test asserts the **specific** C sentinel first (e.g. `assert_eq!(cv, -1)`
for row 7, `assert_eq!(cv, 0)` for row 3) and only then asserts Rust equals C,
so a test cannot pass by both sides being wrong in the same way.

## Fix applied during Phase C

Rows 15 and 16 initially diverged when the Rust `.so` was built with
`debug_assertions` on: `copy_data_block` used `std::ptr::copy_nonoverlapping`,
whose debug precondition check aborts on a null or self-overlapping pointer,
whereas the C `memcpy` faults (row 15) or is a no-op (row 16). The C source
literally calls `memcpy`, so `copy_data_block` — and the `array2`/`array1` copy
and the `strncpy` in `overunder` — now call libc `memcpy`/`strncpy` through
`extern "C"`. That is the more literal translation and makes the behaviour
identical to C in **both** build profiles.
