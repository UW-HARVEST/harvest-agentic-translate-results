# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. The library uses **no** error
enums, **no** `RETURN_ERROR` macro, **no** `assert`, **no** `return NULL`, and
**no** `errno`. Greps performed:

```sh
grep -nE 'assert|RETURN_ERROR|return *-1|return *NULL|errno|abort|exit\(' c_src/src/lib.c   # -> no matches
grep -nE 'return|default:|else$|if *\(' c_src/src/lib.c
```

Every way the C code can reject / degrade on input is therefore one of:
a **sentinel return value** (`0x00`, `0xDEAD`), an **implementation-defined
saturation** (`cvttsd2si` "integer indefinite" = `INT_MIN`), a **wrap-around**
on signed overflow, or a **fatal out-of-bounds read** (`SIGSEGV`). All of them
are enumerated below, one row per distinct rejection branch.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `classify_mode` | `mode` matches none of the four literals (fall-through past all four `strcmp`s), e.g. `"STANDARD"`, `""`, `"standar"`, `"standardx"`, `"turbo\0x"`-suffix, arbitrary bytes | returns sentinel `0x00` |
| 2  | `classify_mode` | `mode == NULL` — dereferenced by `strcmp` with no null check (`lib.c:30`) | fatal `SIGSEGV` inside `strcmp` |
| 3  | `apply_multiplier` | `level` outside `0..=4` → `switch` `default:` (`lib.c:57-58`), e.g. `5`, `-1`, `INT_MIN`, `INT_MAX` | returns sentinel `0xDEAD` (53421), **base is discarded** |
| 4  | `apply_multiplier` | `level` in `0..=4` but `base` near `INT_MAX` so the fall-through additions overflow signed `int` (`lib.c:47-55`) | wraps modulo 2^32 (GCC/x86-64) |
| 5  | `convert_time_factor` | `factor * 1e12 >= 2^31` (out of `int` range), e.g. `factor = 1.0` | `(int)` via `cvttsd2si` → `INT_MIN` (`-2147483648`) |
| 6  | `convert_time_factor` | `factor * 1e12 < -2^31` (out of `int` range), e.g. `factor = -1.0` | `cvttsd2si` → `INT_MIN` |
| 7  | `convert_time_factor` | `factor` is `NaN` (product is `NaN`) | `cvttsd2si` → `INT_MIN` |
| 8  | `convert_time_factor` | `factor` is `+inf` / `-inf` (product is `±inf`) | `cvttsd2si` → `INT_MIN` |
| 9  | `convert_time_factor` | `factor = 0.0` / `-0.0` / subnormal → product underflows to (signed) zero | returns `0` (no rejection; boundary of rows 5–8) |
| 10 | `convert_negative_overflow` | `value * -1e15 >= 2^31`, e.g. `value = -1.0` | `cvttsd2si` → `INT_MIN` |
| 11 | `convert_negative_overflow` | `value * -1e15 < -2^31`, e.g. `value = 1.0` | `cvttsd2si` → `INT_MIN` |
| 12 | `convert_negative_overflow` | `value` is `NaN` | `cvttsd2si` → `INT_MIN` |
| 13 | `convert_negative_overflow` | `value` is `±inf` | `cvttsd2si` → `INT_MIN` |
| 14 | `get_modified_time` | `offset_days * 86400` overflows `int` (`lib.c:81`), e.g. `offset_days = 100000` | `int` product wraps, then sign-extends to `time_t` |
| 15 | `get_modified_time` | `offset_hours * 3600` overflows `int`, e.g. `offset_hours = INT_MAX` | `int` product wraps, then sign-extends |
| 16 | `get_modified_time` | the `int` sum of the two products overflows `int` | sum wraps in `int`, then sign-extends (**not** 64-bit arithmetic) |
| 17 | `hash_time_value` | `bytes[i] << 24` with `bytes[i] >= 0x80` overflows signed `int` (`lib.c:90`) | shift wraps; result still masked with `0x7FFFFFFF` (always `>= 0`) |
| 18 | `hash_time_value` | `hash *= 0x1F` overflows signed `int` (`lib.c:91`) | wraps modulo 2^32; final `& 0x7FFFFFFF` |
| 19 | `modeselect` | `mode_selector % 4 < 0` (i.e. `mode_selector < 0` and not a multiple of 4) → `modes[mode_index]` reads **before** the 4-element array (`lib.c:101-102`) | out-of-bounds stack read yields a non-pointer; fatal `SIGSEGV` in `classify_mode`/`strcmp` |
| 20 | `modeselect` | `complexity % 5 < 0` (negative `complexity`) → `apply_multiplier` `default:` | `multiplier == 0xDEAD` (row 3 reached through the pipeline) |
| 21 | `modeselect` | `seed != 0` ⇒ `factor1 = seed*1e8`, `factor1*1e12` always out of `int` range | `result1 == INT_MIN` |
| 22 | `modeselect` | `time_offset != 0` ⇒ `factor2 = time_offset*-1e7`, `factor2*-1e15` always out of range | `result2 == INT_MIN` |
| 23 | `modeselect` | final `result * 0x10 + 0xBEEF` overflows signed `int` (`lib.c:135`) | would wrap modulo 2^32 — but **unreachable**: `result <= 0x40 + 0xDEAD + 0xFFF`, and the two `^=` masks are provably no-ops (see note below), so `result * 0x10 + 0xBEEF` always fits. The test asserts C and Rust agree on the exact final value and that it is consistent with the printed one. |
| 24 | `modeselect` | `mode_selector`/`complexity`/`time_offset`/`seed` = `INT_MIN` (`INT_MIN % 4 == 0`, `INT_MIN % 5 == -3`, `INT_MIN % 24 == -8`) | no rejection: `INT_MIN % 4 == 0` selects `"standard"`; `INT_MIN % 5 == -3` hits row 20 |

Out-of-range "enum" values: the API takes no `enum` parameters — the only
enum-like discriminants are the `int level` of `apply_multiplier` (rows 3–4)
and the `int mode_selector` index of `modeselect` (row 19). Both are covered
above with values that have no valid variant.

## Status

All 24 rows have a passing differential test.

| row | test |
|-----|------|
| 1 | `err_row01_classify_mode_unrecognized` |
| 2 | `err_row02_classify_mode_null` (subprocess, both must `SIGSEGV`) |
| 3 | `err_row03_apply_multiplier_invalid_level` |
| 4 | `err_row04_apply_multiplier_base_overflow` |
| 5–9 | `err_row05_09_convert_time_factor_ranges` |
| 10–13 | `err_row10_13_convert_negative_overflow_ranges` |
| 14–16 | `err_row14_16_get_modified_time_int_overflow` |
| 17–18 | `err_row17_18_hash_time_value_overflow` |
| 19 | `err_row19_modeselect_negative_index_segv` (subprocess) |
| 20 | `err_row20_modeselect_negative_complexity` |
| 21 | `err_row21_modeselect_seed_nonzero_result1` |
| 22 | `err_row22_modeselect_time_offset_nonzero_result2` |
| 23 | `err_row23_modeselect_final_overflow` |
| 24 | `err_row24_modeselect_int_min_args` |

## Notes discovered while writing the tests

* **`result ^= (result1 & 0xFF)` and `result ^= (result2 & 0xFF00)` are dead.**
  Inside `modeselect`, `result1` is `0` (only when `seed == 0`) or `INT_MIN`
  (`0x80000000`) — rows 21/9 — and `result2` likewise (rows 22/9). Both
  `0x80000000 & 0xFF` and `0x80000000 & 0xFF00` are `0`, so neither `^=` can
  ever change `result`. Mutating either mask in the Rust translation is
  therefore undetectable *through `modeselect`*; the underlying converters are
  covered directly instead (rows 5–13, and `CONFIGS.md` rows 17–28).
* **`printf("%ld", (long)modified_time)` cannot be distinguished from `%d`.**
  `get_modified_time` returns `(time()>>29) + offset` where `offset` is an
  `int`. `offset = 86400*d + 3600*h` mod 2^32 is always a multiple of
  `gcd(3600, 2^32) = 16`, so `offset <= 2147483632` and
  `modified_time <= 2147483635` — always inside `int` range at the current
  epoch (`time() >> 29 == 3`). Noted so the (equivalent) format string is not
  mistaken for an untested code path.
* **No `assert`, no error enum, no `errno`, no `NULL` return** anywhere in
  `lib.c`; the complete rejection vocabulary is `0x00`, `0xDEAD`, `INT_MIN`,
  silent wrap-around, and `SIGSEGV`.
