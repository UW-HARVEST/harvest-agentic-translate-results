# CONFIGS.md — Phase A: configuration surface table (valid inputs)

## Axes derived mechanically from the C source

### Axis 1 — `Impairment` (the only runtime option/mode; the `switch` in `colourblind`)

`grep -n 'case' c_src/src/lib.c` →

| value | enumerator | branch taken |
|---|---|---|
| 0 | `cbProtanopia`   | `Protanopia(R, G, B)` |
| 1 | `cbDeuteranopia` | `Deuteranopia(R, G, B)` |
| 2 | `cbTritanopia`   | `Tritanopia(R, G, B)` |

There are **no other flags, modes, `#ifdef`s, env vars, or global state** in
the library. `grep -cE '#if|#ifdef|getenv|static [^v]' c_src/src/lib.c` finds
only the three `static void` helpers.

### Axis 2 — the three lowest-level entry points

`colourblind` is the ONLY exported symbol, but it is a pure dispatcher: the
real work lives in three distinct straight-line matrix multiplies. Each must be
driven directly (i.e. one row per helper, not one row for "the wrapper"):

* `Protanopia`   — reached with `Impairment = 0`
* `Deuteranopia` — reached with `Impairment = 1`
* `Tritanopia`   — reached with `Impairment = 2`

Their coefficient matrices differ, and `Tritanopia`'s *R* row has a different
expression **shape** (`R + k*G - k*B` — a bare `R` term and a subtraction,
vs. the other two rows' `k*R + k*G ± k*B`), so its operand/rounding order is a
separate code path.

Per-helper expression shapes (each row is a distinct rounding/operand order):

| helper | R row | G row | B row |
|---|---|---|---|
| Protanopia   | `a*R + b*G + c*B` (c ≈ 2.9e-9)  | `a*R + b*G - c*B` (subtract) | `-a*R + b*G + B` (bare B, negative coeff) |
| Deuteranopia | `a*R + b*G + c*B` (c ≈ 3.6e-9)  | `a*R + b*G - c*B` (subtract) | `-a*R + b*G + B` (bare B, negative coeff) |
| Tritanopia   | `R + b*G - c*B` (**bare R**, subtract) | `-a*R + b*G + c*B` (tiny negative a) | `a*R + b*G + c*B` (tiny positive a) |

### Axis 3 — input SHAPE of the three `float` channels

The C does no branching on values, but IEEE-754 does. Distinct shapes the
hardware/rounding treats differently:

| shape | representative values |
|---|---|
| S1 unit-range colour | uniform random in `[0.0, 1.0]` (the library's intended domain) |
| S2 byte-range colour | uniform random in `[0.0, 255.0]` |
| S3 signed / out-of-gamut | uniform random in `[-1e3, 1e3]` |
| S4 zeros | `+0.0`, `-0.0` in every position (sign-of-zero of the result is observable) |
| S5 subnormals | `f32::from_bits(1)`, `MIN_POSITIVE/2`, random subnormal bit patterns (tests absence of FTZ/DAZ) |
| S6 huge / overflow to ±Inf | `f32::MAX`, `MAX/2`, `3.0e38` (products/sums overflow → ±Inf) |
| S7 tiny / underflow to 0 | `f32::MIN_POSITIVE`, `1e-40` (products underflow: note the `2.9e-9`/`3.6e-9`/`4.5e-11` coefficients) |
| S8 ±Infinity | `±f32::INFINITY` in 1, 2 and 3 channels (`Inf*0` and `Inf - Inf` → NaN inside the expression) |
| S9 single NaN | one channel NaN (quiet, both signs), others finite |
| S10 multi-NaN, differing signs & payloads | 2 or 3 channels NaN with **different sign bits and different payloads** — this is the operand-order-sensitive case: `ADDSS`/`MULSS`/`SUBSS` return the *destination* operand when both are NaN, so the surviving NaN's bits reveal the exact operand order the compiler chose. This is the single most divergence-prone shape. |
| S11 signalling NaN | `f32::from_bits(0x7F80_0001)` and `0xFF80_0001` (must be quieted identically) |
| S12 fully random bit patterns | `u32` random → `f32::from_bits` (covers all classes at once, incl. NaN payloads) |

### Axis 4 — pointer ALIASING (a real, observable API shape)

Each helper snapshots `*Red`, `*Green`, `*Blue` into locals **before** any
store, then stores in the order `*Red`, `*Green`, `*Blue`. With distinct
pointers this is invisible; with aliasing pointers it is fully observable.
`colourblind` takes three independent `float*`, so a caller can legally pass:

| A1 | `R`, `G`, `B` all distinct |
| A2 | `R == G`, `B` distinct |
| A3 | `R == B`, `G` distinct |
| A4 | `G == B`, `R` distinct |
| A5 | `R == G == B` (one variable) |

### Axis 5 — memory layout / repeated invocation

| L1 | three separate stack slots |
| L2 | three adjacent elements of one array (contiguous) |
| L3 | heap `Vec<f32>` pixel buffer, function applied per pixel over many pixels (real consumer pattern, catches state leakage between calls) |
| L4 | the same triple fed through `colourblind` repeatedly (idempotence/accumulation must match) |

## The configuration table (pruned cross-product)

Every row is run against **both** `.so`s via `libloading` and compared
**bit-for-bit** (`to_bits()`, so NaN payloads and signed zeros count).
Rows marked "randomised" use ≥2000 seeded-random inputs (seed fixed,
`SplitMix64`), not one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `Protanopia` (`Impairment=0`) | S1 unit-range, A1 distinct, L1 — randomised | `cfg_c1` | [x] |
| C2 | `Deuteranopia` (`Impairment=1`) | S1 unit-range, A1 distinct, L1 — randomised | `cfg_c2` | [x] |
| C3 | `Tritanopia` (`Impairment=2`) | S1 unit-range, A1 distinct, L1 — randomised | `cfg_c3` | [x] |
| C4 | all 3 | S2 byte-range `[0,255]`, A1, L1 — randomised | `cfg_c4` | [x] |
| C5 | all 3 | S3 signed out-of-gamut `[-1e3,1e3]`, A1, L1 — randomised | `cfg_c5` | [x] |
| C6 | all 3 | S4 ±0.0 — exhaustive over all 8 sign combinations × 3 impairments | `cfg_c6` | [x] |
| C7 | all 3 | S5 subnormals, A1 — randomised subnormal bit patterns | `cfg_c7` | [x] |
| C8 | all 3 | S6 huge → overflow to ±Inf, A1 — randomised near `f32::MAX` | `cfg_c8` | [x] |
| C9 | all 3 | S7 tiny → underflow, A1 — randomised | `cfg_c9` | [x] |
| C10 | all 3 | S8 ±Inf in 1/2/3 channels — exhaustive over the pattern set × 3 impairments | `cfg_c10` | [x] |
| C11 | all 3 | S9 single quiet NaN (both signs, several payloads) × channel position × 3 impairments | `cfg_c11` | [x] |
| C12 | all 3 | **S10 multi-NaN, differing signs and payloads** — exhaustive over channel subsets × sign/payload combos × 3 impairments (operand-order-sensitive) | `cfg_c12` | [x] |
| C13 | all 3 | S11 signalling NaN, both signs, each channel × 3 impairments | `cfg_c13` | [x] |
| C14 | all 3 | S12 fully random `u32` bit patterns, A1 — large randomised sweep (all float classes mixed) | `cfg_c14` | [x] |
| C15 | all 3 | A2 `R==G` aliasing × S1 and S12 — randomised | `cfg_c15` | [x] |
| C16 | all 3 | A3 `R==B` aliasing × S1 and S12 — randomised | `cfg_c16` | [x] |
| C17 | all 3 | A4 `G==B` aliasing × S1 and S12 — randomised | `cfg_c17` | [x] |
| C18 | all 3 | A5 `R==G==B` single slot × S1 and S12 — randomised | `cfg_c18` | [x] |
| C19 | all 3 | L2 contiguous array-of-3 (checks no out-of-bounds write past the triple; guard elements verified untouched) | `cfg_c19` | [x] |
| C20 | all 3 | L3 heap pixel buffer, per-pixel application over 4096 random pixels, whole buffer compared | `cfg_c20` | [x] |
| C21 | all 3 | L4 repeated application (10 iterations) of the same impairment to one triple — convergence/accumulation path | `cfg_c21` | [x] |
| C22 | all 3 | mixed sequence: apply impairments 0,1,2,0,1,2… in one process to the same buffer (cross-helper state leakage) | `cfg_c22` | [x] |
| C23 | all 3 | S1 but with one channel exactly `1.0` / `0.0` / `-1.0` boundary constants × all positions × 3 impairments | `cfg_c23` | [x] |
| C24 | all 3 | values chosen so a sum is an exact tie needing round-to-nearest-even (`x`, `x+ulp` pairs around powers of two) — randomised around `2^k` | `cfg_c24` | [x] |

## Feature combinations

`Cargo.toml` has **no `[features]` table**, so the only combination is the
default (== `--no-default-features`). Verified by running the suite under both
`cargo test --release` and `cargo test --release --no-default-features`.

## Finding: NaN-payload selection is C-compiler-optimisation dependent

Recorded here because it constrains what "byte-identical" can mean.

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, and the generated
`flags.make` confirms the compile line is exactly `C_FLAGS = -fPIC` — i.e. gcc
at its default **`-O0`**. The Rust is pinned to that build and matches it with
**0 failing tests**.

Rebuilding the *same* C source at other optimisation levels and re-running the
suite (only the `C_SO` env var changes):

| C build | failing tests |
|---|---|
| `gcc -O0` (**the canonical CMake build**) | **0** |
| `gcc -O1` | 3 |
| `gcc -O2` | 3 |
| `gcc -O3` | 3 |
| `gcc -Os` | 3 |

All three failures in the optimised builds are `cfg_c11` / `cfg_c12` /
`cfg_c13` — the NaN rows — and every reported difference is only *which* NaN
operand survives, e.g. for `Impairment=0`, input
`[0x7F800001, 0xFFC00000, 0x3F800000]`:

```
gcc -O0 / Rust -> [0x7FC00001, 0xFFC00000, 0xFFC00000]
gcc -O2         -> [0x7FC00001, 0x7FC00001, 0x7FC00001]
```

`ADDSS`/`SUBSS`/`MULSS` return the **destination** operand when both operands
are NaN, so the surviving payload is decided purely by the operand order the
compiler picked. gcc reorders those operands at `-O1` and above, so the two C
builds **disagree with each other** on these inputs — no single Rust
implementation can match both. The translation therefore matches the build that
`c_src/CMakeLists.txt` actually produces (`-O0`). Every non-NaN input agrees
across all five C builds.

`./run_all.sh` prints this comparison as an informational step.
