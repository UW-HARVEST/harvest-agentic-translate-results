# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c` (59 lines, the only source file).

## Mechanical grep for rejection constructs

```sh
$ grep -nE 'RETURN_ERROR|return -1|return NULL|return [0-9-]|assert|errno|goto|exit\(|abort\(' c_src/src/lib.c
(no matches)

$ grep -nE 'if *\(|switch|else|\?|#if' c_src/src/lib.c
12:    if (s == 0) {
24:    switch (i) {
```

## Finding

`hsv_to_rgb` returns `void`. It contains:

* **no** error-return macro or statement,
* **no** error enum, status code, or sentinel value,
* **no** `assert`,
* **no** explicit range check, clamp, or min/max constant,
* **no** null-pointer check,
* **no** `errno` use.

The function has exactly **two control-flow constructs**: the `s == 0` guard
(line 12, an early `return;`) and the `switch (i)` sector dispatch (line 24)
whose `default:` arm absorbs every index outside `0..=4`.

Therefore the *explicit* error surface is EMPTY. Every row below is a
**rejection-adjacent / boundary condition** that the C code reaches implicitly.
Each row has a differential test in `tests/differential.rs` (Phase C).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `hsv_to_rgb` | `src[1] == 0.0f` (achromatic guard, line 12) — the ONLY early `return` | writes `{v, v, v}` to `dest`, leaves `dest[0..3]` otherwise untouched, returns void |
| E2 | `hsv_to_rgb` | `src[1] == -0.0f` (negative zero saturation; `-0.0f == 0` is true in C) | same as E1: `{v, v, v}` |
| E3 | `hsv_to_rgb` | sector index `i < 0` (e.g. `h < 0`, so `floorf(h/60) < 0`) → `default:` arm | `{v, p, q}`, no clamping, no rejection |
| E4 | `hsv_to_rgb` | sector index `i >= 5` (e.g. `h >= 300`, incl. `h >= 360` which is out of the documented hue range) → `default:` arm | `{v, p, q}`, no wrap-around, no rejection |
| E5 | `hsv_to_rgb` | `h = NaN` → `floorf(NaN) = NaN` → `(int)NaN` is **UB**; x86-64 `cvttss2si` yields the integer-indefinite value `INT_MIN` → `default:` arm | `{v, p, q}` with `f = NaN`, so `q`/`t` are NaN (or NaN×0 → NaN) |
| E6 | `hsv_to_rgb` | `h = +INFINITY` → `(int)+inf` is UB → `INT_MIN` → `default:` arm | `{v, p, q}` with `f = NaN` (`inf - (-2^31)` = inf; `inf - inf` … see test) |
| E7 | `hsv_to_rgb` | `h = -INFINITY` → `(int)-inf` is UB → `INT_MIN` → `default:` arm | `{v, p, q}` |
| E8 | `hsv_to_rgb` | `\|h/60\| >= 2^31` finite (e.g. `h = 1e30f`) → float-to-int conversion out of `int` range, UB → `INT_MIN` → `default:` arm | `{v, p, q}` |
| E9 | `hsv_to_rgb` | `h/60` exactly `2147483648.0f` (`2^31`, first value past `INT_MAX`) — one step past the valid conversion range | `INT_MIN` → `default:` arm |
| E10 | `hsv_to_rgb` | `h/60` exactly `-2147483648.0f` (`-2^31`, still IN range for `int`) — boundary that must NOT take the indefinite path | `i = INT_MIN` (a *representable* result) → `default:` arm |
| E11 | `hsv_to_rgb` | `s = NaN` → `s == 0` is false → chromatic path with NaN saturation | `p = v*(1-NaN) = NaN`, etc.; no rejection |
| E12 | `hsv_to_rgb` | `s = ±INFINITY` (far outside the documented `[0,1]` range, no clamp) | arithmetic propagates inf/NaN; no rejection |
| E13 | `hsv_to_rgb` | `s < 0` or `s > 1` (outside documented range, no clamp, no rejection) | unclamped arithmetic result |
| E14 | `hsv_to_rgb` | `v = NaN` / `±INFINITY` / negative / `> 1` (outside documented range, no clamp) | unclamped arithmetic result |
| E15 | `hsv_to_rgb` | subnormal `h`, `s`, `v` (smallest denormals) — boundary of the float range | full-precision denormal arithmetic, no flush-to-zero difference |
| E16 | `hsv_to_rgb` | `dest == src` (full aliasing / in-place conversion) — C reads all three inputs into locals *before* the first store, so this is well defined in C | same output as the non-aliased call |
| E17 | `hsv_to_rgb` | `dest` partially overlaps `src` (`dest = src+1`, `dest = src+2`) | same output as the non-aliased call (reads precede writes) |
| E18 | `hsv_to_rgb` | `src == NULL` — **no null check in C**, dereferenced at line 7 | UB: `SIGSEGV` on Linux/x86-64 |
| E19 | `hsv_to_rgb` | `dest == NULL` (with valid non-zero `src[1]`) — no null check, stored to at line 51 | UB: `SIGSEGV` |
| E20 | `hsv_to_rgb` | `dest == NULL` and `src[1] == 0.0f` (achromatic path stores at line 13) | UB: `SIGSEGV` |
| E21 | `hsv_to_rgb` | *"out-of-range enum value"* class: the C API declares **no enum, flag, or mode parameter**, so the analogous input is an arbitrary 32-bit pattern reinterpreted as `float`. Every one of the 2^32 bit patterns is a legal `float` argument (incl. every NaN payload, both zeros, both infinities). | no rejection path exists; result is whatever the arithmetic produces, bit-for-bit |

Rows E18–E20 are the null-pointer boundary. Because the C code has no null
check, the only observable behaviour is a fault; the tests compare the
*termination signal* of both libraries in forked child processes.

## Status

All 21 rows have a passing differential test — see `PHASE_C_RESULTS` at the
bottom of this file.

### PHASE_C_RESULTS

| row | test name | status |
|-----|-----------|--------|
| E1  | `e1_e2_zero_and_negative_zero_saturation` | ✅ pass |
| E2  | `e1_e2_zero_and_negative_zero_saturation` | ✅ pass |
| E3  | `e3_negative_sector_index` | ✅ pass |
| E4  | `e4_sector_index_ge_5` | ✅ pass |
| E5  | `e5_hue_nan` | ✅ pass |
| E6  | `e6_e7_hue_infinities` | ✅ pass |
| E7  | `e6_e7_hue_infinities` | ✅ pass |
| E8  | `e8_hue_huge_finite` | ✅ pass |
| E9  | `e9_e10_int_conversion_boundaries` | ✅ pass |
| E10 | `e9_e10_int_conversion_boundaries` | ✅ pass |
| E11 | `e11_saturation_nan` | ✅ pass |
| E12 | `e12_saturation_infinities` | ✅ pass |
| E13 | `e13_saturation_out_of_range` | ✅ pass |
| E14 | `e14_value_out_of_range` | ✅ pass |
| E15 | `e15_subnormals` | ✅ pass |
| E16 | `e16_full_aliasing_in_place` | ✅ pass |
| E17 | `e17_partial_overlap` | ✅ pass |
| E18 | `e18_e19_e20_null_pointers` (forked children) | ✅ pass |
| E19 | `e18_e19_e20_null_pointers` (forked children) | ✅ pass |
| E20 | `e18_e19_e20_null_pointers` (forked children) | ✅ pass |
| E21 | `e21_arbitrary_bit_patterns` | ✅ pass |
