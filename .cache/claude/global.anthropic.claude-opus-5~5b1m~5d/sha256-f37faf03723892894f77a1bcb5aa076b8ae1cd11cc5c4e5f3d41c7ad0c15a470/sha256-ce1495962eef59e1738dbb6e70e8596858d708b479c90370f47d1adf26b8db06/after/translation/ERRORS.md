# ERRORS.md — Error / rejection surface table

Mechanically derived from every rejection-shaped construct in `c_src/src/lib.c`.
This library has **no error enum, no `errno`, no `RETURN_ERROR` macro, no
`assert`, and no NULL checks**. Its entire rejection surface consists of:

* the three guard branches in `safe_double_to_int` (`> INT_MAX`, `< INT_MIN`,
  `isnan`) — clamping/sentinel returns, `lib.c:40-45`;
* the `default:` arm of `process_with_fallthrough`, whose `result = -1` is the
  only `-1` sentinel in the file (`lib.c:69-71`);
* the fixed `sizeof(label) - 1` bound + forced NUL in `overunder` (`lib.c:121-122`);
* unchecked pointer dereferences in `copy_data_block` (`lib.c:78`) — no
  validation at all, so an invalid pointer is a hard fault in both languages.

Column "expected C result" is the value the C `.so` actually produces (confirmed
by the differential tests, not by documentation).

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| 1  | `safe_double_to_int` | `d > (double)INT_MAX`, i.e. `d > 2147483647.0` (e.g. `2147483648.0`, `1e15`) | returns `INT_MAX` = `2147483647` |
| 2  | `safe_double_to_int` | `d == nextafter(2147483647.0, INF)` — one ULP past the valid range | returns `INT_MAX` |
| 3  | `safe_double_to_int` | `d == +INFINITY` | returns `INT_MAX` (first branch taken) |
| 4  | `safe_double_to_int` | `d < (double)INT_MIN`, i.e. `d < -2147483648.0` (e.g. `-2147483649.0`, `-1e15`) | returns `INT_MIN` = `-2147483648` |
| 5  | `safe_double_to_int` | `d == nextafter(-2147483648.0, -INF)` — one ULP past the valid range | returns `INT_MIN` |
| 6  | `safe_double_to_int` | `d == -INFINITY` | returns `INT_MIN` (second branch taken) |
| 7  | `safe_double_to_int` | `d` is a quiet NaN (both range comparisons are false, so `isnan` arm is reached) | returns `0` |
| 8  | `safe_double_to_int` | `d` is a *signalling* / negative-sign NaN (`-NAN`, custom payload bit pattern) | returns `0` (same arm) |
| 9  | `safe_double_to_int` | boundary *inside* the range: `d == 2147483647.0` exactly (`>` is false) | returns `2147483647` via the `(int)d` cast, **not** the clamp |
| 10 | `safe_double_to_int` | boundary *inside* the range: `d == -2147483648.0` exactly (`<` is false) | returns `-2147483648` via the `(int)d` cast, **not** the clamp |
| 11 | `process_with_fallthrough` | `code` outside `{0,1,2,3,4,5}` — negative, e.g. `-1`, `-5`, `INT_MIN` | `default:` arm → returns `-1` |
| 12 | `process_with_fallthrough` | `code` outside `{0,1,2,3,4,5}` — one past the range, `code == 6` | `default:` arm → returns `-1` |
| 13 | `process_with_fallthrough` | `code == INT_MAX` (far out-of-range "enum" value crossing FFI) | `default:` arm → returns `-1` |
| 14 | `process_with_fallthrough` | `code == 0` (sentinel-looking value that is *valid*) | returns `0`, **ignoring** `base_value` entirely |
| 15 | `process_with_fallthrough` | signed overflow: `code == 5`, `base_value == INT_MAX` (adds 150) | wraps two's-complement → `INT_MAX + 150` wrapped = `-2147483499` |
| 16 | `process_with_fallthrough` | signed underflow: `code == 5`, `base_value == INT_MIN` | no wrap needed → `-2147483498` |
| 17 | `copy_data_block` | `dest == NULL`, `src` valid — no null check, `memcpy` writes to `NULL` | process dies with `SIGSEGV` (11) |
| 18 | `copy_data_block` | `src == NULL`, `dest` valid — `memcpy` reads from `NULL` | process dies with `SIGSEGV` (11) |
| 19 | `copy_data_block` | both `dest == NULL` and `src == NULL` | process dies with `SIGSEGV` (11) |
| 20 | `copy_data_block` | `src` points at a buffer **smaller than 40 bytes** (undersized/oversized-length class): reads all `sizeof(DataBlock)` bytes regardless | copies 40 bytes incl. the 4 padding bytes after `id` and the 4 trailing pad bytes; no truncation, no rejection |
| 21 | `handle_pointer_operations` | signed overflow in `value * 2`: `value == INT_MAX` | wraps → `2 * INT_MAX` wrapped is `-2`, `+ 100` = **`98`** (not a saturated value) |
| 22 | `handle_pointer_operations` | signed overflow in `value * 2`: `value == INT_MIN` | wraps → `0 + 100` = `100` |
| 23 | `overunder` | `a < 0` ⇒ C's `%` truncates toward zero so `a % 6 < 0` ⇒ `process_with_fallthrough` takes `default:` | `switch_result == -1` folded into `total` |
| 24 | `overunder` | `a == INT_MIN` ⇒ `a % 6 == -2` (negative, `default:` arm) **and** `a * a` overflows | `switch_result == -1`; see row 25 for `conv4` |
| 25 | `overunder` | `d*d + a*a` overflows to a **negative** int ⇒ `sqrt(negative)` = NaN ⇒ `safe_double_to_int(NaN)` | `conv4 == 0` (row 7 path), no trap, no `errno` check |
| 26 | `overunder` | `a` large enough that `a * 1.5 > INT_MAX` (e.g. `a == INT_MAX`) | `conv1` clamped to `INT_MAX` (row 1 path) |
| 27 | `overunder` | `b` large enough that `b * 2.7 > INT_MAX` / `< INT_MIN` | `conv2` clamped to `INT_MAX` / `INT_MIN` |
| 28 | `overunder` | total accumulation overflows `int` (all four args near `INT_MAX`) | wraps two's-complement; no saturation, no rejection |
| 29 | `overunder` | fixed hard-coded clamps `safe_double_to_int(1e15)` / `(-1e15)` executed on **every** call | always prints `2147483647` then `-2147483648` (rows 1 & 4) |
| 30 | `overunder` | `strncpy(label, "Source", sizeof(label)-1)` + `label[19]='\0'`: source shorter than the bound | `label` = `"Source"` then **13 zero-pad bytes**, `label[19]` forced to `0`; `%s` prints `Source` |

## Notes on rows 17-19

There is no way for the C code to "return an error" here — the C dereferences
unconditionally. The differential test therefore forks a child process for each
of C and Rust, performs the call, and asserts that **both children die with the
identical signal**. That is the observable behaviour these rows specify.

## Status

| row | test | status |
|-----|------|--------|
| 1-10  | `tests/phase_c_errors.rs::err_safe_double_to_int_*` | [x] pass |
| 11-16 | `tests/phase_c_errors.rs::err_fallthrough_*`        | [x] pass |
| 17-19 | `tests/phase_c_errors.rs::err_copy_data_block_null_*` | [x] pass |
| 20    | `tests/phase_c_errors.rs::err_copy_data_block_reads_full_struct_incl_padding` | [x] pass |
| 21-22 | `tests/phase_c_errors.rs::err_handle_pointer_operations_overflow` | [x] pass |
| 23-29 | `tests/phase_c_errors.rs::err_overunder_*`          | [x] pass |
| 30    | `tests/phase_b_valid.rs::cfg_overunder_label_is_source_padded` | [x] pass |

All 30 rows have a passing differential test that asserts C and Rust return the
**same** sentinel / clamp / signal, and that the shared value still equals the
one documented above (so a future change to either side is caught). Rows 17-19
compare the child-process termination signal: both libraries die with
`SIGSEGV` (11).

Re-verified against the C rebuilt at `-O0`/`-O1`/`-O2`/`-O3`/`-Os` and both
Rust profiles. See the "Divergence found and fixed" note in `CONFIGS.md` for
the `memcpy` issue that rows 17-19 and CONFIGS row 19 uncovered in debug builds.
