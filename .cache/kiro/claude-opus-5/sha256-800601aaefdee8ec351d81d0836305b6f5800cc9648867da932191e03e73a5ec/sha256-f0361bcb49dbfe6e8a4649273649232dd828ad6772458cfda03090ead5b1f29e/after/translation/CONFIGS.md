# CONFIGS.md — Configuration / valid-input surface table (Phase A)

## Mechanical derivation of the axes

Public API surface (the entire header, `c_src/include/lib.h`, 1 line):

```c
void rgb_to_hsv(float *dest, const float *src);
```

There is exactly **one** public entry point, and it is simultaneously the
lowest-level and the highest-level one — there is no convenience wrapper layered
over a lower-level primitive, so "test the low-level entry points too" collapses
onto this single function.

Runtime option / mode / flag axes: **none.** Greps for preprocessor and
conditional constructs over the whole C source return:

```
grep -nE "^#"       -> src/lib.c:1: #include "lib.h"      (no #ifdef, no #define, no build-time modes)
grep -nE "if|switch|\?" src/lib.c:
  13:  min = (((min) < (g)) ? (min) : (g));
  14:  min = (((min) < (b)) ? (min) : (b));
  15:  max = (((max) > (g)) ? (max) : (g));
  16:  max = (((max) > (b)) ? (max) : (b));
  19:  if (delta == 0 || max == 0) { ... return; }
  26:  if (r == max)
  28:  else if (g == max)
  33:  if (h < 0)
```

So all nine branches the C takes are driven purely by the **values** of
`src[0..=2]`. The configuration axes are therefore the value-shape axes:

- **A1 — which channel holds `max`** (the line 26/28/else three-way split):
  `r`, `g`, `b`, and the tie cases (`r==g`, `g==b`, `r==b`, `r==g==b`), because
  the chain `if (r==max) / else if (g==max) / else` resolves ties by *position*,
  not by value.
- **A2 — the ternary min/max chain (lines 13–16)**: which operand each `<`/`>`
  keeps. Distinguishes ordinary ordering from the cases where a comparison is
  **false because an operand is NaN**, which makes the ternaries keep the
  opposite operand from what `fminf`/`f32::min` would.
- **A3 — the `delta == 0 || max == 0` early return (line 19)**: achromatic
  (`r==g==b`), `max` exactly `+0.0`, `max` exactly `-0.0`, and the distinct case
  `max == 0` while `delta != 0` (negative inputs) which short-circuits a
  *chromatic* input onto the achromatic path.
- **A4 — the `h < 0` wrap (line 33)**: only reachable on the `r == max` branch,
  requires `g < b`. Also the `h` exactly `0` / exactly `-0.0` boundary.
- **A5 — numeric class of the components**: normalized `[0,1]`, `0..255`,
  negative, mixed sign, `±0.0`, subnormal, `f32::MAX`-scale (so `max-min`
  overflows to `+inf`), `±inf`, NaN, and arbitrary raw bit patterns.
- **A6 — buffer/pointer shape** (the only non-value axis the signature permits):
  distinct `dest`/`src`, `dest == src`, partially overlapping, padded
  (`len > 3`), and misaligned. Overlap/alignment rows live in `ERRORS.md`
  (E4–E8); the distinct-buffer shape is the baseline for every row here.

Rows below are the cross-product of A1–A5, pruned to the combinations the C
actually distinguishes. Every row is driven with **many randomized inputs**
(fixed seed `0x5EED_1234`, SplitMix64) inside the stated class, not one hand-picked
value, and compared bit-for-bit between the C `.so` and the Rust `.so`.

## The table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `rgb_to_hsv` | achromatic, `r == g == b`, random value in `(0,1]` → `delta == 0`, early return, `h=s=0`, `v=max` | [x] |
| C2 | `rgb_to_hsv` | achromatic, `r == g == b`, random value in `(1, 1e30]` → `delta == 0` early return at large magnitude | [x] |
| C3 | `rgb_to_hsv` | all channels exactly `+0.0` → both `delta == 0` **and** `max == 0` true | [x] |
| C4 | `rgb_to_hsv` | `r` strict max, `g > b`, all in `[0,1]` → `r==max` branch, `h` in `(0,60)`, no wrap | [x] |
| C5 | `rgb_to_hsv` | `r` strict max, `g < b`, all in `[0,1]` → `r==max` branch, `h < 0`, wrap `+= 360` taken | [x] |
| C6 | `rgb_to_hsv` | `r` strict max, `g == b` exactly → `r==max` branch with `h` computed as `±0.0` before scaling, wrap boundary not taken | [x] |
| C7 | `rgb_to_hsv` | `g` strict max, random `r`,`b` in `[0,1]` → `2 + (b-r)/delta` branch | [x] |
| C8 | `rgb_to_hsv` | `b` strict max, random `r`,`g` in `[0,1]` → `else` / `4 + (r-g)/delta` branch | [x] |
| C9 | `rgb_to_hsv` | tie `r == g > b` → chain resolves to the **`r==max`** branch (position-based tie-break) | [x] |
| C10 | `rgb_to_hsv` | tie `g == b > r` → chain resolves to the **`g==max`** branch | [x] |
| C11 | `rgb_to_hsv` | tie `r == b > g` → chain resolves to the **`r==max`** branch | [x] |
| C12 | `rgb_to_hsv` | uniformly random triple in `[0,1]` (typical normalized consumer input; hits A1/A4 at random) | [x] |
| C13 | `rgb_to_hsv` | uniformly random triple in `[0,255]` (8-bit-style unnormalized input) | [x] |
| C14 | `rgb_to_hsv` | mixed sign: `max > 0` with at least one negative channel → `delta > max`, so `s > 1` (out of nominal range, unchecked) | [x] |
| C15 | `rgb_to_hsv` | all channels negative → `max < 0`, `delta != 0`, `max != 0` → full chromatic path with negative `s` and negative `v` | [x] |
| C16 | `rgb_to_hsv` | `max` lands on exactly `0` while `delta != 0` (e.g. one channel `+0.0`, others negative) → `max == 0` short-circuits a chromatic input to the achromatic early return | [x] |
| C17 | `rgb_to_hsv` | signed-zero shapes: every combination of `+0.0`/`-0.0` across the 3 channels (8 cases, exhaustive) → probes `max == 0` with `max == -0.0` and the sign of the stored `v` | [x] |
| C18 | `rgb_to_hsv` | exactly one NaN, in `r` (position 0) → ternary chain keeps non-NaN operands per the literal C order | [x] |
| C19 | `rgb_to_hsv` | exactly one NaN, in `g` (position 1) | [x] |
| C20 | `rgb_to_hsv` | exactly one NaN, in `b` (position 2) | [x] |
| C21 | `rgb_to_hsv` | two or three NaNs — all 4 remaining NaN masks, with random finite values in the other slots and random NaN payloads/signs | [x] |
| C22 | `rgb_to_hsv` | `+inf` in one random channel, finite elsewhere → `max == inf`, `delta == inf`, ratio `finite/inf == 0` | [x] |
| C23 | `rgb_to_hsv` | `-inf` in one random channel, finite elsewhere → `min == -inf`, `delta == inf` | [x] |
| C24 | `rgb_to_hsv` | both `+inf` and `-inf` present → `delta == inf` and `inf/inf == NaN` propagating into `h` | [x] |
| C25 | `rgb_to_hsv` | subnormal components (random denormal bit patterns) → `delta` subnormal or `0`; `delta/max` may be a normal ratio | [x] |
| C26 | `rgb_to_hsv` | `f32::MAX`-scale opposite-sign components → `max - min` **overflows to `+inf`** while `max` is finite → `s == inf` | [x] |
| C27 | `rgb_to_hsv` | near-identical channels: `min = nextafter(max, -inf)` → smallest possible non-zero `delta`, `(g-b)/delta` overflow-prone | [x] |
| C28 | `rgb_to_hsv` | one-step-past-nominal-range boundaries: components drawn from `{-0.0, +0.0, nextafter(0,-1), nextafter(0,+1), 1.0, nextafter(1,-1), nextafter(1,+1), f32::MIN_POSITIVE, f32::MAX, f32::MIN}` — exhaustive 10³ = 1000 triples | [x] |
| C29 | `rgb_to_hsv` | fully random raw 32-bit patterns per channel (mixes NaNs, infinities, denormals, huge and tiny magnitudes) — the broadest fuzz row | [x] |
| C30 | `rgb_to_hsv` | random triples restricted to a tiny value alphabet `{-2,-1,-0.0,0.0,0.5,1,2}` so ties and equalities occur with high probability — stresses the A1 tie-break and A3 branches jointly | [x] |
