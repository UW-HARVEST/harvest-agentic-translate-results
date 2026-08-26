# CONFIGS.md — Phase B configuration-surface table

Derived **mechanically** from `c_src/include/lib.h` (the whole public API) and
every branch in `c_src/src/lib.c`.

## Build-time configuration axes

| axis | values | source |
|------|--------|--------|
| Cargo features | **none** — `Cargo.toml` has no `[features]` section | `Cargo.toml` |
| CMake options | **none** — `CMakeLists.txt` declares no `option()`/`if()`/`target_compile_definitions` beyond the automatic `-Dtranslated_rust_EXPORTS` | `c_src/CMakeLists.txt` |
| C preprocessor conditionals | **none** — the only directive in the whole C source is `#include "lib.h"` | `grep '^#' c_src/**` |

⇒ **Exactly one** feature combination exists: the empty set.
`cargo check/test` and `cargo check/test --no-default-features` are the same
build. Both are still run explicitly (Phase D).

## Public entry points (the FULL set)

`c_src/include/lib.h` declares exactly **one** function and **one** enum:

```c
typedef enum cb_impairment { cbProtanopia, cbDeuteranopia, cbTritanopia } cb_impairment;
void colourblind(cb_impairment Impairment, float *R, float *G, float *B);
```

`colourblind` **is** the lowest-level entry point — there is no convenience
wrapper layer. The three transform kernels `Protanopia`, `Deuteranopia`,
`Tritanopia` are `static` (not in the dynamic symbol table, see `SYMBOLS.md`)
and are reachable *only* by selecting `Impairment` 0 / 1 / 2, so driving all
three `Impairment` values **is** driving the low-level surface directly.

## Runtime configuration axes the C actually branches on

* **Axis I — `Impairment`** (`switch`, `lib.c:25`): 3 distinct code paths
  (`0`→`Protanopia`, `1`→`Deuteranopia`, `2`→`Tritanopia`) + a fall-through
  no-op path for everything else (that path is `ERRORS.md` rows E1–E8).
  Each of the 3 paths uses a *different* 3×3 constant matrix and a *different*
  operator pattern, so the cross-product with the input shape is meaningful:

  | path | `*Red` | `*Green` | `*Blue` |
  |------|--------|----------|---------|
  | `Protanopia`   | `a*R + b*G + c*B` | `a*R + b*G - c*B` | `-a*R + b*G + B` |
  | `Deuteranopia` | `a*R + b*G + c*B` | `a*R + b*G - c*B` | `-a*R + b*G + B` |
  | `Tritanopia`   | `R + b*G - c*B`   | `-a*R + b*G + c*B` | `a*R + b*G + c*B` |

* **Axis II — float input shape.** There is no `if` on the float values, but the
  arithmetic itself is value-dependent, so the distinct shapes the *hardware*
  treats differently are the real configuration axis: normals, ±0.0, subnormals,
  ±FLT_MAX (sum overflow), FLT_MIN, ±inf, NaN (incl. payload propagation and
  `inf - inf`), and catastrophic cancellation.

* **Axis III — pointer aliasing.** The C signature has **no `restrict`**, so a
  caller may legally pass the same `float*` for two or three arguments. The
  kernels read all three values into locals *before* any store
  (`float R = *Red, G = *Green, B = *Blue;`), so under aliasing the C result is
  well defined: the **last** store to a given address wins, and every read sees
  the *original* value. This is a genuine axis and must be replicated exactly.

* **Axis IV — pointer alignment.** GCC emits `movss`, which has no alignment
  requirement, so byte-offset `float*`s work in C.

* **Axis V — call sequencing.** No globals, no `static` mutable state in the C
  ⇒ every call must be independent and deterministic. Verified, not assumed.

## CONFIGURATION-SURFACE TABLE

Every row is exercised by a differential test that loads **both** `.so`s via
`libloading` and compares the three output `f32`s **bit-for-bit**
(`to_bits()` equality, so `-0.0 != 0.0` and NaN payloads are compared exactly).
Every row uses many randomized inputs from a fixed-seed SplitMix64 PRNG.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, random normals in [0,1] (intended sRGB-ish range) — 1000 seeded inputs | `cfg_v1_imp0` | [x] |
| C2 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, random normals in [0,1] (intended sRGB-ish range) — 1000 seeded inputs | `cfg_v1_imp1` | [x] |
| C3 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, random normals in [0,1] (intended sRGB-ish range) — 1000 seeded inputs | `cfg_v1_imp2` | [x] |
| C4 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, random normals in [-1,1] (negatives) — 1000 seeded inputs | `cfg_v2_imp0` | [x] |
| C5 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, random normals in [-1,1] (negatives) — 1000 seeded inputs | `cfg_v2_imp1` | [x] |
| C6 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, random normals in [-1,1] (negatives) — 1000 seeded inputs | `cfg_v2_imp2` | [x] |
| C7 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, random normals across full exponent range 2^-126..2^127 — 1000 seeded inputs | `cfg_v3_imp0` | [x] |
| C8 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, random normals across full exponent range 2^-126..2^127 — 1000 seeded inputs | `cfg_v3_imp1` | [x] |
| C9 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, random normals across full exponent range 2^-126..2^127 — 1000 seeded inputs | `cfg_v3_imp2` | [x] |
| C10 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, all exact +0.0 — 1 input | `cfg_v4_imp0` | [x] |
| C11 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, all exact +0.0 — 1 input | `cfg_v4_imp1` | [x] |
| C12 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, all exact +0.0 — 1 input | `cfg_v4_imp2` | [x] |
| C13 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, all 8 combinations of signed zeros (+-0.0) — 8 inputs | `cfg_v5_imp0` | [x] |
| C14 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, all 8 combinations of signed zeros (+-0.0) — 8 inputs | `cfg_v5_imp1` | [x] |
| C15 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, all 8 combinations of signed zeros (+-0.0) — 8 inputs | `cfg_v5_imp2` | [x] |
| C16 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, subnormals (incl. smallest 1e-45 and random subnormal bit patterns) — 500 seeded inputs | `cfg_v6_imp0` | [x] |
| C17 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, subnormals (incl. smallest 1e-45 and random subnormal bit patterns) — 500 seeded inputs | `cfg_v6_imp1` | [x] |
| C18 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, subnormals (incl. smallest 1e-45 and random subnormal bit patterns) — 500 seeded inputs | `cfg_v6_imp2` | [x] |
| C19 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, FLT_MAX / -FLT_MAX (sums overflow to +-inf) — 8 combos | `cfg_v7_imp0` | [x] |
| C20 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, FLT_MAX / -FLT_MAX (sums overflow to +-inf) — 8 combos | `cfg_v7_imp1` | [x] |
| C21 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, FLT_MAX / -FLT_MAX (sums overflow to +-inf) — 8 combos | `cfg_v7_imp2` | [x] |
| C22 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, FLT_MIN smallest normal (+-1.17549435e-38) — 8 combos | `cfg_v8_imp0` | [x] |
| C23 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, FLT_MIN smallest normal (+-1.17549435e-38) — 8 combos | `cfg_v8_imp1` | [x] |
| C24 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, FLT_MIN smallest normal (+-1.17549435e-38) — 8 combos | `cfg_v8_imp2` | [x] |
| C25 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, all 8 combinations of +-inf (yields inf-inf = NaN in Tritanopia) — 8 inputs | `cfg_v9_imp0` | [x] |
| C26 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, all 8 combinations of +-inf — 8 inputs | `cfg_v9_imp1` | [x] |
| C27 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, all 8 combinations of +-inf (yields inf-inf = NaN) — 8 inputs | `cfg_v9_imp2` | [x] |
| C28 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, quiet NaN, negative NaN, NaN with distinct payload bits (payload propagation) — 64 combos | `cfg_v10_imp0` | [x] |
| C29 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, quiet NaN, negative NaN, NaN with distinct payload bits — 64 combos | `cfg_v10_imp1` | [x] |
| C30 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, quiet NaN, negative NaN, NaN with distinct payload bits — 64 combos | `cfg_v10_imp2` | [x] |
| C31 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, mixed +-inf with finite operands — 500 seeded inputs | `cfg_v11_imp0` | [x] |
| C32 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, mixed +-inf with finite operands — 500 seeded inputs | `cfg_v11_imp1` | [x] |
| C33 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, mixed +-inf with finite operands — 500 seeded inputs | `cfg_v11_imp2` | [x] |
| C34 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, G == B exactly (catastrophic cancellation in Tritanopia's c*G - c'*B) — 500 seeded inputs | `cfg_v12_imp0` | [x] |
| C35 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, G == B exactly — 500 seeded inputs | `cfg_v12_imp1` | [x] |
| C36 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, G == B exactly (catastrophic cancellation) — 500 seeded inputs | `cfg_v12_imp2` | [x] |
| C37 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, arbitrary random u32 bit patterns reinterpreted as f32 (covers every class incl. sNaN) — 4000 seeded inputs | `cfg_v13_imp0` | [x] |
| C38 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, arbitrary random u32 bit patterns as f32 — 4000 seeded inputs | `cfg_v13_imp1` | [x] |
| C39 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, arbitrary random u32 bit patterns as f32 — 4000 seeded inputs | `cfg_v13_imp2` | [x] |
| C40 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, exact integers and powers of two (+-2^k, k in -149..127) — all k | `cfg_v14_imp0` | [x] |
| C41 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, exact integers and powers of two — all k | `cfg_v14_imp1` | [x] |
| C42 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, exact integers and powers of two — all k | `cfg_v14_imp2` | [x] |
| C43 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, adjacent values 1 ULP apart (v, nextafter(v)) around many magnitudes — 1000 seeded inputs | `cfg_v15_imp0` | [x] |
| C44 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, adjacent values 1 ULP apart — 1000 seeded inputs | `cfg_v15_imp1` | [x] |
| C45 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, adjacent values 1 ULP apart — 1000 seeded inputs | `cfg_v15_imp2` | [x] |
| C46 | `colourblind` | `Impairment=0` (cbProtanopia -> static `Protanopia`), distinct ptrs, iterated in-place application: output fed back as input 32 times — 300 seeded inputs | `cfg_v16_imp0` | [x] |
| C47 | `colourblind` | `Impairment=1` (cbDeuteranopia -> static `Deuteranopia`), distinct ptrs, iterated in-place 32 times — 300 seeded inputs | `cfg_v16_imp1` | [x] |
| C48 | `colourblind` | `Impairment=2` (cbTritanopia -> static `Tritanopia`), distinct ptrs, iterated in-place 32 times — 300 seeded inputs | `cfg_v16_imp2` | [x] |
| C49 | `colourblind` | `Impairment=0` (cbProtanopia), **R and G alias the same float** (no `restrict` in C; reads precede writes), random normals + full bit patterns — 1000 inputs | `cfg_a2_imp0` | [x] |
| C50 | `colourblind` | `Impairment=1` (cbDeuteranopia), **R and G alias the same float**, random normals + full bit patterns — 1000 inputs | `cfg_a2_imp1` | [x] |
| C51 | `colourblind` | `Impairment=2` (cbTritanopia), **R and G alias the same float**, random normals + full bit patterns — 1000 inputs | `cfg_a2_imp2` | [x] |
| C52 | `colourblind` | `Impairment=0` (cbProtanopia), **R and B alias the same float**, random normals + full bit patterns — 1000 inputs | `cfg_a3_imp0` | [x] |
| C53 | `colourblind` | `Impairment=1` (cbDeuteranopia), **R and B alias the same float**, random normals + full bit patterns — 1000 inputs | `cfg_a3_imp1` | [x] |
| C54 | `colourblind` | `Impairment=2` (cbTritanopia), **R and B alias the same float**, random normals + full bit patterns — 1000 inputs | `cfg_a3_imp2` | [x] |
| C55 | `colourblind` | `Impairment=0` (cbProtanopia), **G and B alias the same float**, random normals + full bit patterns — 1000 inputs | `cfg_a4_imp0` | [x] |
| C56 | `colourblind` | `Impairment=1` (cbDeuteranopia), **G and B alias the same float**, random normals + full bit patterns — 1000 inputs | `cfg_a4_imp1` | [x] |
| C57 | `colourblind` | `Impairment=2` (cbTritanopia), **G and B alias the same float**, random normals + full bit patterns — 1000 inputs | `cfg_a4_imp2` | [x] |
| C58 | `colourblind` | `Impairment=0` (cbProtanopia), **R, G and B all alias one single float**, random normals + full bit patterns — 1000 inputs | `cfg_a5_imp0` | [x] |
| C59 | `colourblind` | `Impairment=1` (cbDeuteranopia), **R, G and B all alias one single float**, random normals + full bit patterns — 1000 inputs | `cfg_a5_imp1` | [x] |
| C60 | `colourblind` | `Impairment=2` (cbTritanopia), **R, G and B all alias one single float**, random normals + full bit patterns — 1000 inputs | `cfg_a5_imp2` | [x] |
| C61 | `colourblind` | byte-offset (unaligned) `float*` for all three args, `Impairment` in {0,1,2}, random normals — 300 inputs (C `movss` has no alignment requirement) | `cfg_unaligned_all_imps` | [x] |
| C62 | `colourblind` | sequential calls chaining all three impairments 0->1->2 on the same buffer, random normals — 500 inputs (checks absence of hidden global state) | `cfg_chain_all_imps` | [x] |
| C63 | `colourblind` | same input replayed twice in a row (determinism / no internal state) for `Impairment` in {0,1,2} | `cfg_replay_determinism` | [x] |
| C64 | `colourblind` | NaN **mixed with finite** operands: exhaustive 3-cube over a pool of 4 NaN variants + `{0.25, -3.5, 0.0, +inf}`, `Impairment` in {0,1,2} — 512 combos x 3 | `cfg_v17_nan_finite_cube_all_imps` | [x] |
| C65 | `colourblind` | high-volume randomized soak: 200 000 arbitrary-bit-pattern triples with a randomly chosen valid `Impairment` | `cfg_soak_random_bit_patterns` | [x] |

**TOTAL: 65 rows.**

## Why the per-channel NaN rows matter (C28–C30, C64)

These rows found a real divergence. When an operand is already a NaN, an x86
SSE instruction forwards a NaN operand rather than computing, preferring the
one in the *destination* register (Intel SDM, "Rules for Handling NaNs"). GCC's
`-O0` register allocation puts a different term in the destination for different
sub-expressions, so the surviving payload differs **per output channel**:

| kernel | `*Red` | `*Green` | `*Blue` |
|--------|--------|----------|---------|
| `Protanopia`   | **B, R, G** | G, R, B | G, R, B |
| `Deuteranopia` | **B, R, G** | G, R, B | G, R, B |
| `Tritanopia`   | G, R, B | **B, R, G** | **B, R, G** |

e.g. `Protanopia`'s `*Red` ends in `addss %xmm1,%xmm0` with `%xmm0 = c*B`, so
`B`'s payload wins, while `*Green` ends in `subss %xmm1,%xmm0` with
`%xmm0 = b*G + a*R`, so `G`'s payload wins. `src/lib.rs` reproduces this by
transcribing the instruction sequence through `mulss`/`addss`/`subss` helpers
that take `(dest, src)` in machine order.

## Test-efficacy evidence (mutation testing)

A test suite that passes proves nothing unless it can *fail*. Five mutations
were injected into `src/lib.rs`, the suite was re-run, and the source was
restored (verified byte-identical afterwards with `diff`):

| mutation | detected? | rows that caught it |
|----------|-----------|---------------------|
| original `&mut *ptr` deref (the as-translated code) | **yes** — process abort | `cfg_unaligned_all_imps` (C61) |
| one coefficient changed by 1 ULP (`0.82944301379913`) | **yes** | 20+ value rows incl. C1–C9, C34–C39, C61–C65 |
| `addss` `(dest, src)` operands swapped in `Protanopia`'s `*Red` | **yes** | `cfg_v10_imp0` (C28), `cfg_v17_...` (C64), `cfg_soak_...` (C65) — *only* the NaN rows, as expected |
| a `default:` arm added to the `switch` | **yes** | `ERRORS.md` rows E1–E6 |
| final store order reversed in `Protanopia` | **yes** | `cfg_a2_imp0`, `cfg_a3_imp0`, `cfg_a4_imp0`, `cfg_a5_imp0` (C49/C52/C55/C58) — *only* the aliasing rows, as expected |

Two further mutations were **equivalent mutants** (no observable behaviour
change on x86-64) and are correctly not reported as failures: swapping
`read_unaligned`/`write_unaligned` for `read`/`write` (both lower to `movss`;
differs only in Rust's debug UB checks), and storing an already-read value back
before reading the other lanes.

## Caveat: NaN payloads are a property of the C *build*, not the C *source*

Measured over the 3-cube of {4 NaN variants + 4 finite values} × 3 impairments
(1536 cases) with a small `dlopen` harness:

| comparison | cases differing |
|------------|-----------------|
| C as specified by `c_src/CMakeLists.txt` (`-O0`) **vs Rust** | **0 / 1536** |
| C as specified (`-O0`) **vs the same C source built `-O3`** | 339 / 1536 |

The second row is the C library disagreeing with *itself* across optimisation
levels: which NaN payload survives depends on GCC's register allocation, and
`-O3` allocates differently. NaN-payload identity is therefore only definable
relative to a specific C build.

`src/lib.rs` is matched bit-exactly to the build the project actually
specifies — `CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so `C_FLAGS = -fPIC`
and the effective optimisation level is `-O0`. Every non-NaN input (finite,
zero, signed zero, subnormal, `±inf`, overflowing sums) is bit-identical
regardless of optimisation level, because the NaN-forwarding rule is the only
place where operand *order* is observable.
