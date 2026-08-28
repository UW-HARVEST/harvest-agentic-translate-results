# Phase A.2 — Error-surface table

Derived mechanically from `c_src/src/lib.c`. The library has **no** error enum,
no `RETURN_ERROR` macro, no `assert`, and no `errno` use; every rejection is
either a sentinel return, a guarded branch that silently skips work, or a
hardware-level out-of-range conversion. Every distinct rejection/branch in the C
source gets one row.

Grep inventory that the rows are derived from:

* `return -1;` — 1 occurrence (`lib.c:39`, `find_value_in_buffer`).
* `if (result != NULL)` — `lib.c:36` (`find_value_in_buffer`).
* `if (b != 0)` — `lib.c:57` (`calculate_with_doubles`).
* `for (int i = 0; i < size; i++)` — `lib.c:49` (`create_numeric_buffer`, the
  non-positive-`size` guard).
* `if (pos >= 0) … else` — `lib.c:112`/`lib.c:115` (`doubleneg`).
* `if (direct_search != NULL)` — `lib.c:121` (`doubleneg`, no `else`).
* `(int)value` — `lib.c:30` (`convert_double_to_int`): out-of-range / NaN is UB
  in C; at `-O0` GCC emits `cvttsd2si`, whose documented answer for an
  unrepresentable source is the *integer indefinite* value `0x80000000` =
  `INT_MIN` = `-2147483648`.
* No `assert`, no `NULL` parameter validation anywhere: passing a null buffer
  with a positive length is a hard fault in C and must be a hard fault in Rust
  too (not a graceful `-1`), so the "null + zero length" boundary is the only
  null case that is *defined* and therefore testable.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|----|----------|----------------------------------------------|-------------------|------|----|
| 1  | `find_value_in_buffer` | needle byte absent from the whole buffer (`memchr` → `NULL`) | returns `-1` | `err_01_find_absent_needle` | [x] |
| 2  | `find_value_in_buffer` | `size == 0` (empty range ⇒ `memchr` → `NULL`) even when the byte exists just past the end | returns `-1` | `err_02_find_zero_size` | [x] |
| 3  | `find_value_in_buffer` | `buffer == NULL` **and** `size == 0` (no dereference) | returns `-1`, no fault | `err_03_find_null_zero_size` | [x] |
| 4  | `find_value_in_buffer` | `search_val` outside `0..=255`, e.g. `-1`, `256`, `300`, `INT_MIN`, `INT_MAX` — narrowed by `(char)` then re-widened as `unsigned char`, so only the low byte is honoured | returns the index of the **low byte** of `search_val`, or `-1`; never rejects | `err_04_find_out_of_range_needle` | [x] |
| 5  | `find_value_in_buffer` | `size` larger than the number of matching bytes but needle present at index `size-1` (last-byte boundary) | returns `size-1` | `err_05_find_last_byte_boundary` | [x] |
| 6  | `find_value_in_buffer` | `size` one *past* a match (`size` = match index) — match must **not** be seen | returns `-1` | `err_06_find_one_past_match` | [x] |
| 7  | `create_numeric_buffer` | `size == 0` | loop body never runs; buffer untouched | `err_07_create_zero_size` | [x] |
| 8  | `create_numeric_buffer` | `size < 0` (`-1`, `-256`, `INT_MIN`) | loop condition `0 < size` false ⇒ buffer untouched, no fault | `err_08_create_negative_size` | [x] |
| 9  | `create_numeric_buffer` | `buffer == NULL` with `size <= 0` | no dereference, returns cleanly | `err_09_create_null_nonpositive_size` | [x] |
| 10 | `create_numeric_buffer` | negative `seed` ⇒ `(seed + i*7) % 256` is a **negative** remainder (C `%` truncates toward zero) which `(char)` then reinterprets | writes negative `char` values, e.g. seed `-1` ⇒ byte `-1`/`0xFF` | `err_10_create_negative_seed` | [x] |
| 11 | `create_numeric_buffer` | `seed` near `INT_MAX` ⇒ `seed + i*7` signed overflow (UB, wraps at `-O0`) | wrapped `int` arithmetic result | `err_11_create_seed_overflow` | [x] |
| 12 | `calculate_with_doubles` | `b == 0` (division guard) | `result` stays `0.0`, then `0.0 * pow(10, c%10)` ⇒ `+0.0`; **never** `inf`/`nan` | `err_12_calc_zero_divisor` | [x] |
| 13 | `calculate_with_doubles` | `c` negative ⇒ `c % 10` is a negative exponent | multiplies by `pow(10, -k)`, e.g. `1e-9` | `err_13_calc_negative_exponent` | [x] |
| 14 | `calculate_with_doubles` | `c == INT_MIN` (`INT_MIN % 10 == -8`, must not trap/panic) | multiplies by `pow(10,-8)` | `err_14_calc_c_int_min` | [x] |
| 15 | `calculate_with_doubles` | `a == INT_MIN, b == -1` (would overflow in *integer* division, but the C converts to `double` first) | `2147483648.0 * pow(10, c%10)`, no trap | `err_15_calc_int_min_over_minus_one` | [x] |
| 16 | `convert_double_to_int` | `value > INT_MAX` after truncation (`2147483648.0`, `1e300`, `f64::MAX`) | `cvttsd2si` indefinite ⇒ `INT_MIN` (`-2147483648`) | `err_16_cvt_above_int_max` | [x] |
| 17 | `convert_double_to_int` | `value < INT_MIN` after truncation (`-2147483649.0`, `-2^40`, `-1e300`, `f64::MIN`) | `INT_MIN` | `err_17_cvt_below_int_min` | [x] |
| 18 | `convert_double_to_int` | `value == +INFINITY` | `INT_MIN` | `err_18_cvt_pos_infinity` | [x] |
| 19 | `convert_double_to_int` | `value == -INFINITY` | `INT_MIN` | `err_19_cvt_neg_infinity` | [x] |
| 20 | `convert_double_to_int` | `value` is NaN — quiet, negative-quiet, signalling, and arbitrary NaN payloads | `INT_MIN` | `err_20_cvt_nan_payloads` | [x] |
| 21 | `convert_double_to_int` | exact representability boundaries: `2147483647.0`, `2147483647.5`, `2147483648.0`, `-2147483648.0`, `-2147483648.5`, `-2147483649.0` (one step past the valid range in both directions) | in-range values convert exactly; the first out-of-range value flips to `INT_MIN` | `err_21_cvt_boundaries` | [x] |
| 22 | `convert_double_to_int` | `-0.0` and negative subnormals (truncate to `-0.0`, must not become `-1`) | `0` | `err_22_cvt_negative_zero_and_subnormal` | [x] |
| 23 | `doubleneg` | `pos < 0` path — the `else` branch that prints `"Value %d not found"`. **Provably unreachable**: `create_numeric_buffer(buffer, 256, param1)` writes `(char)((param1 + 7i) % 256)` for `i ∈ 0..256`; `gcd(7,256) = 1`, so the 256 bytes are a *permutation of all 256 byte values* for **every** `param1` (including wrap-around ones, since `2^32 ≡ 0 mod 256`). Hence `memchr` always hits. Verified empirically for 15 extreme seeds. | the `else` line never appears; the reachable rejection of the same underlying `memchr` is row 1 | `err_23_doubleneg_value_not_found_is_unreachable` (asserts the line is absent, and that a `"Found value …"` line appears for all 4 searches, in **both** libraries — a Rust generator bug would make Rust disagree) | [x] |
| 24 | `doubleneg` | `direct_search == NULL` path — the `"Direct memchr found byte 100 at offset:"` line is skipped. **Provably unreachable** for the same reason as row 23 (byte `100` is always present). | the line is always present, with an identical offset in both libraries | `err_24_doubleneg_direct_search_never_null` | [x] |
| 25 | `doubleneg` | `converted_int == INT_MIN` ⇒ `INT_MIN % 1000 == -648` (must not panic in Rust, must not be `+648`) | `result` decreases by `648` | `err_25_doubleneg_int_min_modulo` | [x] |
| 26 | `doubleneg` | `param2/3/4 == INT_MIN` ⇒ `param % 256` on `INT_MIN` (no overflow panic) | `INT_MIN % 256 == 0` | `err_26_doubleneg_params_int_min` | [x] |
| 27 | `doubleneg` | signed overflow computing `param1 + i*param2` in the combined-feature loop (UB in C, wraps at `-O0`; Rust must wrap, not panic). Note the `result` accumulator itself is provably bounded (`|result| < 10 + 3 + 2·999 + 4·255 + 255 + 10`), so it can never overflow. | wrapped `int` feeds `% 256`, then the low byte feeds `memchr` | `err_27_doubleneg_stride_overflow` | [x] |
| 28 | *all* | out-of-range **enum** values across the FFI boundary | **N/A — the public API declares no `enum` type**; the equivalent "any `int` is accepted" surface is covered by rows 4, 8, 10, 11, 14, 26, 27 (full-range `int` inputs incl. `INT_MIN`/`INT_MAX`) | `err_28_no_enum_full_int_range_sweep` | [x] |

All 28 rows have a passing differential test in `translation/tests/errors.rs`.
