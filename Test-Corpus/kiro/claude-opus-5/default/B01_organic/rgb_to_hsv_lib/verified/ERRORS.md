# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep of the whole C source

```
grep -nE 'return|assert|NULL|ERROR|errno|if *\(|#if|switch' c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:19:    if (delta == 0 || max == 0) {
c_src/src/lib.c:23:        return;
c_src/src/lib.c:26:    if (r == max)
c_src/src/lib.c:28:    else if (g == max)
c_src/src/lib.c:33:    if (h < 0)
```

Findings, stated precisely so the table below is not guesswork:

- **0** error-return macros (`RETURN_ERROR`, `return -1`, `return NULL`, …).
- **0** `assert` / `static_assert` / `abort` / `errno` uses.
- **0** null-pointer checks, **0** length or count arguments, **0** explicit
  range checks, **0** min/max limit constants.
- **0** enums, **0** flags, **0** mode arguments anywhere in the public header —
  therefore there is **no out-of-range-enum-value class of input for this API**.
  (Checked: `lib.h` is a single `void rgb_to_hsv(float*, const float*)`
  declaration. There is no integer/enum parameter that could carry a value with
  no valid variant.)
- The function returns `void`. It has **no error channel at all** — no return
  code, no sentinel, no out-param status. Consequently every "rejection" the C
  performs is expressed as *a particular output triple written to `dest`*, or as
  *undefined behaviour* for out-of-contract pointers.

So the error surface consists of (a) the degenerate/short-circuit conditions the
C explicitly tests, (b) the non-finite and signed-zero float inputs that make
those tests behave non-obviously, and (c) the generic FFI pointer boundaries.
Every row is asserted as **exact bitwise equality of the 3 written `f32`s**
between C and Rust (bit patterns, so `NaN` payload and `-0.0` vs `+0.0` are
distinguished), not merely "both did something".

## Error / rejection table

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| E1 | `rgb_to_hsv` | `delta == 0` via `r == g == b` (achromatic), e.g. `{0.5,0.5,0.5}` | line 19 short-circuits: writes `dest = {0.0, 0.0, max}`; `h`/`s` keep their initialisers | `err_e1_delta_zero_achromatic` | [x] |
| E2 | `rgb_to_hsv` | `max == 0` with `delta != 0`, i.e. largest channel is exactly `0.0` and some channel is negative, e.g. `{-1.0, 0.0, 0.0}` | line 19 short-circuits on the **second** disjunct: `dest = {0.0, 0.0, 0.0}`. No division by `max` occurs, so no `inf`/`NaN` is produced | `err_e2_max_zero` | [x] |
| E3 | `rgb_to_hsv` | all channels `0.0` — both disjuncts of line 19 true | `dest = {0.0, 0.0, 0.0}` | `err_e3_all_zero` | [x] |
| E4 | `rgb_to_hsv` | `delta == 0` where `max == min` but both negative, e.g. `{-2,-2,-2}` | `dest = {0.0, 0.0, -2.0}`; `v` is negative — the C does **not** clamp | `err_e4_negative_achromatic` | [x] |
| E5 | `rgb_to_hsv` | `max == -0.0` and `min == -0.0` (all channels `-0.0`): `delta = -0.0 - -0.0 = +0.0`, `+0.0 == 0` true | `dest = {0.0, 0.0, -0.0}` — `v` retains the **negative zero** sign bit | `err_e5_negative_zero` | [x] |
| E6 | `rgb_to_hsv` | mixed `+0.0` / `-0.0`, e.g. `{-0.0, +0.0, -0.0}`: ternaries use `<` / `>` which treat `-0.0 == +0.0`, so the *first* operand wins ties | `delta == 0` → `dest = {0.0, 0.0, v}` where `v`'s sign bit is decided by the tie-breaking order of the C ternaries | `err_e6_mixed_signed_zero` | [x] |
| E7 | `rgb_to_hsv` | `NaN` in `src[0]` (`r`). All `<`/`>` comparisons with `NaN` are false, so `min = b`, `max = b`; `delta = b - b`; `r == max` is false, `g == max` compared normally | falls past line 19 only if `delta != 0`; otherwise `dest = {0,0,max}`. Either way a specific bit pattern that must match exactly | `err_e7_nan_r` | [x] |
| E8 | `rgb_to_hsv` | `NaN` in `src[1]` (`g`) | ternary `min < g` false ⇒ `min = g = NaN`; `max > g` false ⇒ `max = NaN`; `delta = NaN`; `NaN == 0` false and `NaN == 0` false ⇒ **no** short-circuit; `s = NaN/NaN = NaN`; `r == max` false, `g == max` false ⇒ else branch `h = 4 + (r-g)/delta = NaN`; `h < 0` false ⇒ `dest = {NaN, NaN, NaN}` | `err_e8_nan_g` | [x] |
| E9 | `rgb_to_hsv` | `NaN` in `src[2]` (`b`) | analogous non-short-circuiting `NaN` propagation | `err_e9_nan_b` | [x] |
| E10 | `rgb_to_hsv` | `NaN` with a non-canonical payload / signalling `NaN` bit pattern | arithmetic quiets it; the exact resulting payload must match C bit-for-bit | `err_e10_nan_payloads` | [x] |
| E11 | `rgb_to_hsv` | `+inf` as the max channel, e.g. `{inf, 0, 0}` | `max = inf`, `delta = inf`, `s = inf/inf = NaN`, `r == max` true ⇒ `h = (g-b)/inf = 0`, `h *= 60 = 0` ⇒ `dest = {0.0, NaN, inf}` | `err_e11_pos_inf` | [x] |
| E12 | `rgb_to_hsv` | `-inf` present, e.g. `{-inf, 0, 0}` | `min = -inf`, `max = 0`, `delta = inf`; line 19: `delta==0` false but `max==0` **true** ⇒ short-circuit `dest = {0,0,0}` | `err_e12_neg_inf` | [x] |
| E13 | `rgb_to_hsv` | `inf - inf` in `delta`, e.g. `{inf, -inf, 0}` | `delta = inf - (-inf) = inf`; and `{inf, inf, x}` gives `delta = inf - x`; the `{-inf,…}` mixes give `NaN` where `max == min == ±inf` — all must match exactly | `err_e13_inf_mixes` | [x] |
| E14 | `rgb_to_hsv` | overflow of `delta = max - min` to `+inf` from finite inputs, e.g. `{FLT_MAX, -FLT_MAX, 0}` | `delta` rounds to `+inf`; `s = inf/FLT_MAX = inf`; division by `inf` yields `0`/signed `0` ⇒ exact bits must match | `err_e14_delta_overflow` | [x] |
| E15 | `rgb_to_hsv` | subnormal `delta` / subnormal channels, e.g. `{1e-45, 0, 0}` (smallest positive subnormal) | `s = delta/max = 1.0`; `h = (0-0)/delta = 0` — no flush-to-zero may be introduced | `err_e15_subnormals` | [x] |
| E16 | `rgb_to_hsv` | `h < 0` branch (line 33) taken: `r == max` and `g < b`, e.g. `{1.0, 0.0, 0.5}` | `h = (g-b)/delta < 0` ⇒ `h += 360` after `h *= 60` | `err_e16_h_negative_wrap` | [x] |
| E17 | `rgb_to_hsv` | `h` computes to exactly `-0.0` before the `h < 0` test, e.g. `r == max`, `g == b` with `g - b == -0.0` | `-0.0 < 0` is **false** ⇒ no `+360`; `dest[0]` must retain the `-0.0` bits, not `+0.0` and not `360.0` | `err_e17_h_negative_zero` | [x] |
| E18 | `rgb_to_hsv` | tie `r == max` **and** `g == max` (e.g. `{1,1,0}`): the `if/else if` chain means the `r` branch wins | `h = (g-b)/delta`, **not** the `g` branch — branch priority must be replicated | `err_e18_tie_r_g` | [x] |
| E19 | `rgb_to_hsv` | tie `g == max == b`, `r` smaller (e.g. `{0,1,1}`): `r == max` false, `g == max` true | `h = 2 + (b-r)/delta` — the `g` branch, not the `b`/else branch | `err_e19_tie_g_b` | [x] |
| E20 | `rgb_to_hsv` | `b` is the strict max ⇒ the final `else` branch (there is **no** `b == max` test; `else` is unconditional) | `h = 4 + (r-g)/delta`. Reached even when *no* channel equals `max` (possible with `NaN`), so `else` is a genuine fallthrough, not a `b == max` test | `err_e20_else_fallthrough` | [x] |
| E21 | `rgb_to_hsv` | `dest == src` (full aliasing). C reads all 3 of `src` into locals before any store, so aliasing is benign but observable | in-place conversion; result identical to the non-aliased case | `err_e21_alias_exact` | [x] |
| E22 | `rgb_to_hsv` | partial overlap, `dest = src + 1` and `dest = src - 1` | same: all reads precede all writes ⇒ same output as non-aliased | `err_e22_alias_offset` | [x] |
| E23 | `rgb_to_hsv` | `src == NULL` | **UB** — no null check exists in the C (`src[0]` is dereferenced unconditionally at line 4). Both libraries fault. Verified out-of-process: both die on the same fatal signal | `err_e23_null_src_faults_identically` | [x] |
| E24 | `rgb_to_hsv` | `dest == NULL` | **UB** — no null check; `dest[0]` is written unconditionally on both the early-return and the main path. Both fault with the same signal | `err_e24_null_dest_faults_identically` | [x] |
| E25 | `rgb_to_hsv` | buffer shorter than 3 `float`s (zero-length / 1-element / 2-element allocation) | **UB** — the C takes no length parameter and always touches indices 0..2. Out of contract; documented, not asserted as a value |  — (documented; no length parameter exists to test) | [x] |
| E26 | `rgb_to_hsv` | branch-priority ties constructed deliberately (`r==g`, `r==b`, `g==b` at `max`) over the exhaustive special-value grid and 120 000 random bit patterns — random inputs essentially never produce an exact tie, so the `if`/`else if` chain would otherwise be untested | whichever branch the C's `if r == max / else if g == max / else` chain selects | `err_branch_priority_ties_constructed` | [x] |

Rows E1–E22, E23–E24 and E26 all have executing differential tests. E25 is
recorded for completeness: there is no length argument in the API, so there is no
in-contract way to express it and no defined behaviour to compare.

## Divergence found and fixed

**E23/E24, debug profile.** The first run of the null-pointer rows FAILED: the
C `.so` died with `SIGSEGV` (11) but the Rust `.so` died with `SIGABRT` (6),
printing `panicked at src/lib.rs: null pointer dereference occurred`. Cause:
`rustc` instruments a plain `*ptr` dereference when `debug_assertions` are
enabled, turning the fault into a Rust panic. The C has no null check, so this
was a genuine behavioural difference in the translated library — visible to any
consumer of the debug artifact.

Fixed in the **Rust** (never the C) by performing the three loads and three
stores with `std::ptr::read_volatile` / `write_volatile`, which rustc does not
instrument. Both libraries now die with the identical signal in *both* build
profiles. As a bonus this pins the C's "all three reads happen before any
store" ordering that rows E21/E22 and `CONFIGS.md` C18/C19 depend on. Verified:
`err_e23_null_src_faults_identically` and `err_e24_null_dest_faults_identically` pass against the debug and the release cdylib.

## Mutation testing (evidence the rows are not vacuous)

12 deliberate mutations were injected into `translation/src/lib.rs`, rebuilt, and
run against the suite (`/tmp/mutate.py` pattern; see the transcript). Result:

- **CAUGHT (8):** `f32::max` instead of the C ternary (5 failures) · `h < 0` →
  `h <= 0` (12) · dropping the `max == 0.0` disjunct (4) · dropping the
  `delta == 0.0` disjunct (11) · `||` → `&&` (13) · `h += 360` → `359` (17) ·
  `min` ternary operand order (6) · `delta / max` → `delta * (1.0 / max)` (19).
- **Survived (4), each provably semantics-preserving,** so these are equivalent
  mutants rather than coverage gaps: `h *= 60.0` computed in `f64` (the product
  of a 24-bit and a 6-bit significand is exact in `f64`, so there is no double
  rounding) · `4.0 + x` → `2.0 + 2.0 + x` · the `r`/`g` branch swap (when
  `r == g == max` then `min == b`, so `delta == g - b` and both formulas
  evaluate to exactly `1.0`) · `2.0 + (b-r)/d` → `2.0 - (r-b)/d` (IEEE
  subtraction is exactly antisymmetric).

Every mutation that changes observable behaviour is detected.

## Gate status

- [x] Every row above with observable behaviour has a passing differential test
      asserting the **same** outcome (identical `f32` bit patterns, or identical
      fatal signal) from both the C `.so` and the Rust `.so`.
