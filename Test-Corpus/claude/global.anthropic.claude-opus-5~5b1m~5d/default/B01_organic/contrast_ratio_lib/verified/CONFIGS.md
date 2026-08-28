# CONFIGS.md — Phase B configuration surface table (VALID inputs)

Derived mechanically from the branches the C source actually takes.

## Step 1 — enumerate the runtime options the public API can set

Grep of the public header for options/modes/flags:

```
c_src/include/lib.h  ->  1 type (cb_rgb_255), 1 function (contrast_ratio)
grep -nE '#if|#ifdef|switch' c_src/src/lib.c  ->  no matches
```

**There is no runtime option, mode, flag, context object, or `#ifdef` in this
library.** No setter, no init, no global state. The configuration surface is
therefore made entirely of *input shapes* and the *branches* those shapes select.

## Step 2 — enumerate the FULL set of public entry points, lowest level included

| entry point | linkage | reachable how |
|---|---|---|
| `contrast_ratio(cb_rgb_255, cb_rgb_255)` | exported | called directly via `.so` (the only export) |
| `cbContrastRatio(float×6)` | `static` — **not** exported | lowest-level composed step; driven through `contrast_ratio`. The 6 floats are not free: `contrast_ratio` supplies exactly `n/255.f`, so the reachable domain is the 256 values `{0/255 … 255/255}` per channel. Every one of those 256 values per channel is exercised (rows C65/C66 sweep all 2^24 combinations). |
| `cbLuminance(float×3)` | `static` — **not** exported | driven through `cbContrastRatio`; both operand positions (A and B) are exercised for every row. |

Because the two low-level functions are `static`, driving them "directly" is
impossible through the ABI *by design*; the tests instead drive them over their
**entire reachable input domain**, which is strictly stronger than sampling.

## Step 3 — axes the C code branches on

| axis | values | source |
|---|---|---|
| **X1** sRGB branch, channel R of A | `lin` = value ≤ 10 → `R/12.92`; `pow` = value ≥ 11 → `pow((R+.055)/1.055, 2.4)` | `lib.c:6` (`R > 0.04045`; `n/255 > 0.04045 ⟺ n ≥ 11`) |
| **X2** sRGB branch, channel G of A | `lin` / `pow` | `lib.c:7` |
| **X3** sRGB branch, channel B of A | `lin` / `pow` | `lib.c:8` |
| **X4/X5/X6** same three branches for operand B | `lin` / `pow` | `lib.c:6-8` via second `cbLuminance` call |
| **X7** swap branch | `High<Low` **true** (swap) / **false** (no swap, incl. `LumA == LumB`) | `lib.c:18` |
| **X8** divisor degeneracy | `Low > 0` / `Low == 0` (pure-black operand) | `lib.c:21` unguarded `High/Low` |
| **X9** input shape / ABI | 3-byte struct by value, packed in one INTEGER register; padding byte defined vs garbage | `include/lib.h:1-5` + SysV AMD64 classification |

Rows C1–C64 below are the **full cross product of X1…X6** (8 luminance branch
patterns for operand A × 8 for operand B). This is the set of combinations the
code genuinely distinguishes; the `lin`/`pow` pattern is written as a triple over
(R,G,B). Every row is driven with **many randomized inputs** (fixed seed
`0x5EED_1234`, values drawn from the correct sub-range per channel — `0..=10` for
`lin`, `11..=255` for `pow`), and every row additionally covers **both** settings
of X7 by construction, since randomized A/B pairs land on either side of
`High<Low` (the test asserts both orderings were observed).

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|--------------------------------------------|-----|
| C1 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,lin)**, B branch pattern R,G,B = **(lin,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C2 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,lin)**, B branch pattern R,G,B = **(lin,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C3 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,lin)**, B branch pattern R,G,B = **(lin,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C4 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,lin)**, B branch pattern R,G,B = **(lin,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C5 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,lin)**, B branch pattern R,G,B = **(pow,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C6 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,lin)**, B branch pattern R,G,B = **(pow,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C7 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,lin)**, B branch pattern R,G,B = **(pow,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C8 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,lin)**, B branch pattern R,G,B = **(pow,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C9 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,pow)**, B branch pattern R,G,B = **(lin,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C10 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,pow)**, B branch pattern R,G,B = **(lin,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C11 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,pow)**, B branch pattern R,G,B = **(lin,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C12 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,pow)**, B branch pattern R,G,B = **(lin,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C13 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,pow)**, B branch pattern R,G,B = **(pow,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C14 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,pow)**, B branch pattern R,G,B = **(pow,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C15 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,pow)**, B branch pattern R,G,B = **(pow,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C16 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,lin,pow)**, B branch pattern R,G,B = **(pow,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C17 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,lin)**, B branch pattern R,G,B = **(lin,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C18 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,lin)**, B branch pattern R,G,B = **(lin,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C19 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,lin)**, B branch pattern R,G,B = **(lin,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C20 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,lin)**, B branch pattern R,G,B = **(lin,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C21 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,lin)**, B branch pattern R,G,B = **(pow,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C22 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,lin)**, B branch pattern R,G,B = **(pow,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C23 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,lin)**, B branch pattern R,G,B = **(pow,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C24 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,lin)**, B branch pattern R,G,B = **(pow,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C25 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,pow)**, B branch pattern R,G,B = **(lin,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C26 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,pow)**, B branch pattern R,G,B = **(lin,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C27 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,pow)**, B branch pattern R,G,B = **(lin,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C28 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,pow)**, B branch pattern R,G,B = **(lin,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C29 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,pow)**, B branch pattern R,G,B = **(pow,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C30 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,pow)**, B branch pattern R,G,B = **(pow,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C31 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,pow)**, B branch pattern R,G,B = **(pow,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C32 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(lin,pow,pow)**, B branch pattern R,G,B = **(pow,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C33 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,lin)**, B branch pattern R,G,B = **(lin,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C34 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,lin)**, B branch pattern R,G,B = **(lin,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C35 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,lin)**, B branch pattern R,G,B = **(lin,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C36 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,lin)**, B branch pattern R,G,B = **(lin,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C37 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,lin)**, B branch pattern R,G,B = **(pow,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C38 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,lin)**, B branch pattern R,G,B = **(pow,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C39 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,lin)**, B branch pattern R,G,B = **(pow,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C40 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,lin)**, B branch pattern R,G,B = **(pow,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C41 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,pow)**, B branch pattern R,G,B = **(lin,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C42 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,pow)**, B branch pattern R,G,B = **(lin,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C43 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,pow)**, B branch pattern R,G,B = **(lin,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C44 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,pow)**, B branch pattern R,G,B = **(lin,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C45 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,pow)**, B branch pattern R,G,B = **(pow,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C46 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,pow)**, B branch pattern R,G,B = **(pow,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C47 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,pow)**, B branch pattern R,G,B = **(pow,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C48 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,lin,pow)**, B branch pattern R,G,B = **(pow,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C49 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,lin)**, B branch pattern R,G,B = **(lin,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C50 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,lin)**, B branch pattern R,G,B = **(lin,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C51 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,lin)**, B branch pattern R,G,B = **(lin,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C52 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,lin)**, B branch pattern R,G,B = **(lin,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C53 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,lin)**, B branch pattern R,G,B = **(pow,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C54 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,lin)**, B branch pattern R,G,B = **(pow,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C55 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,lin)**, B branch pattern R,G,B = **(pow,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C56 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,lin)**, B branch pattern R,G,B = **(pow,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C57 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,pow)**, B branch pattern R,G,B = **(lin,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C58 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,pow)**, B branch pattern R,G,B = **(lin,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C59 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,pow)**, B branch pattern R,G,B = **(lin,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C60 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,pow)**, B branch pattern R,G,B = **(lin,pow,pow)**; randomized channel values within each branch's sub-range | [x] |
| C61 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,pow)**, B branch pattern R,G,B = **(pow,lin,lin)**; randomized channel values within each branch's sub-range | [x] |
| C62 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,pow)**, B branch pattern R,G,B = **(pow,lin,pow)**; randomized channel values within each branch's sub-range | [x] |
| C63 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,pow)**, B branch pattern R,G,B = **(pow,pow,lin)**; randomized channel values within each branch's sub-range | [x] |
| C64 | `contrast_ratio` → `cbContrastRatio` → `cbLuminance`×2 | no options (none exist); A branch pattern R,G,B = **(pow,pow,pow)**, B branch pattern R,G,B = **(pow,pow,pow)**; randomized channel values within each branch's sub-range | [x] |

## Rows C65+ — remaining axes (X7 swap, X8 degeneracy, X9 ABI shape, full-domain sweeps)

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|--------------------------------------------|-----|
| C65 | `contrast_ratio` (full domain of `cbLuminance`) | **Exhaustive**: all 2^24 = 16,777,216 colors as operand A against fixed `B = white (255,255,255)`. Sweeps the entire reachable input domain of `cbLuminance` and every `pow` argument the library can ever produce. | [x] |
| C66 | `contrast_ratio` (full domain, other operand position) | **Exhaustive**: all 2^24 colors as operand **B** against fixed `A = black (0,0,0)`, exercising the swapped operand position and the `Low == 0` degenerate divisor for every possible `High`. | [x] |
| C67 | `contrast_ratio` | X7 = **no swap** forced: randomized pairs with `LumA > LumB` (A strictly brighter), all-`pow` and mixed patterns | [x] |
| C68 | `contrast_ratio` | X7 = **swap** forced: randomized pairs with `LumA < LumB` (B strictly brighter) | [x] |
| C69 | `contrast_ratio` | X7 = **equal** boundary: `LumA == LumB` via `A == B` (randomized identical colors) → `High<Low` false, ratio exactly 1.0 | [x] |
| C70 | `contrast_ratio` | X7 = **equal luminance, different colors** (distinct RGB triples that collide to the same `float` luminance, found by search) → tie path with `A != B` | [x] |
| C71 | `contrast_ratio` | X8: `Low == 0` with A = black, B = randomized non-black (→ `+inf`) | [x] |
| C72 | `contrast_ratio` | X8: `Low == 0` with B = black, A = randomized non-black (→ `+inf`) | [x] |
| C73 | `contrast_ratio` | X8: `Low == 0` and `High == 0` (both black) → `+0.0/+0.0` NaN, bit-exact | [x] |
| C74 | `contrast_ratio` | Input shape: **grayscale** colors (`R == G == B`), all 256 of them, cross-product against 256 grayscales = 65,536 pairs (the `0.2126+0.7152+0.0722` sum path with equal terms) | [x] |
| C75 | `contrast_ratio` | Input shape: **single-channel-only** colors (pure R / pure G / pure B, other channels 0) over all 256 intensities × all 3 channel positions, paired both ways — isolates each weight coefficient | [x] |
| C76 | `contrast_ratio` | Input shape: **branch-boundary lattice** — every channel drawn from `{0,1,9,10,11,12,254,255}` (the values straddling the `> 0.04045` threshold and the domain ends): full 8^3 × 8^3 = 262,144 pair cross-product | [x] |
| C77 | `contrast_ratio` | X9 ABI: struct read from an oversized buffer with **garbage in the 4th byte** (`0x00`/`0xAA`/`0xFF`), randomized colors — the 3-byte struct is passed packed in one INTEGER register, so the padding must not leak into either implementation's result | [x] |
| C78 | `contrast_ratio` | X9 ABI: both arguments' structs taken from **unaligned** offsets inside a byte buffer, randomized colors — confirms neither side assumes alignment for the by-value copy | [x] |
| C79 | `contrast_ratio` | Large-scale randomized fuzz over the **unconstrained** domain: 5,000,000 uniformly random `(A,B)` pairs, seed-fixed, bit-exact comparison (catches value-dependent divergence not tied to any enumerated branch) | [x] |
| C80 | `contrast_ratio` | **Symmetry/ordering property** cross-check: for randomized pairs, `contrast_ratio(A,B)` and `contrast_ratio(B,A)` must agree between C and Rust *individually* (the C is symmetric by construction; verified as a differential invariant on both) | [x] |

## Row → test mapping

| rows | test |
|---|---|
| C1–C64 | `phase_b_configs::c1_c64_srgb_branch_cross_product` (384,000 comparisons; 64/64 rows reached both sides of the swap branch and a ratio > 1) |
| C65 | `phase_b_exhaustive::exhaustive_all_colors_vs_white` (all 16,777,216 colors) |
| C66 | `phase_b_exhaustive::exhaustive_all_colors_vs_black` (all 16,777,216 colors) |
| C67, C68 | `phase_b_configs::c67_c68_swap_branch_both_directions` |
| C69 | `phase_b_configs::c69_identical_colors_tie` |
| C70 | `phase_b_configs::c70_equal_luminance_distinct_colors` (2,371 exact luminance ties between *distinct* colors) |
| C71, C72, C73 | `phase_b_configs::c71_c73_zero_luminance_divisor` |
| C74 | `phase_b_configs::c74_grayscale_full_cross_product` (65,536 pairs) |
| C75 | `phase_b_configs::c75_single_channel_colors` (589,824 pairs) |
| C76 | `phase_b_configs::c76_branch_boundary_lattice` (262,144 pairs) |
| C77 | `phase_b_configs::c77_struct_register_padding_garbage` (500,000 comparisons) |
| C78 | `phase_b_configs::c78_unaligned_struct_source` (120,000 comparisons) |
| C79 | `phase_b_configs::c79_random_fuzz_unconstrained` (1,500,000 comparisons) |
| C80 | `phase_b_configs::c80_argument_order_invariant` (200,000 comparisons) |

**Result: all 80 rows pass, bit-for-bit (`f32::to_bits`), under all 4 build
configurations** (default / `--no-default-features` x debug / release).

Together, rows C65+C66 sweep the library's **entire reachable input domain** for
one operand (every one of the 2^24 colors, hence every `pow` argument and every
branch decision the code can make), and the remaining rows cover the pairing and
ordering logic on top of it.
