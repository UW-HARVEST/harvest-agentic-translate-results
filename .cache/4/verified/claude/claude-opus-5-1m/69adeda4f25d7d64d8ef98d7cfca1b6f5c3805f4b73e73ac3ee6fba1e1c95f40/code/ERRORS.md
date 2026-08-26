# ERRORS.md — Error-surface table (Phase C)

Mechanically derived from `c_src/src/pow.c`. The complete rejection surface of
this library is these 8 lines:

```c
  errno = 0;                                   // line 34  <- pre-existing errno is DISCARDED
  double result = pow(base, exponent);          // line 35
  if (errno == EDOM) {                          // line 36
    fprintf(stderr, "Domain error: pow(%.2f, %.2f) is undefined in the real "
                    "number domain.\n", base, exponent);
    return -1;                                  // line 41
  } else if (errno == ERANGE) {                  // line 42
    fprintf(stderr, "Range error: pow(%.2f, %.2f) caused overflow or "
                    "underflow.\n", base, exponent);
    return -1;                                  // line 46
  }
  return result;                                // line 49
```

There are exactly **two** error-return statements (both `return -1`), reached
via `errno == EDOM` and `errno == ERANGE`. Because a rejection is only
observable from the *combination* of (return value, stderr bytes, residual
errno) — and because `-1.0` is also a perfectly legal `pow` result — every row
below asserts **all three** observables, not just the return value.

`EDOM = 33`, `ERANGE = 34` on Linux/glibc.

## Rows

Each row is one distinct trigger reaching one of the rejection branches.
All rows are in `tests/errors.rs`.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| E1 | `my_pow` | **EDOM branch** — negative finite base, non-integer finite exponent, e.g. `(-2.0, 0.5)` | ret `-1.0` (bits `0xBFF0000000000000`); stderr `Domain error: pow(-2.00, 0.50) is undefined in the real number domain.\n`; residual `errno == EDOM` |
| E2 | `my_pow` | **EDOM branch**, non-representable root, e.g. `(-8.0, 1.0/3.0)` | ret `-1.0`; stderr `Domain error: pow(-8.00, 0.33) is undefined in the real number domain.\n` (note `%.2f` truncates `0.333…` to `0.33`); residual `errno == EDOM` |
| E3 | `my_pow` | **EDOM branch**, exponent one ULP off an integer, e.g. `(-2.0, nextafter(3.0, 4.0))` | ret `-1.0`; stderr renders the exponent as `3.00` even though it is *not* an integer; residual `errno == EDOM` |
| E4 | `my_pow` | **ERANGE branch, pole error** — `base == +0.0`, exponent negative odd integer `(0.0, -1.0)` | ret `-1.0`; stderr `Range error: pow(0.00, -1.00) caused overflow or underflow.\n`; residual `errno == ERANGE` |
| E5 | `my_pow` | **ERANGE branch, pole error** — `base == -0.0`, exponent negative odd integer `(-0.0, -1.0)` | ret `-1.0`; stderr `Range error: pow(-0.00, -1.00) …\n` (sign of zero is visible in `%.2f`); residual `errno == ERANGE` |
| E6 | `my_pow` | **ERANGE branch, pole error** — `base == ±0.0`, exponent negative *even* integer `(0.0, -2.0)` | ret `-1.0`; stderr `Range error: pow(0.00, -2.00) …\n`; residual `errno == ERANGE` |
| E7 | `my_pow` | **ERANGE branch, pole error** — `base == ±0.0`, exponent negative *non*-integer `(0.0, -0.5)` | ret `-1.0`; stderr `Range error: pow(0.00, -0.50) …\n`; residual `errno == ERANGE` |
| E8 | `my_pow` | **ERANGE branch, overflow** — result magnitude `> DBL_MAX`, e.g. `(10.0, 400.0)` | ret `-1.0`; stderr `Range error: pow(10.00, 400.00) …\n`; residual `errno == ERANGE` |
| E9 | `my_pow` | **ERANGE branch, overflow** from a huge base, `(DBL_MAX, 2.0)` | ret `-1.0`; stderr prints `DBL_MAX` via `%.2f` as a **309-digit** decimal `179769313486231570814527423731704356798070567525844996598917476803157260780028538760589558632766878171540458953514382464234321326889464182768467546703537516986049910576551282076245490090389328944075868508455133942304583236903222948165808559332123348274797826204144723168738177180919299881250404026184124858368.00`; residual `errno == ERANGE` |
| E10 | `my_pow` | **ERANGE branch, overflow** from a subnormal base with negative exponent, `(5e-324, -1.0)` | ret `-1.0`; stderr `Range error: pow(0.00, -1.00) …\n` (subnormal rounds to `0.00` under `%.2f`, so the message is indistinguishable from E4 — this must be reproduced, not "fixed"); residual `errno == ERANGE` |
| E11 | `my_pow` | **ERANGE branch, underflow** — result rounds below `DBL_MIN`, e.g. `(10.0, -400.0)` | ret `-1.0`; stderr `Range error: pow(10.00, -400.00) …\n`; residual `errno == ERANGE` |
| E12 | `my_pow` | **ERANGE branch, underflow** from a small base, `(DBL_MIN, 2.0)` | ret `-1.0`; stderr `Range error: pow(0.00, 2.00) …\n` (`DBL_MIN` renders as `0.00`); residual `errno == ERANGE` |
| E13 | `my_pow` | **ERANGE branch, underflow** from a subnormal base, `(5e-324, 2.0)` and `(-5e-324, 3.0)` | ret `-1.0`; stderr `… pow(0.00, 2.00) …` / `… pow(-0.00, 3.00) …`; residual `errno == ERANGE` |
| E14 | `my_pow` | **ERANGE branch, one step past the overflow boundary** — smallest exponent that overflows for a given base (found by `nextafter` bisection around `log(DBL_MAX)/log(base)`) | the pair straddling the boundary must classify identically in C and Rust: the lower side returns a finite value with clean errno, the upper side returns `-1.0` + range message |
| E15 | `my_pow` | **ERANGE branch, one step past the underflow boundary** (same bisection, negative side) | same straddle requirement as E14 |
| E16 | `my_pow` | **`errno = 0` on line 34 must DISCARD caller state** — caller sets `errno = EDOM` (33), then calls with a perfectly valid pair `(2.0, 10.0)` | ret `1024.0`, **no stderr at all** — a translation that read `errno` without clearing it first would wrongly report a domain error |
| E17 | `my_pow` | same as E16 with `errno = ERANGE` (34) preset, valid pair `(2.0, 3.0)` | ret `8.0`, no stderr |
| E18 | `my_pow` | same as E16 with an arbitrary unrelated preset `errno = 22` (EINVAL), valid pair `(3.0, 3.0)` | ret `27.0`, no stderr |
| E19 | `my_pow` | **`-1.0` is NOT a sentinel** — `(-1.0, 3.0)` legitimately evaluates to `-1.0` with `errno == 0` | ret `-1.0` **and no stderr**; distinguishes a real result from a rejection. Also `(-1.0, -1.0)`. |
| E20 | `my_pow` | **neither branch taken for IEEE specials** — glibc's `pow` does *not* set `errno` for NaN/Inf specials, so they fall through to `return result` | `(NAN, 2.0) -> NaN`, `(2.0, NAN) -> NaN`, `(1.0, NAN) -> 1.0`, `(NAN, 0.0) -> 1.0`, `(INF, 2.0) -> INF`, `(-INF, 3.0) -> -INF`, `(2.0, -INF) -> +0.0`; **no stderr**, residual `errno == 0` |
| E21 | `my_pow` | **stderr write failure path** — fd 2 closed/`EBADF` while an error message is emitted (`(-2.0, 0.5)`) | the C ignores `fprintf`'s return value, so behaviour is unchanged: ret `-1.0`; must not abort/panic in Rust |
| E22 | `my_pow` | **exhaustive errno-branch classification over random inputs** — for uniformly random bit patterns, whichever branch the C takes (`EDOM` / `ERANGE` / none) the Rust must take the same one | identical (return bits, stderr bytes, residual errno) triple for every input |

## Generic C-API boundary classes

The instructions require covering null pointers, zero/oversized lengths, and
out-of-range enum values. This API's signature is
`double my_pow(double base, double exponent)`:

| generic class | applicability | how it is covered |
|---|---|---|
| null pointers | **N/A** — no pointer, array, string or struct parameters, and no out-params. Both arguments are passed by value. Nothing can be null. | n/a |
| zero / oversized lengths | **N/A** — no length, count or size parameter exists. | n/a |
| out-of-range enum values across FFI | **N/A** — no enum, bitflag, or mode parameter exists; there is no discrete-valued input whose domain could be exceeded. | n/a |
| **the actual equivalent for this API**: an `f64` argument accepts *any* of the 2^64 bit patterns, so the "value with no valid variant" analogue is the non-finite / non-canonical float space | rows E20, E22 plus `tests/configs.rs` rows C26–C31: quiet NaN, **signaling** NaN (`0x7FF0000000000001` — must be quieted to `0x7FF8000000000001`, payload preserved), negative NaN (`0xFFF8…`), NaN with arbitrary random payloads, `±INF`, `±0.0`, `±DBL_MAX`, `±DBL_MIN`, `±5e-324`, and fully random `u64`-as-`f64` fuzzing |

## Checklist

- [x] E1  EDOM: negative base, fractional exponent
- [x] E2  EDOM: negative base, 1/3 exponent (`%.2f` truncation)
- [x] E3  EDOM: negative base, exponent 1 ULP off integer
- [x] E4  ERANGE pole: `+0` ^ negative odd int
- [x] E5  ERANGE pole: `-0` ^ negative odd int (`-0.00` rendering)
- [x] E6  ERANGE pole: `±0` ^ negative even int
- [x] E7  ERANGE pole: `±0` ^ negative non-int
- [x] E8  ERANGE overflow: `(10, 400)`
- [x] E9  ERANGE overflow: `DBL_MAX` base (309-digit `%.2f`)
- [x] E10 ERANGE overflow: subnormal base, negative exponent
- [x] E11 ERANGE underflow: `(10, -400)`
- [x] E12 ERANGE underflow: `DBL_MIN` base
- [x] E13 ERANGE underflow: subnormal bases, both signs
- [x] E14 ERANGE overflow boundary straddle (`nextafter`)
- [x] E15 ERANGE underflow boundary straddle (`nextafter`)
- [x] E16 caller `errno = EDOM` preset must be discarded
- [x] E17 caller `errno = ERANGE` preset must be discarded
- [x] E18 caller `errno = EINVAL` preset must be discarded
- [x] E19 legitimate `-1.0` result is not a rejection (no stderr)
- [x] E20 IEEE specials set no errno, take neither branch
- [x] E21 stderr unwritable (`EBADF`) does not change behaviour
- [x] E22 randomized errno-branch classification agreement
