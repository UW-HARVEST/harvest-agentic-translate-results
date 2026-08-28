# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from the C source. Exhaustive grep of `c_src/` for every
rejection mechanism:

```
grep -nE 'RETURN_ERROR|return *-1|return *NULL|assert|errno|goto|_MIN|_MAX' \
     src/lib.c include/lib.h   ->  no matches
grep -nE '\*'    src/lib.c include/lib.h  ->  only the float multiplications on
                                              line 9; NO pointer type anywhere
grep -nE 'enum'  src/lib.c include/lib.h  ->  no matches
```

**Findings — the API has no conventional error surface:**

- no error-return macro, no `-1` / `NULL` sentinel, no error enum, no `errno` use
- no `assert`, no explicit range check, no min/max constant
- no pointer parameter anywhere → **no null-pointer path exists**
- no `enum` parameter → **no out-of-range-enum path exists**
- `contrast_ratio` takes two `cb_rgb_255` **by value**; the members are
  `unsigned char`, so **every** one of the 2^24 bit patterns per argument is a
  valid, in-range input. There is no "oversized length" or "invalid value".
- the only `return` statements (`src/lib.c` lines 10, 22, 28) are unconditional
  success returns.

Consequently the entire "rejection" surface of this library is **degenerate
floating-point outcomes** — the C code deliberately does *not* guard the
division `High / Low` (no WCAG `+0.05` offset), so it can divide by zero. Those
are enumerated below, one row per distinct condition the C actually produces,
and each is asserted **bit-for-bit** (`to_bits()`), not merely "both are
non-finite" — the NaN payload/sign must match too.

`Lum((0,0,0)) == +0.0f` exactly (each channel takes the `x/12.92` branch giving
`+0.0`, and `0.2126f*0 + 0.7152f*0 + 0.0722f*0 == +0.0f`), and pure black is the
*only* input with zero luminance (any channel `n >= 1` contributes a strictly
positive term). So `Low == 0` exactly when an operand is pure black.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|---|----------|----------------------------------------------|-------------------|------|----|
| E1 | `contrast_ratio` | `A == (0,0,0)`, `B != (0,0,0)` → `LumA=0 < LumB`, `High<Low` **true** → swap → `Low = LumA = +0.0`, `High = LumB > 0` → `High/Low` | `+inf` (`0x7F800000`), no trap, no error code | `err_e1_black_a_nonblack_b` | [x] |
| E2 | `contrast_ratio` | `A != (0,0,0)`, `B == (0,0,0)` → `Low = LumB = +0.0`, `High = LumA > 0`; `High<Low` **false** (no swap) → `High/Low` | `+inf` (`0x7F800000`) | `err_e2_nonblack_a_black_b` | [x] |
| E3 | `contrast_ratio` | `A == (0,0,0)` **and** `B == (0,0,0)` → `High = Low = +0.0`; `0<0` false → `+0.0/+0.0` | NaN, **exact bit pattern** (x86 SSE `divss` indefinite QNaN) must match | `err_e3_both_black_nan_bits` | [x] |
| E4 | `contrast_ratio` | `A == B` and non-black → `High == Low`, `High<Low` false → `x/x` | exactly `1.0f` (`0x3F800000`) | `err_e4_identical_colors_exact_one` | [x] |
| E5 | `contrast_ratio` | value one step past the sRGB branch boundary: channel `10` (`10/255 = 0.0392 > 0.04045` **false** → `x/12.92`) vs channel `11` (`0.0431` **true** → `pow`). Off-by-one here silently changes the branch. | both branches taken identically by C and Rust | `err_e5_branch_boundary_10_11` | [x] |
| E6 | `contrast_ratio` | extremal in-range channel values `0` and `255` (the "zero and oversized length" analogue for this API — the full domain endpoints), all 8 corner colors | finite ratio, bit-identical | `err_e6_domain_endpoints` | [x] |
| E7 | `contrast_ratio` | ABI edge: the 3-byte by-value struct is read out of a larger buffer whose 4th byte is garbage (`0xFF`/`0xAA`); the padding byte in the register must be ignored by both | result independent of the garbage padding byte | `err_e7_struct_padding_garbage` | [x] |
| E8 | `contrast_ratio` | every remaining bit pattern of the argument pair is in-range by construction; verified by exhaustive sweep of all 2^24 colors (rows C65/C66) that no input produces a divergence or a spurious non-finite value | no rejection path; only E1–E3 are non-finite | `exhaustive_all_colors_vs_white`, `exhaustive_all_colors_vs_black` | [x] |

**Generic boundaries required by Phase C, and why they are or are not applicable**

| generic boundary | applicable? | handling |
|---|---|---|
| null pointers | **no** — API takes no pointers (both args by value) | n/a, documented above |
| zero length / oversized length | **no** — API takes no length/buffer | closest analogue = domain endpoints `0`/`255`, covered by E6 |
| value one past valid range | **n/a for the type** (all 256 `unsigned char` values valid) but the *internal* threshold has a one-past boundary → covered by E5; and out-of-range struct **padding** covered by E7 |
| out-of-range enum across FFI | **no** — API declares no enum | n/a, documented above |
