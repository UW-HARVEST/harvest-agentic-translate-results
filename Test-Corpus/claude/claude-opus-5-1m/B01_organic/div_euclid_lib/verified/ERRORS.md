# ERRORS.md — Phase A: ERROR-SURFACE TABLE

Derived mechanically from every rejection / guard / min-max constant in
`c_src/src/lib.c` (the whole file is 31 lines; every `return`, every `==`/`!=`
guard and every constant is accounted for below).

Grep of the complete rejection surface:

```
4:    if (v2 == 0) {          <- the only explicit rejection
5:        return 0;           <-   sentinel result 0
11:        else if (v2 != (-0x7fffffff - 1))   <- INT_MIN guard (min constant)
15:    else if (v1 != (-0x7fffffff - 1))       <- INT_MIN guard (min constant)
18:        else if (v2 != (-0x7fffffff - 1))   <- INT_MIN guard (min constant)
24:    else if (v2 != (-0x7fffffff - 1))       <- INT_MIN guard (min constant)
```

* No `assert` / `NDEBUG` behaviour anywhere.
* No pointer parameters ⇒ no `return NULL`, no null checks.
* No length/size parameters ⇒ no oversized-length checks.
* No `enum` parameters ⇒ no enum-range checks (see rows E13–E15 for how the
  FFI-boundary equivalents are still covered).
* `-0x7fffffff - 1` (= `INT_MIN` = `-2147483648`) is the only named
  min/max constant; it appears in 4 guards, each guarding a different
  arithmetic branch against the `INT_MIN` negation / `INT_MIN / -1` overflow.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | pass |
|---|----------|---------------------------------------------|-------------------|------|------|
| E1 | `div_euclid` | `v2 == 0`, `v1` arbitrary non-extreme (randomized) — division-by-zero rejection at `lib.c:4` | returns `0`; no `SIGFPE`, no trap | `e1_div_by_zero_random` | [x] |
| E2 | `div_euclid` | `v2 == 0 && v1 == 0` | returns `0` | `e2_zero_over_zero` | [x] |
| E3 | `div_euclid` | `v2 == 0 && v1 == INT_MIN` (rejection wins over the `INT_MIN` guards) | returns `0` | `e3_int_min_over_zero` | [x] |
| E4 | `div_euclid` | `v2 == 0 && v1 == INT_MAX` | returns `0` | `e4_int_max_over_zero` | [x] |
| E5 | `div_euclid` | `v1 >= 0 && v2 == INT_MIN` — guard `v2 != (-0x7fffffff-1)` at `lib.c:11` is FALSE, so `-v2` is never evaluated; takes `q = 0, r = v1` | returns `0` (`r = v1 >= 0` ⇒ no tail adjust) | `e5_nonneg_over_int_min` | [x] |
| E6 | `div_euclid` | `v1 < 0 && v1 != INT_MIN && v2 == INT_MIN` — guard at `lib.c:18` FALSE; takes `q = 1, r = v1 - q*v2` | returns `1` (`r = v1 - INT_MIN > 0`) | `e6_neg_over_int_min` | [x] |
| E7 | `div_euclid` | `v1 == INT_MIN && v2 == INT_MIN` — guards at `lib.c:15` and `lib.c:24` both FALSE; takes `q = 1, r = 0` | returns `1` | `e7_int_min_over_int_min` | [x] |
| E8 | `div_euclid` | `v1 == INT_MIN && v2 == -1` — the guarded branch at `lib.c:24-25` performs `q = ((-(v1-v2)) / (-v2)) + 1` = `INT_MAX + 1`, i.e. C signed-overflow | returns `INT_MIN` (`-2147483648`, wrapped) — must not panic | `e8_int_min_over_minus_one` | [x] |
| E9 | `div_euclid` | `v1 == INT_MIN && v2 == 1` — `lib.c:22-23`, `q = -(INT_MAX/1) - 1` = `INT_MIN` (boundary of representable quotient) | returns `INT_MIN` | `e9_int_min_over_one` | [x] |
| E10 | `div_euclid` | `v1 == INT_MIN && v2 > 0` arbitrary (randomized) — guard at `lib.c:15` FALSE, so the code must never evaluate `-v1` | same value as C, no trap | `e10_int_min_over_positive_random` | [x] |
| E11 | `div_euclid` | `v1 == INT_MIN + 1 && v2 == -1` — guards at `lib.c:15`/`lib.c:18` both TRUE, `(-v1)/(-v2)` = `INT_MAX/1` (largest in-range result) | returns `INT_MAX` (`2147483647`) | `e11_int_min_plus_one_over_minus_one` | [x] |
| E12 | `div_euclid` | `v1 == INT_MAX && v2 == -1` — `lib.c:11-12`, `q = -(INT_MAX/1)` | returns `-2147483647` | `e12_int_max_over_minus_one` | [x] |
| E13 | `div_euclid` | FFI boundary: argument registers carry dirty upper 32 bits (symbol called through an `extern "C" fn(i64, i64) -> i64` signature) — every 64-bit pattern is "one step past" the `int` range | both sides truncate to `int`; low 32 bits of the return are identical to the `c_int` call | `e13_dirty_high_bits_abi` | [x] |
| E14 | `div_euclid` | Out-of-range *enum-like* value across FFI: `div_euclid` takes no `enum`, so the analogue is "every 32-bit pattern is a valid `int`" — the full domain is exercised, incl. all `0x80000000`/`0x7fffffff`-class patterns | identical for every bit pattern tested (dense boundary sweep) | `e14_all_bit_patterns_boundary` | [x] |
| E15 | `div_euclid` | Null pointer / zero-or-oversized length: **N/A** — the ABI has no pointer and no length parameter (`int div_euclid(int, int)`); nothing to pass as `NULL`. Asserted structurally by the header/symbol check. | N/A | `e15_no_pointer_or_length_params` (documented + symbol-signature check) | [x] |

All 15 rows have a differential test in `tests/phase_c_errors.rs`; every row is
asserted against **both** `.so`s loaded with `libloading` and compared for the
same returned value (not merely "both failed").
