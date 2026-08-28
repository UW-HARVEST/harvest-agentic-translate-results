# CONFIGS.md — configuration surface for VALID inputs

Derived mechanically from the C, the mirror of `ERRORS.md`.

## Public entry points (the complete set)

`nm -D` on the C `.so` exports exactly one function, and `lib.h` declares
exactly that one:

```c
void colourblind(cb_impairment Impairment, float *R, float *G, float *B);
```

There is no convenience-vs-low-level split to worry about: `colourblind` **is**
the lowest-level entry point. The three per-impairment transforms
(`Protanopia`, `Deuteranopia`, `Tritanopia`) are `static`, hence unreachable
from outside — the only way to drive them is through `colourblind`, so every row
below goes through it and selects the helper via `Impairment`.

## Axes the C actually branches on

| axis | where the C branches | values enumerated |
|---|---|---|
| **A. impairment / mode** | `switch (Impairment)`, `lib.c:25` — three `case` labels selecting three different coefficient matrices | `cbProtanopia`(0), `cbDeuteranopia`(1), `cbTritanopia`(2) |
| **B. expression shape per component** | each helper writes 3 components with a *different* expression shape: `a*R + b*G + c*B` (add-chain), `a*R + b*G - c*B` (sub tail), `a*R + b*G + B` (raw `B` addend, `Protanopia`/`Deuteranopia` blue), `R + b*G - c*B` (raw `R` addend, `Tritanopia` red) | all 4 shapes, covered by A |
| **C. pointer aliasing** | not a branch, but observable state: all three helpers read `*Red,*Green,*Blue` into locals **first** (`lib.c:4,11,18`) then store Red→Green→Blue, so aliasing changes the result | distinct; `R==G`; `R==B`; `G==B`; `R==G==B`; reversed/permuted distinct pointers |
| **D. float value class** | no branch in the source, but the hardware branches: `mulss`/`addss`/`subss` behave differently per IEEE class, and the coefficients span 1e-11…8.7e-1 so class mixing is reachable | in-gamut `[0,1]`; wide normals; `±0`; subnormals; near-`MIN_POSITIVE`; near-`MAX` (overflow to `±INF`); `±INF`; qNaN; sNaN; sign-mismatched NaN pairs; exact powers of two |
| **E. non-finite mixing** | `INF*0`, `INF-INF`, two NaNs meeting in one expression (the *destination* operand of `mulss`/`addss`/`subss` wins the NaN tie, so operand order is observable) | all three impairments × NaN/INF placements in R, G, B |
| **F. memory placement / alignment** | `movss` has no alignment requirement | 4-byte aligned; deliberately misaligned by 1,2,3 bytes; three separate allocations vs one contiguous array |
| **G. call sequencing** | no internal state, but a stateful bug in the Rust would show here | single call; repeated calls on the same buffer (iterated transform); interleaved impairments on the same buffer |
| **H. build configuration** | `grep -rE '#if|#ifdef|#define' c_src/` → **none**. `[features]` in `Cargo.toml` → **none** | exactly one configuration exists (`--no-default-features` ≡ default) |

Axis **A × C × D/E** is the meaningful cross-product; **B** is implied by A,
**F**/**G** are orthogonal robustness axes applied on top, and **H** collapses to
a single configuration.

Every row is checked with **many randomized inputs** from a fixed-seed
(`0x5EED_C0DE_1234_5678`) SplitMix64 generator, and compared **bit-exactly**
(`f32::to_bits`) between the C `.so` and the Rust `.so`, both loaded with
`libloading`. Row counts are per-row `N` in the test source. Rows 25-28 are
*exhaustive* rather than sampled.

Total compared calls across the suite: **~830 million** per profile
(`cargo test --release` takes ~25 s). Run everything, including the anti-vacuity
gates, with `scripts/verify_all.sh`.

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `colourblind` | `cbProtanopia`, distinct pointers, in-gamut random `[0,1]` (N=20000) | `cfg_row01_protanopia_in_gamut` | [x] |
| 2 | `colourblind` | `cbDeuteranopia`, distinct pointers, in-gamut random `[0,1]` (N=20000) | `cfg_row02_deuteranopia_in_gamut` | [x] |
| 3 | `colourblind` | `cbTritanopia`, distinct pointers, in-gamut random `[0,1]` (N=20000) | `cfg_row03_tritanopia_in_gamut` | [x] |
| 4 | `colourblind` | all 3 impairments, distinct pointers, **uniform random bit patterns** (any of the 2³² floats incl. NaN/INF/subnormal) (N=60000 each) | `cfg_row04_all_impairments_random_bitpatterns` | [x] |
| 5 | `colourblind` | all 3 impairments, **wide normals** `±1e±38` log-uniform → exercises overflow to `±INF` and underflow to subnormal (N=20000 each) | `cfg_row05_all_impairments_wide_normals` | [x] |
| 6 | `colourblind` | all 3 impairments, **signed zeros** — all 8 sign combinations of `±0.0` | `cfg_row06_signed_zeros` | [x] |
| 7 | `colourblind` | all 3 impairments, **subnormals**: smallest/largest subnormal and random subnormal triples, both signs (N=5000) | `cfg_row07_subnormals` | [x] |
| 8 | `colourblind` | all 3 impairments, **near-`f32::MAX`** so each component overflows to `±INF` (N=5000) | `cfg_row08_overflow_to_infinity` | [x] |
| 9 | `colourblind` | all 3 impairments, `±INF` in every position — all 3⁴ combinations of {`-INF`,`0`,`+INF`} plus `INF*tiny_coeff`, `INF-INF` | `cfg_row09_infinities_all_positions` | [x] |
| 10 | `colourblind` | all 3 impairments, **qNaN with random payloads**, both signs, in each of R/G/B and in all pairs/triples — pins NaN-payload *and* NaN-sign propagation (`addss`/`subss`/`mulss` keep the destination operand) (N=8000) | `cfg_row10_quiet_nan_payload_propagation` | [x] |
| 11 | `colourblind` | all 3 impairments, **sNaN** in each position — must quieten identically (payload + sign) | `cfg_row11_signalling_nan_quieting` | [x] |
| 12 | `colourblind` | all 3 impairments, **two NaNs with opposite signs / different payloads meeting in one expression** — the exact case where operand order is observable | `cfg_row12_nan_vs_nan_operand_order` | [x] |
| 13 | `colourblind` | all 3 impairments, **exact powers of two** and dyadic values (exact-arithmetic corners, ties-to-even in the add chain) (N=5000) | `cfg_row13_powers_of_two_and_ties` | [x] |
| 14 | `colourblind` | all 3 impairments, aliasing **`R == G`** (2 distinct pointers), random data (N=6000) | `cfg_row14_alias_r_eq_g` | [x] |
| 15 | `colourblind` | all 3 impairments, aliasing **`R == B`**, random data (N=6000) | `cfg_row15_alias_r_eq_b` | [x] |
| 16 | `colourblind` | all 3 impairments, aliasing **`G == B`**, random data (N=6000) | `cfg_row16_alias_g_eq_b` | [x] |
| 17 | `colourblind` | all 3 impairments, aliasing **`R == G == B`** (1 pointer) — only the last store survives, random data incl. exotic bits (N=6000) | `cfg_row17_alias_all_three` | [x] |
| 18 | `colourblind` | all 3 impairments, **permuted / reversed distinct pointers** into one array (`&a[2],&a[1],&a[0]` and all 6 permutations) — proves argument order, not memory order, drives the maths (N=3000) | `cfg_row18_permuted_pointers` | [x] |
| 19 | `colourblind` | all 3 impairments, **misaligned** pointers (offset 1,2,3 bytes) with random data (N=3000) | `cfg_row19_misaligned_layout` | [x] |
| 20 | `colourblind` | all 3 impairments, **three separate heap allocations** vs one contiguous array (same values) — result must be layout-independent (N=3000) | `cfg_row20_separate_allocations` | [x] |
| 21 | `colourblind` | all 3 impairments, **repeated in-place application** (100 iterations on the same buffer) — an idempotence/stateless check that also walks values far outside `[0,1]` (N=500 seeds) | `cfg_row21_repeated_in_place` | [x] |
| 22 | `colourblind` | **interleaved impairments** on the same buffer in random order (mode switching, e.g. P→T→D→P…), 50 calls per buffer (N=500 seeds) | `cfg_row22_interleaved_impairments` | [x] |
| 23 | `colourblind` | all 3 impairments, **every valid impairment × every value class**, full cross-product sweep from a class table (`{in-gamut, ±0, subnormal, MIN_POSITIVE, 1.0, MAX, ±INF, qNaN, sNaN}`³ × 3 modes) | `cfg_row23_mode_by_class_cross_product` | [x] |
| 24 | `colourblind` | all 3 impairments, **`u8` 0..=255 sweep scaled to `[0,1]`** — the library's real-world use (24-bit sRGB pixels), all 16.7M triples sampled deterministically (N=200000) | `cfg_row24_srgb_pixel_sweep` | [x] |
| 25 | `colourblind` | all 3 impairments, **EXHAUSTIVE over all 2^24 NaN encodings** (sign × 23-bit payload) in each channel position, against 4 partner pairs (normal/normal, opposite-signed NaN, ±INF, sNaN+qNaN) — 604 M compared calls | `cfg_row25_exhaustive_nan_payload_and_sign_in_each_channel` | [x] |
| 26 | `colourblind` | all 3 impairments, **EXHAUSTIVE over all 2^24 NaN encodings with all three channels NaN at once** (three distinct payloads/signs, 3 rotations) — the only shape where every add/sub has two NaN operands, so the destination-wins tie-break is decisive. 75 M compared calls | `cfg_row26_exhaustive_nan_in_all_three_channels` | [x] |
| 27 | `colourblind` | all 3 impairments, **EXHAUSTIVE over every biased-exponent pair** (256×256) × 4 mantissas × rotating signs — systematically hits overflow, gradual underflow, cancellation and zero/subnormal/normal/inf/NaN class boundaries | `cfg_row27_exhaustive_exponent_cross_product` | [x] |
| 28 | `colourblind` | all 3 impairments × all 3 positions, **strided sweep of the entire 2^32 input space** of one channel (step 0x101, so every exponent and a spread of every mantissa region is visited) with the other two channels randomised — 150 M compared calls | `cfg_row28_strided_full_u32_channel_sweep` | [x] |
| 29 | `colourblind` | **feature configuration H**: the whole suite re-run under `--no-default-features` and under every combination emitted by `scripts/feature_matrix.sh`, in BOTH the `debug` and `release` profiles (debug enables Rust's UB checks, which is a genuinely different code path — see the `movss` note in `src/lib.rs`). There are no `[features]`, so the matrix is the single default configuration; the script derives that from `Cargo.toml` rather than assuming it | `scripts/feature_matrix.sh` | [x] |
