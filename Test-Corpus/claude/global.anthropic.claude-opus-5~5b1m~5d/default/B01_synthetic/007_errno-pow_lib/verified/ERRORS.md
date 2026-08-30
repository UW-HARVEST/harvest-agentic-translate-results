# ERRORS.md — Error-surface table (Phase A)

Mechanically derived from `c_src/src/pow.c`. The whole file is 19 lines of
code; every rejection path is listed below.

Grep inventory of the C error surface:

```
$ grep -n 'return\|errno\|assert\|EDOM\|ERANGE\|NULL\|if\|else' c_src/src/pow.c
34:  errno = 0;
36:  if (errno == EDOM) {
41:    return -1;
42:  } else if (errno == ERANGE) {
46:    return -1;
49:  return result;
```

* error-return statements: **2** (`return -1` at line 41, `return -1` at line 46)
* `assert`: **0**
* null checks: **0** (both parameters are `double` by value — there is no
  pointer parameter anywhere in the public API, so no null check exists)
* explicit range checks / min-max constants: **0** (the code never compares
  `base` or `exponent` against a bound; it delegates entirely to libm and then
  inspects `errno`)
* error enums: **0** — the sentinel is the `double` value `-1.0`

So there are exactly **2 distinct rejection branches**, each reached via a
distinct `errno` value set by glibc `pow`. Because each branch is reachable
through several genuinely different classes of input, the table has one row per
branch plus one row per distinct *trigger class* of that branch, since those
classes exercise different libm code paths.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `my_pow` | `errno == EDOM` branch (line 36): finite negative `base` with finite non-integral `exponent`, e.g. `my_pow(-2.0, 0.5)`. glibc `pow` returns NaN and sets `errno = EDOM`. | returns `-1.0`; writes `"Domain error: pow(-2.00, 0.50) is undefined in the real number domain.\n"` to `stderr` | `err_e1_edom_negative_base_fractional_exponent` | [x] |
| E2 | `my_pow` | same `EDOM` branch, second trigger class: negative base whose magnitude is > 1 with a non-integral exponent > 1, e.g. `my_pow(-1.5, 2.5)` — separate libm path from E1 | returns `-1.0`; `"Domain error: ..."` on `stderr` | `err_e2_edom_negative_base_large_fractional_exponent` | [x] |
| E3 | `my_pow` | `errno == ERANGE` branch (line 42), pole error: `base == +0.0` with negative `exponent`, e.g. `my_pow(0.0, -1.0)`. glibc returns `+Inf`, sets `errno = ERANGE`. | returns `-1.0`; writes `"Range error: pow(0.00, -1.00) caused overflow or underflow.\n"` to `stderr` | `err_e3_erange_pole_positive_zero` | [x] |
| E4 | `my_pow` | `ERANGE` branch, pole error with **negative** zero: `base == -0.0`, negative odd-integer `exponent`, e.g. `my_pow(-0.0, -1.0)`. glibc returns `-Inf`, sets `errno = ERANGE`. Distinct sign path from E3. | returns `-1.0`; `"Range error: pow(-0.00, -1.00) ..."` on `stderr` | `err_e4_erange_pole_negative_zero` | [x] |
| E5 | `my_pow` | `ERANGE` branch, pole error with negative **non-integral** exponent: `my_pow(0.0, -0.5)` → `+Inf`, `errno = ERANGE` | returns `-1.0`; `"Range error: ..."` on `stderr` | `err_e5_erange_pole_fractional_exponent` | [x] |
| E6 | `my_pow` | `ERANGE` branch, **overflow**: result exceeds `DBL_MAX`, e.g. `my_pow(1e300, 2.0)` or `my_pow(2.0, 10000.0)` → `+Inf`, `errno = ERANGE`. Note the C prints the full `%.2f` expansion of `1e300` (≈309 digits) — the Rust must reproduce that byte-for-byte. | returns `-1.0`; `"Range error: pow(<309 digits>.00, 2.00) ..."` on `stderr` | `err_e6_erange_overflow` | [x] |
| E7 | `my_pow` | `ERANGE` branch, **underflow**: result is subnormal/zero, e.g. `my_pow(1e-300, 2.0)` or `my_pow(2.0, -10000.0)` → `+0.0`, `errno = ERANGE`. Distinct from E6 (underflow vs overflow libm path). | returns `-1.0`; `"Range error: pow(0.00, 2.00) ..."` on `stderr` (note `%.2f` of `1e-300` prints as `0.00`) | `err_e7_erange_underflow` | [x] |
| E8 | `my_pow` | `ERANGE` branch, overflow with **negative** result: `my_pow(-1e300, 3.0)` → `-Inf`, `errno = ERANGE` | returns `-1.0`; `"Range error: ..."` on `stderr` | `err_e8_erange_overflow_negative` | [x] |

## Generic FFI boundary cases (not table rows, covered anyway)

`my_pow` takes two `double`s by value and returns a `double`. It has **no**
pointer, length, enum, or struct parameter, therefore:

* **null pointers** — not applicable: no pointer parameter exists in the public
  API. Verified: `pow.h` declares only `double my_pow(double, double)`.
* **zero / oversized lengths** — not applicable: no length or buffer parameter.
* **out-of-range enum values** — not applicable: no enum parameter. There is no
  mode/flag argument that could receive an integer with no valid variant.

The equivalent "every value is a legal input" boundary set for a `double` API is
covered as the following extra differential tests, all of which must agree
bit-for-bit:

| case | inputs | why |
|------|--------|-----|
| B1 | quiet NaN as base, as exponent, as both | `pow(NaN, 0)` is specified to return `1.0` with `errno == 0` (no rejection), so the C returns `1.0` not `-1.0` |
| B2 | signalling NaN bit patterns | NaN payload must propagate identically |
| B3 | `±Inf` in either or both arguments | `pow(-1, Inf) == 1`, `errno == 0` — no rejection |
| B4 | `±0.0` with non-negative exponent | `pow(0,0) == 1`, `errno == 0` |
| B5 | `±DBL_MAX`, `±DBL_MIN`, subnormals, `±DBL_EPSILON` | one step past the representable range in each direction |
| B6 | exponent one step past the overflow threshold (`nextafter` around `1024/log2(base)`) | straddles the `ERANGE` boundary — the value *just* inside must return the real result, the value *just* outside must return `-1.0` |
| B7 | errno hygiene: a preceding call that leaves `errno` set to `EDOM`/`ERANGE`, then a valid call | the C sets `errno = 0` first (line 34), so a stale `errno` must **not** cause a spurious `-1.0` |

All rows E1–E8 and B1–B7 are exercised by
`translation/tests/phase_c_errors.rs` through both `.so` files loaded with
`libloading`. Mapping:

| row | test |
|-----|------|
| B1, B4 | `bnd_b1_b4_non_finite_inputs_are_not_rejected` |
| B2 | `bnd_b2_nan_payload_propagation`, `c17_signalling_nan_bit_patterns` |
| B3 | `bnd_b3_infinity_cross_product` |
| B5 | `bnd_b5_representable_range_edges` |
| B6 | `bnd_b6_one_step_past_erange_boundary` |
| B7 | `bnd_b7_stale_errno_does_not_cause_spurious_rejection`, `c24_errno_hygiene_and_statelessness` |
| (extra) `errno` side-effect parity | `bnd_errno_side_effect_parity` |
| (extra) API shape has no pointer/enum params | `bnd_no_pointer_or_enum_parameters_in_api` |

## Status

All 8 error rows and all 7 boundary rows PASS: the C and Rust `.so` return the
same `-1.0` sentinel **and** emit byte-identical `stderr` diagnostics (compared
via `dup2` capture of fd 2), for hand-picked triggers and for thousands of
randomized inputs per row.

Each error test additionally calls `assert_all_return_sentinel`, which fails if
the supposedly-invalid inputs did *not* reach an error branch. Without it a row
could "pass" by comparing two identical success paths and prove nothing.
