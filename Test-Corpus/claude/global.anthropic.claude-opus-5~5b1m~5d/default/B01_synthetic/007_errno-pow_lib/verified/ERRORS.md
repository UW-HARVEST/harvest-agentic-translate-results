# ERRORS.md — Phase A: Error-surface table

Mechanically derived from `c_src/src/pow.c`. Grep results for every rejection
construct in the C source:

```
$ grep -n 'return -1\|return NULL\|assert\|RETURN_ERROR\|errno\|EDOM\|ERANGE\|exit(' c_src/src/pow.c
26:#include <errno.h>
34:  errno = 0;
36:  if (errno == EDOM)   -> fprintf(stderr, "Domain error: ...") ; return -1;
42:  } else if (errno == ERANGE) -> fprintf(stderr, "Range error: ...") ; return -1;
```

There are exactly **TWO** rejection branches in the C (`return -1` at line 41 and
line 46). There are **no** `assert`s, **no** null-pointer checks, **no** explicit
range checks, **no** error enums, and **no** min/max constants — the function
takes two by-value `double`s. `EDOM == 33` and `ERANGE == 34` on this platform
(verified by compiling a probe against the same glibc).

Both rejections are driven *indirectly*: the C sets `errno = 0`, calls glibc
`pow`, and inspects `errno`. So the exact invalid inputs are exactly the inputs
for which **glibc `pow` sets `errno`**. Each such distinct trigger class is one
row below (all verified empirically against the same glibc `pow` used by both
libraries).

Every row's expected result is the pair *(return value, stderr bytes)*, since the
C also writes a diagnostic; the tests assert on **both**, byte-for-byte.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| E1 | `my_pow` | `errno == EDOM` branch, line 36–41: negative finite base with **non-integral finite** exponent, e.g. `(-2.0, 0.5)` — glibc `pow` returns `-NaN` and sets `errno=EDOM` | returns `-1.0` exactly; writes `"Domain error: pow(-2.00, 0.50) is undefined in the real number domain.\n"` to `stderr` |
| E2 | `my_pow` | Same EDOM branch, non-half-integer non-integral exponent, e.g. `(-1.5, 1.5)`, `(-3.0, 2.7)`, `(-0.5, -1.3)` | returns `-1.0`; `"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n"` |
| E3 | `my_pow` | `errno == ERANGE` branch, line 42–46: **overflow to +Inf** from large base, e.g. `(1e300, 3.0)`, `(2.0, 100000.0)` | returns `-1.0`; writes `"Range error: pow(1e+300-as-%.2f, 3.00) caused overflow or underflow.\n"` to `stderr` |
| E4 | `my_pow` | ERANGE branch: **overflow one step past `DBL_MAX`**, e.g. `(1.7976931348623157e308, 1.0000001)` | returns `-1.0`; range-error message |
| E5 | `my_pow` | ERANGE branch: **overflow to −Inf** (negative base, odd integral exponent), e.g. `(-1e300, 3.0)` | returns `-1.0`; range-error message |
| E6 | `my_pow` | ERANGE branch: **underflow flushing to +0**, e.g. `(1e-300, 3.0)`, `(2.0, -100000.0)` | returns `-1.0`; range-error message |
| E7 | `my_pow` | ERANGE branch: **underflow with non-integral exponent**, e.g. `(1e-300, 1.2)` | returns `-1.0`; range-error message |
| E8 | `my_pow` | ERANGE branch: **underflow to −0** (negative base, odd integral exponent), e.g. `(-1e-300, 3.0)` | returns `-1.0`; range-error message |
| E9 | `my_pow` | ERANGE branch: **pole / divide-by-zero**, `+0.0` base with negative **odd integral** exponent, `(0.0, -3.0)` — glibc returns `+Inf`, `errno=ERANGE` | returns `-1.0`; range-error message |
| E10 | `my_pow` | ERANGE branch: pole, `+0.0` base with negative **even integral** exponent, `(0.0, -2.0)` | returns `-1.0`; range-error message |
| E11 | `my_pow` | ERANGE branch: pole, `+0.0` base with negative **non-integral** exponent, `(0.0, -0.5)` | returns `-1.0`; range-error message |
| E12 | `my_pow` | ERANGE branch: pole, `-0.0` base with negative **odd integral** exponent, `(-0.0, -3.0)` — glibc returns `-Inf`, `errno=ERANGE` | returns `-1.0`; range-error message |
| E13 | `my_pow` | ERANGE branch: pole, `-0.0` base with negative **even integral** exponent, `(-0.0, -2.0)` | returns `-1.0`; range-error message |
| E14 | `my_pow` | ERANGE branch: pole, `-0.0` base with negative **non-integral** exponent, `(-0.0, -1.5)` | returns `-1.0`; range-error message |

## Adjacent boundaries that DO NOT error (negative controls — must NOT print, must NOT return −1 via the error path)

These are the "one step past / one step inside" cases that distinguish a correct
translation from one that mis-tests `errno`. They are included as explicit
rows because a Rust version that (for example) forgot to zero `errno`, or that
used Rust's `f64::powf` (which never touches `errno`), would diverge here.

| # | function | condition | expected C result |
|---|----------|-----------|-------------------|
| N1 | `my_pow` | negative base, **integral** exponent (`(-2.0, 3.0)`, `(-2.0, 4.0)`) → `errno` stays 0 | returns `pow()` result (`-8.0`, `16.0`); **no** stderr output |
| N2 | `my_pow` | any NaN involved (`(NaN, 2.0)`, `(2.0, NaN)`, `(NaN, NaN)`, `(-2.0, NaN)`) → `errno` stays 0 | returns NaN (exact bits `7ff8000000000000`); no stderr |
| N3 | `my_pow` | `pow(1.0, NaN) == 1.0` and `pow(NaN, 0.0) == 1.0`, `errno` stays 0 | returns `1.0`; no stderr |
| N4 | `my_pow` | infinite operands (`(inf,0)`, `(-inf,3)`, `(-inf,-3)`, `(0.5,inf)`, `(2,-inf)`, `(-2,-inf)`, `(inf,-inf)`, `(-0.0,-inf)`, `(-1.0,inf)`, `(-2.0,inf)`) → `errno` stays 0 | returns the IEEE `pow` value; no stderr. NB `(-0.0,-inf)` → `+inf` with **no** ERANGE, unlike E12 |
| N5 | `my_pow` | `+0.0` base with **positive** exponent `(0.0, 2.0)` → `errno` 0 | returns `+0.0`; no stderr |
| N6 | `my_pow` | result is the **smallest normal** `(2.0, -1022.0)` → `errno` 0 | returns `2.2250738585072014e-308`; no stderr |
| N7 | `my_pow` | result is **subnormal but nonzero** `(2.0, -1040.0)` → glibc does **not** set ERANGE | returns `8.4879831638610893e-314` (bits `0000000400000000`); no stderr |
| N8 | `my_pow` | genuine `-1.0` result via the **happy path**, e.g. `(-1.0, 3.0)` → `errno` 0 | returns `-1.0` **without** any stderr output (aliases the error sentinel — must not print) |
| N9 | `my_pow` | **one step inside** the overflow threshold: `(2.0, 1023.9999999)`, `(2.0, 1024.0 - k·1e-9)`, `(f64::MAX, 1.0)` → still finite, `errno` 0 | returns the finite value; no stderr. (Mirror of E4. NB `(-2.0, 1023.9999999)` is **E2**, not N9: a negative base with a non-integral exponent is EDOM regardless of the result's magnitude.) |

## Notes on generic C-API boundaries

- **Null pointers:** N/A — `my_pow` has no pointer parameters.
- **Zero / oversized lengths:** N/A — no length or buffer parameters.
- **Out-of-range enum values across FFI:** N/A — no enum parameters. The only
  parameter type is `double`, whose entire bit-space is covered by the rows above
  plus the randomized sweeps in Phase B (including signalling/quiet NaN payloads
  and both signed zeros).
- **Stale `errno`:** the C sets `errno = 0` before calling `pow`, so a pre-existing
  `errno` must be ignored. Covered as a dedicated test (set `errno` to `EDOM` in
  the caller, then call with a valid input, and assert no error path is taken).

## Checklist

- [x] E1 — EDOM, negative base, non-integral exponent
- [x] E2 — EDOM, further non-integral exponents
- [x] E3 — ERANGE, overflow to +Inf
- [x] E4 — ERANGE, overflow one step past DBL_MAX
- [x] E5 — ERANGE, overflow to −Inf
- [x] E6 — ERANGE, underflow to +0
- [x] E7 — ERANGE, underflow, non-integral exponent
- [x] E8 — ERANGE, underflow to −0
- [x] E9 — ERANGE, pole +0 / negative odd int
- [x] E10 — ERANGE, pole +0 / negative even int
- [x] E11 — ERANGE, pole +0 / negative non-int
- [x] E12 — ERANGE, pole −0 / negative odd int
- [x] E13 — ERANGE, pole −0 / negative even int
- [x] E14 — ERANGE, pole −0 / negative non-int
- [x] N1–N8 — non-erroring boundary controls
- [x] N9 — one step inside the overflow threshold (no error)
- [x] Stale-`errno` control (incl. out-of-range / non-`errno` `int` values such as
      `i32::MIN`, `i32::MAX`, `EDOM±1`, `ERANGE±1` — the FFI analogue of an
      out-of-range enum, since C accepts any `int` in that slot)
- [x] Full exponent-field sweep of both operands (all 2048 exponent fields ×
      4 mantissas × 2 signs), crossing every zero/subnormal/normal/inf/NaN boundary
- [x] Anti-vacuity self-check: the harness provably detects both an
      always-`0.0` mutant (6/6) and an `f64::powf` mutant that never sets
      `errno` (5/6), and the random sweep genuinely reaches all three
      observable `errno` states `{0, EDOM, ERANGE}`
