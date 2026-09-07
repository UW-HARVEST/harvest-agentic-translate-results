# ERRORS.md — Phase C error / rejection surface table

Mechanically derived by grepping **all** of `c_src/src/lib.c` for every way the
code rejects, sentinels, or degenerates on input. Note the shape of this library:

```
$ grep -nE 'RETURN_ERROR|return -1|return NULL|assert|errno|goto ' c_src/src/lib.c
(no matches)
```

There are **no** `assert`s, no `errno`, no `RETURN_ERROR` macro, no error enum, and
no `NULL` returns. Every function returns `int`/`time_t` unconditionally. The
"rejection" surface therefore consists of the **fall-through / sentinel /
out-of-range branches** that the C code actually takes, plus the generic FFI
boundaries (null pointers, out-of-range "enum-like" ints, overflow). Each row
below is one distinct such branch, taken verbatim from the source.

| # | function | trigger (exact invalid input / condition) | expected C result | test | [x] |
|---|----------|-------------------------------------------|-------------------|------|-----|
| 1 | `classify_mode` | `mode` matches none of the 4 literals (final `return 0x00;`, lib.c:39) — e.g. `"STANDARD"`, `"standar"`, `"standardx"`, `"turbo\0junk"` | `0` | `err_01_classify_mode_unknown` | [x] |
| 2 | `classify_mode` | `mode` is the empty string `""` (degenerate zero-length input, still hits `return 0x00`) | `0` | `err_02_classify_mode_empty` | [x] |
| 3 | `classify_mode` | `mode` is a prefix/superstring of a valid literal — `strcmp` is exact, no prefix match | `0` | `err_03_classify_mode_prefix_superstring` | [x] |
| 4 | `classify_mode` | `mode` contains embedded high bytes / non-ASCII (`strcmp` compares as `unsigned char`, sign-extension bug class) | `0` | `err_04_classify_mode_high_bytes` | [x] |
| 5 | `classify_mode` | `mode == NULL` — C `strcmp(NULL, …)` dereferences null → SIGSEGV. Both libs must crash the *same* way, so this is asserted in a **forked child** by comparing the wait-status of C vs Rust. | fatal signal (SIGSEGV), same for both | `err_05_classify_mode_null_ptr_both_crash` | [x] |
| 6 | `apply_multiplier` | `level` outside `0..=4` → `default: result = 0xDEAD;` (lib.c:57-58). This is the C "invalid enum value" path: `level` is an `int`, so every value with no `case` is a real input. Tested for `-1`, `5`, `6`, `INT_MIN`, `INT_MAX`, and randoms. | `0xDEAD` (`57005`), **independent of `base`** | `err_06_apply_multiplier_out_of_range_level` | [x] |
| 7 | `apply_multiplier` | `level` one step past each end of the valid range: `-1` and `5` | `0xDEAD` | `err_07_apply_multiplier_one_past_range` | [x] |
| 8 | `apply_multiplier` | `base` near `INT_MAX` with `level` in `0..=4` → signed-integer overflow in `result +=` (UB in C; wraps on the ABI). `base = INT_MAX, level = 4` | wrapped 2's-complement sum | `err_08_apply_multiplier_signed_overflow` | [x] |
| 9 | `convert_time_factor` | `factor * 1e12` not representable in `int` → `(int)` conversion out of range (UB; x86-64 `cvttsd2si` yields the *integer indefinite* `0x80000000`). Any `|factor| >= ~2.15e-3` overflows. | `-2147483648` | `err_09_convert_time_factor_overflow` | [x] |
| 10 | `convert_time_factor` | `factor` = `NaN` | `-2147483648` | `err_10_convert_time_factor_nan` | [x] |
| 11 | `convert_time_factor` | `factor` = `+inf` / `-inf` | `-2147483648` | `err_11_convert_time_factor_inf` | [x] |
| 12 | `convert_time_factor` | `factor` denormal / `±0.0` / `-0.0` (truncates to zero, not an error but the boundary of the range check) | `0` | `err_12_convert_time_factor_zero_denormal` | [x] |
| 13 | `convert_time_factor` | `factor * 1e12` lands exactly on `INT_MIN` / `INT_MAX` and one ULP past them (`±2147483647.x`, `±2147483648.0`, `±2147483649.0`) | in-range → truncated value; past range → `-2147483648` | `err_13_convert_time_factor_int_boundaries` | [x] |
| 14 | `convert_negative_overflow` | `value * -1e15` out of `int` range (UB `(int)` cast) — note the **negating** scale factor, so positive inputs underflow | `-2147483648` | `err_14_convert_negative_overflow_underflow` | [x] |
| 15 | `convert_negative_overflow` | `value` = `NaN` / `±inf` | `-2147483648` | `err_15_convert_negative_overflow_nan_inf` | [x] |
| 16 | `convert_negative_overflow` | `value` = `±0.0` → `-0.0 * -1e15 = +0.0`, `+0.0 * -1e15 = -0.0`; both truncate to `0` (sign-of-zero boundary) | `0` | `err_16_convert_negative_overflow_signed_zero` | [x] |
| 17 | `get_modified_time` | `offset_days * 86400` overflows `int` (the product is computed in `int` *before* widening to `time_t`) — e.g. `offset_days = INT_MAX`, `24856`, `-24856` | wrapped `int` product, then sign-extended | `err_17_get_modified_time_int_overflow` | [x] |
| 18 | `get_modified_time` | `offset_hours * 3600` overflows `int` — e.g. `offset_hours = INT_MAX` / `INT_MIN` | wrapped `int` product | `err_18_get_modified_time_hours_overflow` | [x] |
| 19 | `get_modified_time` | the *sum* `(days*86400) + (hours*3600)` overflows `int` while each product alone does not | wrapped `int` sum | `err_19_get_modified_time_sum_overflow` | [x] |
| 20 | `hash_time_value` | `t` negative / `INT64_MIN` / `-1` — the byte loop reads the raw object representation, and `hash *= 0x1F` overflows `int` (UB) every iteration | `hash & 0x7FFFFFFF` over wrapped arithmetic | `err_20_hash_time_value_negative_and_overflow` | [x] |
| 21 | `hash_time_value` | `t = 0` (all-zero object representation; only the `0x5A5A5A5A` seed survives) | fixed value, must match | `err_21_hash_time_value_zero` | [x] |
| 22 | `modeselect` | `mode_selector < 0` → `mode_selector % 4` is **negative** in C, so `modes[mode_index]` reads *before* the array → out-of-bounds stack read (**UB**). The C is not self-consistent here: linked into a small driver it faults with `SIGSEGV` for `-1/-2/-3`, but called from inside the test process it survives and returns a garbage-derived value. No equality can therefore be required; the test drives both libraries in an isolated child, records both outcomes, and asserts each is either a clean return or a memory fault. `INT_MIN` is special-cased: `INT_MIN % 4 == 0`, so it is *in bounds* and its full result **must** match. | UB / unspecified pointer (measured, not asserted equal) | `err_22_modeselect_negative_selector_documented` | [x] |
| 23 | `modeselect` | `complexity % 5` is negative (`complexity < 0`) → `apply_multiplier` hits `default` → multiplier `0xDEAD`. A valid-looking input that reaches the error branch. | multiplier `0xDEAD` folded into result | `err_23_modeselect_negative_complexity` | [x] |
| 24 | `modeselect` | `INT_MIN` for each of the four parameters (`INT_MIN % 4 == 0`, `INT_MIN % 5 == -3`, `INT_MIN % 24 == -8`) plus the overflow in `result * 0x10 + 0xBEEF` | wrapped result, must match | `err_24_modeselect_int_min_params` | [x] |
| 25 | `modeselect` | `INT_MAX` for each of the four parameters (`INT_MAX % 4 == 3`, `% 5 == 2`, `% 24 == 7`) | wrapped result, must match | `err_25_modeselect_int_max_params` | [x] |

## Minimum / maximum constants grepped from the C source

`0x10 0x20 0x30 0x40 0x00` (mode values), `0xFF 0xAB 0x7E 0x1C 0x05` (multiplier
steps), `0xDEAD` (invalid-level sentinel), `0xA0` (fixed base in `modeselect`),
`1e12`, `-1e15`, `1e8`, `-1e7` (double scales), `86400`, `3600`, `29` (shift),
`0x5A5A5A5A` (hash seed), `0x1F` (hash multiplier), `0x7FFFFFFF` (hash mask),
`0x1000` (hash modulus), `0xFF` / `0xFF00` (result xor masks), `0x10` / `0xBEEF`
(final mix), moduli `4`, `5`, `24`, `sizeof(time_t) == 8`. Every one of these is
covered by a row above or by a `CONFIGS.md` row.
