# CONFIGS.md — Configuration / valid-input surface table (Phase A, gate for Phase B)

Derived mechanically from the branches the C code actually takes.

## Public entry points (full set, from `c_src/include/lib.h`)

| entry point | signature | notes |
|-------------|-----------|-------|
| `rgb_to_hsv` | `void rgb_to_hsv(float *dest, const float *src)` | the *only* public symbol; it is simultaneously the lowest-level and the highest-level entry point (no convenience wrapper, no init/teardown, no context object, no options struct) |

## Runtime options / modes

There are **no** runtime options, flags, modes, contexts or globals: no
option-setter functions, no `static` state, no environment lookups, and no
`#if`/`#ifdef` in the source. Behaviour is a pure function of the 3 input floats
and the two pointers. The "configuration axes" are therefore exactly the input
*shapes* that the code branches on.

## Axes the C code branches on (`c_src/src/lib.c`)

| axis | source lines | distinct states |
|------|--------------|-----------------|
| A. min-selection ternaries | 13–14 (`min < g`, `min < b`) | which of r/g/b is the minimum, incl. ties and NaN-false outcomes |
| B. max-selection ternaries | 15–16 (`max > g`, `max > b`) | which of r/g/b is the maximum, incl. ties (first-wins because the test is strict `>`) and NaN-false outcomes |
| C. degenerate guard | 19 (`delta == 0 \|\| max == 0`) | taken / not taken (and *which* disjunct triggers) |
| D. hue sector | 26/28/else (`r == max`, `g == max`, else) | 3 states, with tie-breaking priority r → g → b |
| E. negative-hue fixup | 33 (`h < 0`) | taken / not taken (`-0.0` does **not** take it) |
| F. float value class | all arithmetic | normal, ±0.0, subnormal, ±inf, NaN, FLT_MAX/FLT_MIN extremes, overflow/underflow of `max-min` |
| G. pointer relationship | 3–4, 21–23 & 35–37 | disjoint buffers / `dest == src` (in-place) / partially overlapping / unaligned-by-1-float views |

## Table — one row per meaningful combination

Every row is exercised through **both** `.so` exports with many randomized
inputs (fixed seed, `SplitMix64`) plus the listed edge constants, and the
3 output floats are compared **bit-for-bit**.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `rgb_to_hsv` | disjoint buffers; r strictly max, g > b (sector D=r, E not taken, h ∈ (0,60)) — randomized in (0,1] | [x] |
| 2  | `rgb_to_hsv` | disjoint buffers; r strictly max, b > g (sector D=r, **E taken**, h ∈ (300,360)) — randomized | [x] |
| 3  | `rgb_to_hsv` | disjoint buffers; r max, g == b exactly (h == `+0.0`, E not taken) — randomized magnitude | [x] |
| 4  | `rgb_to_hsv` | disjoint buffers; g strictly max (sector D=g, h ∈ (60,180)) — randomized | [x] |
| 5  | `rgb_to_hsv` | disjoint buffers; b strictly max (sector D=b, h ∈ (180,300)) — randomized | [x] |
| 6  | `rgb_to_hsv` | tie r == g > b → sector priority must pick **r** (line 26 wins) — randomized | [x] |
| 7  | `rgb_to_hsv` | tie g == b > r → max ternaries keep g (strict `>`), sector picks **g** — randomized | [x] |
| 8  | `rgb_to_hsv` | tie r == b > g → sector picks **r** — randomized | [x] |
| 9  | `rgb_to_hsv` | all three equal, positive → guard C via `delta == 0` (early return, s stays 0) — randomized | [x] |
| 10 | `rgb_to_hsv` | all three equal to `0.0` → guard C via **both** disjuncts | [x] |
| 11 | `rgb_to_hsv` | max is exactly `0.0` while `delta != 0` (mixed zero/negative) → guard C via `max == 0`, division skipped — randomized negatives | [x] |
| 12 | `rgb_to_hsv` | all components negative, distinct → no guard, `s < 0`, all 3 sectors covered — randomized in [-1,0) | [x] |
| 13 | `rgb_to_hsv` | mixed-sign components (min < 0 < max) → `delta > max`, `s > 1` — randomized in [-1,1] | [x] |
| 14 | `rgb_to_hsv` | signed zeros: all 8 combinations of `±0.0` in (r,g,b) — pins `v`'s zero sign and the `delta == 0` disjunct | [x] |
| 15 | `rgb_to_hsv` | subnormal inputs (`1e-45`, `FLT_MIN/2`, …), incl. subnormal `delta` and subnormal `max` | [x] |
| 16 | `rgb_to_hsv` | extreme magnitudes: `FLT_MAX`, `-FLT_MAX`, `FLT_MIN`, overflow of `max - min` to `+inf` | [x] |
| 17 | `rgb_to_hsv` | non-finite: every placement of `+inf` / `-inf` over the 3 slots (3^3 combos of {-inf, +inf, 1.0}) | [x] |
| 18 | `rgb_to_hsv` | NaN: quiet NaN in each slot, several NaN payloads/signs, plus all-NaN (checks branch-false semantics and payload propagation) | [x] |
| 19 | `rgb_to_hsv` | 8-bit-quantised sRGB shape: components from `{0,1,…,255}/255` (the library's intended real input shape), exhaustive over a randomized sample | [x] |
| 20 | `rgb_to_hsv` | components ≥ 1 / large positive normals (h denominators large, s < 1) — randomized in [1, 1e6] | [x] |
| 21 | `rgb_to_hsv` | fully random bit patterns reinterpreted as `f32` (any class, incl. NaN/inf/subnormal) — 20 000 randomized cases | [x] |
| 22 | `rgb_to_hsv` | pointer relationship: `dest == src` (in-place) over randomized inputs from rows 1–5 shapes | [x] |
| 23 | `rgb_to_hsv` | pointer relationship: partially overlapping (`dest = buf`, `src = buf+1`) and (`dest = buf+1`, `src = buf`), randomized | [x] |
| 24 | `rgb_to_hsv` | pointer relationship: disjoint but **unaligned to 16 B** (`buf+1` … `buf+3` offsets) → no SSE-alignment assumption difference | [x] |
| 25 | `rgb_to_hsv` | output-window bound: `dest` inside a canary-guarded buffer → exactly 3 floats written, neighbours untouched | [x] |
| 26 | `rgb_to_hsv` | repeated / stateless invocation: 1 call, then 1 000 randomized calls reusing the same buffers, then re-running the first input → identical result (no hidden state) | [x] |
| 27 | `rgb_to_hsv` | batch/stride use: 4096-pixel array converted element-by-element through both `.so`s (composed pipeline over a whole image buffer) | [x] |
| 28 | `rgb_to_hsv` | 4 threads calling both `.so`s concurrently (2 000 randomized inputs each) → confirms the absence of shared state in either implementation | [x] |

Each row maps 1:1 to a test named `rowNN_*` in `tests/configs.rs`
(`row01_r_max_g_gt_b` … `row28_concurrent`). All 28 pass.

## Build configurations

`Cargo.toml` declares no `[features]`; `CMakeLists.txt` declares no options and
the C source has no `#ifdef`. Hence the cross-product of build configurations is
a single cell, and rows 1–28 are run under every spelling of it, in both cargo
profiles, and against three different C compilations:

| # | configuration | command | status |
|---|---------------|---------|--------|
| 1 | default (== no features == all features), dev profile | `cargo test --no-default-features` | ✅ 53 tests |
| 2 | same, release profile | `cargo test --no-default-features --release` | ✅ 53 tests |
| 3 | default spelling | `cargo test` | ✅ 53 tests |
| 4 | all-features spelling | `cargo test --all-features` | ✅ 53 tests |
| 5 | vs. C compiled `-O2` | `HARVEST_C_SO=<so> cargo test --no-default-features` | ✅ 53 tests |
| 6 | vs. C compiled `-O3 -march=native` | `HARVEST_C_SO=<so> cargo test --no-default-features` | ✅ 53 tests |

`./verify.sh` runs all of the above plus the `nm -D` symbol diff.
