# ERRORS.md — Error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return`, every
`if (... != NULL)` / `if (... != 0)` / `if (... >= 0)` guard, every `%` with a
constant modulus (INT_MIN interaction), every narrowing cast, and every
special-value constant (`INFINITY`, `NAN`, `sizeof`-derived bounds).

There are **no** `assert`s, **no** error enums, and **no** error-return macros in
this library. Rejection is expressed in exactly three ways:

1. the sentinel `return -1` in `find_value_in_buffer`,
2. guard branches that *skip* work (`if (b != 0)`, `if (result != NULL)`,
   `if (pos >= 0)`, `if (direct_search != NULL)`, `for (i = 0; i < size; i++)`),
3. implementation-defined / undefined conversions that the reference platform
   (x86-64 SysV, signed `char`, SSE2 `cvttsd2si`) resolves to a specific value.

All rows are covered by `tests/phase_c_errors.rs`, which calls **both** the C
`.so` and the Rust `.so` through `libloading` and asserts the *same* sentinel /
value, never merely "both failed".

| #  | function | trigger (exact invalid input / condition) | expected C result | test |
|----|----------|-------------------------------------------|-------------------|------|
| 1  | `find_value_in_buffer` | searched byte absent from `buffer[0..size]`, so `memchr` returns `NULL` (`lib.c:36` false) | returns `-1` (`lib.c:39`) | `err01_fvib_absent_returns_minus_one` |
| 2  | `find_value_in_buffer` | `size == 0` — `memchr` inspects nothing, returns `NULL` | returns `-1` | `err02_fvib_zero_size_returns_minus_one` |
| 3  | `find_value_in_buffer` | `buffer == NULL` together with `size == 0` (glibc `memchr` never dereferences) | returns `-1` | `err03_fvib_null_buffer_zero_size` |
| 4  | `find_value_in_buffer` | `search_val` outside `0..=255`: `(char)search_val` truncates, then `memchr`'s `int`→`unsigned char` conversion re-narrows (`lib.c:34-35`) | matches on byte `search_val & 0xFF`; `-1` only if that byte is absent | `err04_fvib_search_val_out_of_byte_range` |
| 5  | `find_value_in_buffer` | match at index `0` — result pointer equals `buffer`, difference is `0`, which must **not** be confused with the `-1` sentinel or with `NULL` | returns `0` | `err05_fvib_match_at_index_zero_is_not_error` |
| 6  | `find_value_in_buffer` | `search_val` selects the NUL byte (`0`) — `memchr` is length-based, so NUL is a normal searchable value, not a terminator | returns the index of byte `0x00` | `err06_fvib_nul_byte_is_searchable` |
| 7  | `find_value_in_buffer` | `search_val = INT_MIN` / `INT_MAX` (extreme ints through the `(char)` cast) | byte `0x00` / `0xFF` respectively | `err07_fvib_extreme_search_val` |
| 8  | `calculate_with_doubles` | `b == 0` → division-by-zero guard at `lib.c:57` skips the divide, `result` stays `0.0` | returns `0.0 * pow(10, c%10)` = `+0.0` | `err08_cwd_zero_divisor_guard` |
| 9  | `calculate_with_doubles` | `c == INT_MIN` → `c % 10` on the most negative int (well-defined for modulus 10, unlike `% -1`) | exponent `-8`, returns `(a/b) * 1e-8` | `err09_cwd_int_min_exponent` |
| 10 | `calculate_with_doubles` | `a == INT_MIN, b == -1` — the pair that overflows in *integer* division; here both are widened to `double` first, so no trap | returns `2147483648.0 * pow(10, c%10)` | `err10_cwd_int_min_over_minus_one` |
| 11 | `calculate_with_doubles` | negative `c` → C `%` yields a **negative** remainder (unlike a Euclidean modulus), so `pow` gets a negative exponent | fractional scaling, e.g. `c=-3` → `pow(10,-3)` | `err11_cwd_negative_exponent_sign` |
| 12 | `convert_double_to_int` | `value > INT_MAX` (out of range for `int`) — `lib.c:30` `(int)value` | `cvttsd2si` "integer indefinite" `0x80000000` = `-2147483648` | `err12_cdti_above_int_max` |
| 13 | `convert_double_to_int` | `value < INT_MIN` (out of range) | `-2147483648` | `err13_cdti_below_int_min` |
| 14 | `convert_double_to_int` | `value` is `NaN` (incl. negative-sign and non-default payload bit patterns) | `-2147483648` | `err14_cdti_nan` |
| 15 | `convert_double_to_int` | `value == +INFINITY` (`lib.c:136,141`) | `-2147483648` | `err15_cdti_pos_infinity` |
| 16 | `convert_double_to_int` | `value == -INFINITY` | `-2147483648` | `err16_cdti_neg_infinity` |
| 17 | `convert_double_to_int` | one step past each end of the valid range: `2147483648.0`, `-2147483649.0`, and the adjacent representables | out-of-range → `-2147483648`; in-range → exact truncation | `err17_cdti_one_step_past_range` |
| 18 | `create_numeric_buffer` | `size == 0` → `for (i = 0; i < size; i++)` body never runs | buffer left completely untouched | `err18_cnb_zero_size_writes_nothing` |
| 19 | `create_numeric_buffer` | `size < 0` (negative length, incl. `INT_MIN`) → loop condition false immediately | buffer left completely untouched (no wrap into a huge count) | `err19_cnb_negative_size_writes_nothing` |
| 20 | `create_numeric_buffer` | `buffer == NULL` with `size <= 0` — pointer never dereferenced | no write, no crash | `err20_cnb_null_buffer_nonpositive_size` |
| 21 | `create_numeric_buffer` | `seed` near `INT_MAX` so `seed + i*7` overflows signed `int` | wraps; stored byte is the low 8 bits of the wrapped sum | `err21_cnb_seed_overflow_wraps` |
| 22 | `create_numeric_buffer` | `seed` negative so `(seed + i*7) % 256` is **negative**, then cast to signed `char` | stores the negative `char`, i.e. low 8 bits reinterpreted as `i8` | `err22_cnb_negative_seed_signed_char` |
| 23 | `process_negation` | `var1 == 0` — the only "falsy" input for `!!` | returns `0` | `err23_pn_zero` |
| 24 | `process_negation` | `var1 != 0` including `INT_MIN`, `INT_MAX`, `-1`, and values whose low bits are `0` (e.g. `256`) — `!!` must not degrade to `& 1` | returns `1` | `err24_pn_nonzero_incl_extremes` |
| 25 | `doubleneg` | search value not found → `if (pos >= 0)` false, "not found" branch, `pos` **not** accumulated (`lib.c:112-117`) | *unreachable*: `buffer[i] = (seed + 7i) % 256` with `gcd(7,256)=1` covers all 256 byte values, so every search hits. Asserted unreachable by checking the "not found" string never appears in either implementation's stdout. | `doubleneg_error_paths` (label `row25/26`) |
| 26 | `doubleneg` | byte `100` absent → `if (direct_search != NULL)` false, print + accumulate skipped (`lib.c:120-125`) | *unreachable*, same permutation argument; the "Direct memchr found" line must always be present in both | `doubleneg_error_paths` (label `row25/26`) |
| 27 | `doubleneg` | `param2/3/4 == INT_MIN` → `param % 256` yields a negative search value fed to `find_value_in_buffer` | negative search values still resolve via `& 0xFF`; identical accumulated result | `doubleneg_error_paths` (label `row27`) |
| 28 | `doubleneg` | `converted_int % 1000` / `converted_neg % 1000` where the operand is `INT_MIN` (`lib.c:100`) — `converted_neg` is *always* `INT_MIN` because `-1.0*pow(2,40)` is out of `int` range | `INT_MIN % 1000 == -648`, subtracted from `result` | `doubleneg_error_paths` (label `row28`) |
| 29 | `doubleneg` | `(param1 + i * param2) % 256` overflows signed `int` for large params (`lib.c:129`) | wraps (GCC `-O0`); identical byte searched | `doubleneg_error_paths` (label `row29`) |
| 30 | *(FFI generic)* | out-of-range "enum-like" `int` passed across the FFI boundary — this library declares no `enum`, so the analogous case is any `int` parameter with no distinguished valid subrange (`search_val`, `size`, `seed`, `param1..4`): every one of the 2^32 values is accepted by C | never rejected; must agree for the full extreme set | `err30_no_enum_all_ints_accepted` |

## Generic-boundary coverage (required even though not in the table)

| condition | covered by |
|-----------|-----------|
| null pointer arguments | rows 3, 20 |
| zero length | rows 2, 18 |
| negative / "oversized" length | row 19 (`INT_MIN`, `-1`) |
| one step past a valid range | rows 17, 7 |
| out-of-range enum value over FFI | row 30 (no enums exist; all-`int` domain exercised instead) |

Not tested, because the C itself is out-of-bounds there and the "expected C
result" would be a segfault rather than a value: `find_value_in_buffer` with
`size` larger than the real allocation, `create_numeric_buffer` with
`size` larger than the real allocation, and `buffer == NULL` with `size > 0`.
Both implementations forward to the same glibc `memchr` / perform the same
stores, so there is no behavioural difference to verify, only a shared crash.
