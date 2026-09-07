# ERRORS.md — Phase A error-surface table

Derived mechanically from `c_src/src/lib.c`. Grep audit of the whole file:

```
grep -nE 'assert|RETURN_ERROR|return *-|return *NULL|errno|== *NULL|goto|exit\(|abort' src/lib.c
  -> 0 hits (outside the licence comment)
```

This library has **no error enum, no `errno` use, no asserts, no null checks,
and no negative/`NULL` sentinel returns**. Every function returns
unconditionally. Its "rejection" surface is therefore made of (a) explicit
fallback/sentinel *values* the C computes for unrecognised or out-of-domain
input, (b) hardware-defined results for out-of-range conversions, and (c) two
genuinely undefined-behaviour paths. Each distinct branch below is one row.

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| 1  | `classify_mode` | `mode` matches none of the 4 literals — final `return 0x00` (`src/lib.c:39`) | returns `0` |
| 2  | `classify_mode` | `mode` is the empty string `""` (falls to same `return 0x00`) | returns `0` |
| 3  | `classify_mode` | `mode` is a strict *prefix* of a literal, e.g. `"standar"` — `strcmp != 0` | returns `0` |
| 4  | `classify_mode` | `mode` has a valid literal as a strict prefix, e.g. `"standardX"`, `"turbo "` | returns `0` |
| 5  | `classify_mode` | `mode` differs only in case, e.g. `"Standard"`, `"TURBO"` (`strcmp` is case-sensitive) | returns `0` |
| 6  | `classify_mode` | `mode == NULL` → `strcmp(NULL, "standard")` dereferences NULL | **UB**: SIGSEGV. Not differentially testable; see "UB rows" below |
| 7  | `apply_multiplier` | `level` is any value outside `{0,1,2,3,4}` → `default:` branch (`src/lib.c:57-58`) discards `base` entirely | returns `0xDEAD` = `57005` |
| 8  | `apply_multiplier` | `level == 5` (one step past the largest valid case label) | returns `0xDEAD` |
| 9  | `apply_multiplier` | `level == -1` (one step past the smallest valid case label) | returns `0xDEAD` |
| 10 | `apply_multiplier` | `level == INT_MIN` / `INT_MAX` (extreme out-of-range enum-like value across FFI) | returns `0xDEAD` |
| 11 | `apply_multiplier` | `level` valid but `base` near `INT_MAX` so `base + 0xFF + …` overflows signed `int` | **UB** in ISO C; at `-O0` gcc wraps two's-complement. Rust must produce the same wrapped `int` |
| 12 | `convert_time_factor` | `factor * 1e12` truncates above `INT_MAX` (e.g. `factor = 1.0`) → `(int)` of out-of-range double | **UB** in ISO C; x86-64 `cvttsd2si` yields the *integer indefinite* value `INT_MIN` = `-2147483648` |
| 13 | `convert_time_factor` | `factor * 1e12` truncates below `INT_MIN` (e.g. `factor = -1.0`) | `INT_MIN` (`cvttsd2si` indefinite) |
| 14 | `convert_time_factor` | `factor = NaN` | `INT_MIN` |
| 15 | `convert_time_factor` | `factor = +inf` / `-inf` | `INT_MIN` |
| 16 | `convert_time_factor` | `factor` so small that `factor*1e12` is a subnormal/`±0.0` → truncates to 0 | returns `0` (incl. `-0.0` → `0`) |
| 17 | `convert_time_factor` | boundary: `factor*1e12` exactly `2147483647.x` (in range) vs `2147483648.0` (out) | in-range → truncated value; out-of-range → `INT_MIN` |
| 18 | `convert_negative_overflow` | `value * -1e15` truncates below `INT_MIN` (e.g. `value = 1.0`) | `INT_MIN` |
| 19 | `convert_negative_overflow` | `value * -1e15` truncates above `INT_MAX` (e.g. `value = -1.0`) | `INT_MIN` |
| 20 | `convert_negative_overflow` | `value = NaN` (`NaN * -1e15 = NaN`) | `INT_MIN` |
| 21 | `convert_negative_overflow` | `value = ±inf` | `INT_MIN` |
| 22 | `convert_negative_overflow` | `value = 0.0` → `-0.0`; `value = -0.0` → `+0.0` | returns `0` (sign of zero does not survive `(int)`) |
| 23 | `get_modified_time` | `offset_days * 86400` overflows `int` (any `\|offset_days\| > 24855`, e.g. `100000`) | **UB**; at `-O0` wraps as `int`, then sign-extends to `time_t` and is added to `time(NULL)>>29` |
| 24 | `get_modified_time` | `offset_hours * 3600` overflows `int` (any `\|offset_hours\| > 596523`) | same wrapping `int` semantics |
| 25 | `get_modified_time` | `offset_days = INT_MIN`, `offset_hours = INT_MIN` (extreme) | wrapped `int` sum, sign-extended |
| 26 | `get_modified_time` | the sum of the two products overflows `int` even when each product fits | wrapped `int` sum |
| 27 | `hash_time_value` | `t < 0` — `bytes[i] << ((i%4)*8)` shifts a value ≥ `0x80` into the sign bit of `int` | **UB**; at `-O0` the shift wraps. Masked result is always in `[0, 0x7FFFFFFF]` |
| 28 | `hash_time_value` | `t = INT64_MIN` / `INT64_MAX` (all high bytes set) | wrapped `hash & 0x7FFFFFFF`; never negative |
| 29 | `hash_time_value` | `hash *= 0x1F` overflows signed `int` on essentially every iteration | **UB**; at `-O0` wraps. Result always `>= 0` because of `& 0x7FFFFFFF` |
| 30 | `modeselect` | `complexity % 5` is negative (any `complexity < 0` not a multiple of 5, e.g. `-1` → `-1`) → `apply_multiplier` hits `default:` | multiplier is `0xDEAD`; printed as `Complexity level: -1, Multiplier: 0xDEAD` |
| 31 | `modeselect` | `seed % 24` negative (any `seed < 0` not a multiple of 24) → negative `offset_hours` into `get_modified_time` | negative offset added; no rejection |
| 32 | `modeselect` | `mode_selector < 0` and `mode_selector % 4 != 0` → `mode_index` ∈ `{-1,-2,-3}` → `modes[mode_index]` reads **out of bounds** past the 4-element stack array, then `strcmp` on the garbage pointer | **UB**: empirically SIGSEGV at `-O0`. See "UB rows" |
| 33 | `modeselect` | `mode_selector = INT_MIN` (`INT_MIN % 4 == 0`) → `mode_index = 0`, in bounds | valid: `"standard"`, `0x10`. No rejection (the `INT_MIN % -1` overflow trap does not apply to `% 4`) |
| 34 | `modeselect` | `time_offset` large enough that `get_modified_time`'s `int` math overflows (row 23 reached through `modeselect`) | wrapped, then hashed |
| 35 | `modeselect` | `seed` large enough that `(double)seed * 1e8 * 1e12` overflows `int` (any `seed != 0`) → row 12 | `Result 1: -2147483648 (0x80000000)` |
| 36 | `modeselect` | `time_offset != 0` so `(double)time_offset * -1e7 * -1e15` overflows → row 18/19 | `Result 2: -2147483648 (0x80000000)` |
| 37 | `modeselect` | `result * 0x10 + 0xBEEF` overflows signed `int` (reachable via large `time_hash`/multiplier sums) | **UB**; at `-O0` wraps as `int` |

## UB rows that are not differentially testable (rows 6 and 32)

Rows 6 and 32 are the only rows where the C **crashes** rather than returning a
value, so "the same error/rejection" cannot be asserted against a live C call:

* **Row 6** — `classify_mode(NULL)`: glibc `strcmp` dereferences the null
  pointer. Confirmed SIGSEGV. The Rust `classify_mode` also dereferences the raw
  pointer (`cstr_eq` → `*p.add(i)`), so both fault; verified by running each in a
  forked child and asserting both die on `SIGSEGV` (same signal), rather than by
  comparing return values.
* **Row 32** — `modeselect(-1|-2|-3|-5|…, …)`: verified empirically that the C
  segfaults (exit 139) for `mode_selector % 4 ∈ {-1,-2,-3}` while `mode_selector
  = -4` (index 0) returns normally and matches Rust exactly. The Rust
  deliberately substitutes an empty string for the out-of-range index, which
  yields `classify_mode == 0x00` and returns normally. This divergence is
  **inherent**: the C's behaviour is reading unmapped/garbage stack memory, which
  has no deterministic value to match. Differential tests therefore restrict
  `modeselect`'s `mode_selector` to `>= 0` (plus `INT_MIN` and negative multiples
  of 4, which are in bounds), and a separate test documents the crash asymmetry
  by running the C in a forked child.
