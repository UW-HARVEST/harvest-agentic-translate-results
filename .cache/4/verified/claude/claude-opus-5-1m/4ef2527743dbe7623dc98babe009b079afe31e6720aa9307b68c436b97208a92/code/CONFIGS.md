# CONFIGS.md — CONFIGURATION-SURFACE TABLE (Phase B)

The library exposes no runtime "option" struct; its configuration axes are
(a) the **enum/mode arguments** the public API takes, and (b) the **input
shapes** the C code branches on. Both were derived mechanically from
`c_src/src/lib.c` (`if` / `else if` / `switch` / ternary / comparison sites) and
`c_src/include/lib.h`.

## Axes actually branched on in the C

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| `C2_TYPE typeA` | `CIRCLE`(0), `AABB`(1), out-of-range | `lib.c:84-107` |
| `C2_TYPE typeB` | `CIRCLE`(0), `AABB`(1), out-of-range | `lib.c:86-103, 96-103` |
| geometry relation | overlapping / disjoint / exactly touching (`d2 == r2`) / contained | `lib.c:64, 72, 76-80` |
| float class | normal / `±0.0` / subnormal / `±inf` / quiet `NaN` (payload!) / signalling `NaN` | every `comiss`/`ucomiss`/`addss`/`subss`/`mulss`/`divss` |
| `sign(v1), sign(v2)` | `+/+`, `+/-`, `-/+`, `-/-`, plus `v1==INT_MIN`, `v2==INT_MIN`, `v2==0` | `lib.c:110-139` |
| divisibility | exact (`r == 0`) vs inexact (`r != 0`, sign fixup) | `lib.c:135-138` |
| RNG state | `{0,0}` fixed point / one word zero / arbitrary 64-bit pairs / repeated calls (state mutation) | `lib.c:145-154` |
| `channels` | `== 2` vs `!= 2` (`0, 1, 3, 4, 8, u32::MAX`) | `lib.c:453-455` |
| `bitdepth` | `== 32` vs `!= 32` (`0, 1, 8, 16, 24, 32, 33, u32::MAX`) | `lib.c:455` |
| `blocksize` | `0`, small, `u32::MAX` (overflow) | `lib.c:453-455` |
| triangle shape (`f9`) | non-degenerate / colinear / coincident vertices / `p` inside vs outside | `lib.c:477-490` |
| `h >> 10` (`f10`) | `0` (zero/subnormal row, offset 0), `1..30`, `31` (inf/NaN row), `32` (negative zero/subnormal), `33..62`, `63` (negative inf/NaN row) | `lib.c:862-863`, `m__offset`/`m__exponent` |
| `s == 0` (`f11`,`f12`) | early-return path vs full path | `lib.c:872, 919` |
| hue band (`f11`) | 6 declared bands + the buggy `h<120 && h<180` arm + the `else` arm | `lib.c:881-909` |
| sector `i` (`f12`) | `0,1,2,3,4`, `default` (incl. `i < 0` and `i == INT_MIN`) | `lib.c:931-962` |
| which channel is `max` (`f13`) | `r`, `g`, `b`, ties, `delta == 0`, `max == 0`, `h < 0` wrap | `lib.c:975-999` |
| entry point level | low-level (`c2V`…`c2AABBtoAABB`, `f3`…`f13`) vs dispatcher (`f2`) vs one-shot aggregate (`agglom`) | `include/lib.h`, `nm -D` |

## Configuration rows

Each row is exercised with **many randomized inputs** (fixed-seed
xorshift128+ PRNG in `tests/common/mod.rs`, so runs are reproducible) unless the
row says "exhaustive". Both `.so`s are loaded with `libloading` and only their
exported symbols are called; results are compared as **raw bit patterns**.

Test file: `tests/configs.rs`.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C01 | `c2V` | random `f32` bit patterns (incl. `NaN` payloads, `±0`, subnormals, `±inf`) — struct-return ABI | `c01_c2v` | [x] |
| C02 | `c2Maxv` | random pairs; plus all 25 combinations of `{-inf, -0.0, +0.0, +inf, NaN}` per lane (ordered/unordered `comiss`) | `c02_c2maxv` | [x] |
| C03 | `c2Minv` | ditto for the `<` ternary | `c03_c2minv` | [x] |
| C04 | `c2Clampv` | `lo <= hi`, `lo > hi` (inverted box), `lo == hi`, `NaN` in any of the 6 lanes | `c04_c2clampv` | [x] |
| C05 | `c2Sub` | random pairs; `inf - inf`, `0 - 0`, `NaN` in either operand and in **both** (payload survivor) | `c05_c2sub` | [x] |
| C06 | `c2Dot` | random pairs; both-`NaN` operands with **distinct payloads** (pins gcc's `mulss`/`addss` dst operand) | `c06_c2dot` | [x] |
| C07 | `c2CircletoCircle` | overlapping / disjoint / exactly touching (`d2 == r2`) / zero radius / negative radius / `NaN` radius in one and in **both** circles | `c07_c2circletocircle` | [x] |
| C08 | `c2CircletoAABB` | circle centre inside / outside / on edge / on corner; degenerate box (`min > max`); zero-area box; `NaN` in centre, radius, box | `c08_c2circletoaabb` | [x] |
| C09 | `c2AABBtoAABB` | separated on x / on y / on both / overlapping / touching (`max == min`) / contained / inverted boxes / `NaN` lanes | `c09_c2aabbtoaabb` | [x] |
| C10 | `f2` | `typeA=CIRCLE, typeB=CIRCLE` — dispatch to `c2CircletoCircle`, random circles | `c10_f2_circle_circle` | [x] |
| C11 | `f2` | `typeA=CIRCLE, typeB=AABB` — dispatch to `c2CircletoAABB(A, B)` | `c11_f2_circle_aabb` | [x] |
| C12 | `f2` | `typeA=AABB, typeB=CIRCLE` — dispatch to `c2CircletoAABB(*B, *A)` (**note the argument swap**) | `c12_f2_aabb_circle` | [x] |
| C13 | `f2` | `typeA=AABB, typeB=AABB` — dispatch to `c2AABBtoAABB` | `c13_f2_aabb_aabb` | [x] |
| C14 | `f2` | `A` and `B` aliasing the **same** buffer (self-collision), for all 4 valid type pairs | `c14_f2_aliased` | [x] |
| C15 | `f2` | unaligned `A`/`B` pointers (offset 1..7 inside an over-aligned buffer) for all 4 valid type pairs | `c15_f2_unaligned` | [x] |
| C16 | `f3` | `v1 >= 0, v2 > 0` (the plain `return v1/v2` fast path), random + exact multiples | `c16_f3_pos_pos` | [x] |
| C17 | `f3` | `v1 >= 0, v2 < 0` (`v2 != INT_MIN`), random + exact multiples | `c17_f3_pos_neg` | [x] |
| C18 | `f3` | `v1 < 0` (`!= INT_MIN`), `v2 > 0`, random + exact multiples | `c18_f3_neg_pos` | [x] |
| C19 | `f3` | `v1 < 0` (`!= INT_MIN`), `v2 < 0` (`!= INT_MIN`), random + exact multiples | `c19_f3_neg_neg` | [x] |
| C20 | `f3` | full random `(i32, i32)` cross-product sweep incl. `INT_MIN`/`INT_MAX`/`±1`/`0` | `c20_f3_random_all` | [x] |
| C21 | `f4` | single call, random 128-bit states (both words non-zero) | `c21_f4_single` | [x] |
| C22 | `f4` | **sequences** of 64 successive calls on one state — verifies the in-place state mutation, not just the return value | `c22_f4_sequence` | [x] |
| C23 | `f4` | states with `state[0] == 0`, `state[1] == 0`, all-ones, single-bit states | `c23_f4_edge_states` | [x] |
| C24 | `f5` | exhaustive over all 65 536 low-16-bit values | `c24_f5_exhaustive_low16` | [x] |
| C25 | `f5` | random full 32-bit values (high half must be discarded) | `c25_f5_random_u32` | [x] |
| C26 | `f7` | `channels == 2`, `bitdepth == 32`, random blocksize | `c26_f7_ch2_bd32` | [x] |
| C27 | `f7` | `channels == 2`, `bitdepth != 32`, random blocksize | `c27_f7_ch2_bdother` | [x] |
| C28 | `f7` | `channels != 2`, `bitdepth == 32`, random blocksize | `c28_f7_chother_bd32` | [x] |
| C29 | `f7` | `channels != 2`, `bitdepth != 32`, random blocksize | `c29_f7_chother_bdother` | [x] |
| C30 | `f7` | fully random `(u32, u32, u32)` incl. values that overflow the numerator | `c30_f7_random_all` | [x] |
| C31 | `f9` | non-degenerate triangle, `p` strictly inside | `c31_f9_inside` | [x] |
| C32 | `f9` | non-degenerate triangle, `p` outside (negative / >1 barycentrics) | `c32_f9_outside` | [x] |
| C33 | `f9` | colinear / coincident vertices (`invDenom` = `±inf`), and `p == p1` | `c33_f9_degenerate` | [x] |
| C34 | `f9` | fully random `f32` bit patterns in all 8 lanes (incl. `NaN` payloads and `inf`) | `c34_f9_random_bits` | [x] |
| C35 | `f10` | exhaustive over all 65 536 `uint16_t` inputs (covers every `m__offset`/`m__exponent` row, incl. rows 31 and 63) | `c35_f10_exhaustive` | [x] |
| C36 | `f11` | `s == 0` early-return path, random `h`/`l` | `c36_f11_s_zero` | [x] |
| C37 | `f11` | band 1 `0 <= h < 60`, random `s, l` in `[0,1]` | `c37_f11_band1` | [x] |
| C38 | `f11` | band 2 `60 <= h < 120` | `c38_f11_band2` | [x] |
| C39 | `f11` | band 3 arm `h < 120 && h < 180` — reached only by `h < 0` (the C typo); random negative `h` | `c39_f11_band3_negative_h` | [x] |
| C40 | `f11` | band 4 `180 <= h < 240` | `c40_f11_band4` | [x] |
| C41 | `f11` | band 5 `240 <= h < 300` | `c41_f11_band5` | [x] |
| C42 | `f11` | band 6 `300 <= h < 360` | `c42_f11_band6` | [x] |
| C43 | `f11` | `else` arm: `h >= 360`, `h = +inf`, `h = NaN` | `c43_f11_else_arm` | [x] |
| C44 | `f11` | exact band boundaries `h ∈ {0, 60, 120, 180, 240, 300, 360}` × `l ∈ {0, 0.5, 1}` × `s ∈ {tiny, 0.5, 1}` | `c44_f11_boundaries` | [x] |
| C45 | `f11` | fully random `f32` bit patterns in `h, s, l` (incl. `NaN`, `inf`, subnormals, out-of-gamut `s`/`l`) | `c45_f11_random_bits` | [x] |
| C46 | `f11` | `dest` aliasing `src` (in-place conversion) | `c46_f11_aliased` | [x] |
| C47 | `f12` | `s == 0` early-return path | `c47_f12_s_zero` | [x] |
| C48 | `f12` | sector `i == 0` (`0 <= h < 60`) | `c48_f12_sector0` | [x] |
| C49 | `f12` | sector `i == 1` | `c49_f12_sector1` | [x] |
| C50 | `f12` | sector `i == 2` | `c50_f12_sector2` | [x] |
| C51 | `f12` | sector `i == 3` | `c51_f12_sector3` | [x] |
| C52 | `f12` | sector `i == 4` | `c52_f12_sector4` | [x] |
| C53 | `f12` | `default` sector: `i == 5` (`300 <= h < 360`), `i > 5` (`h >= 360`), `i < 0` (`h < 0`) | `c53_f12_sector_default` | [x] |
| C54 | `f12` | `h` such that `(int)floorf(h/60)` is out of `int` range or `NaN` ⇒ `cvttss2si` indefinite `INT_MIN` | `c54_f12_int_indefinite` | [x] |
| C55 | `f12` | exact sector boundaries `h ∈ {0, 60, 120, 180, 240, 300, 360, -0.0}` × `s, v` grid | `c55_f12_boundaries` | [x] |
| C56 | `f12` | fully random `f32` bit patterns in `h, s, v` | `c56_f12_random_bits` | [x] |
| C57 | `f12` | `dest` aliasing `src` | `c57_f12_aliased` | [x] |
| C58 | `f13` | `r` is strict max (`h = (g-b)/delta`), `g > b` and `g < b` (the `h < 0` wrap) | `c58_f13_r_max` | [x] |
| C59 | `f13` | `g` is strict max (`h = 2 + (b-r)/delta`) | `c59_f13_g_max` | [x] |
| C60 | `f13` | `b` is strict max (`h = 4 + (r-g)/delta`) | `c60_f13_b_max` | [x] |
| C61 | `f13` | ties: `r == g > b`, `g == b > r`, `r == b > g`, `r == g == b` (`delta == 0`) | `c61_f13_ties` | [x] |
| C62 | `f13` | `max == 0`: all-zero input, all-negative input, `-0.0` mixtures | `c62_f13_max_zero` | [x] |
| C63 | `f13` | out-of-gamut: values > 1, negative values, `±inf`, subnormals | `c63_f13_out_of_gamut` | [x] |
| C64 | `f13` | fully random `f32` bit patterns in `r, g, b` (incl. `NaN` in 1, 2 or 3 lanes) | `c64_f13_random_bits` | [x] |
| C65 | `f13` | `dest` aliasing `src` | `c65_f13_aliased` | [x] |
| C66 | `f12` ∘ `f13` | round-trip pipeline: `f13(rgb) -> hsv`, then `f12(hsv) -> rgb'` (composed low-level calls, random rgb) | `c66_f13_f12_roundtrip` | [x] |
| C67 | `f11` ∘ `f13` | pipeline `f13(rgb) -> hsv`, feed as HSL into `f11` (exercises out-of-range `h` from f13 into f11's bands) | `c67_f13_f11_pipeline` | [x] |
| C68 | `agglom` | all 33 parameters fully random bit patterns — the aggregate one-shot entry point | `c68_agglom_random` | [x] |
| C69 | `agglom` | "sane" randomized inputs (in-gamut colours, sensible blocksize/channels/bitdepth, non-degenerate triangle, non-zero RNG state) | `c69_agglom_sane` | [x] |
| C70 | `agglom` | boundary matrix: `f3_2 = 0`, RNG state `{0,0}`, `channels ∈ {0,1,2,3}` × `bitdepth ∈ {0,16,32,33}`, `f10_1 ∈ {0, 0x3ff, 0x7c00, 0xffff}`, `s = 0` for both colour ops | `c70_agglom_boundaries` | [x] |
| C71 | `f11` | libm boundary: full `f32` exponent sweep (sign × exp 0..255 × 61 mantissa patterns) through `fmodf(h/60, 2)`, × 4 `(s, l)` pairs — the C imports glibc's `fmodf`, the Rust links `compiler-builtins`' | `c71_f11_fmodf_sweep` | [x] |
| C72 | `f12` | libm boundary: same exponent sweep through `floorf(h/60)`, × 4 `(s, v)` pairs | `c72_f12_floorf_sweep` | [x] |
| C73 | `agglom` | same exponent sweep applied simultaneously to `f11`/`f12`/`f13`/`f9`/`f2` float parameters | `c73_agglom_exponent_sweep` | [x] |

## Mutation audit (test-sensitivity evidence)

A row being "checked" is only meaningful if the test could actually fail. 42
mutants were injected into `src/lib.rs` one at a time and the suite re-run
(`cargo test --test configs --test errors`, with the cdylib force-rebuilt each
time). **Every semantically observable mutant was caught.** The seven that were
not caught are provably equivalent programs:

| mutant | why it is unobservable |
|--------|------------------------|
| `c2CircletoCircle`: `ss_add(B.r, A.r)` → `ss_add(A.r, B.r)` | only changes which NaN payload lands in `r2`; `r2` is consumed solely by `d2 < r2`, and `comiss` is false for *any* NaN, so the `int` result is unchanged |
| `f12`: `ss_mul(1-f, s)` → `ss_mul(s, 1-f)` | only differs when `s` **and** `1-f` are both NaN; `1-f` is NaN only when `h` is NaN, which forces `i == INT_MIN` and hence the `default:` sector, where `t` is never read |
| `f3`: `r = v1 - q*v2` → `r = v1 + q*v2` in the `v2 == INT_MIN` arm | `q == 1` and `-INT_MIN ≡ +INT_MIN (mod 2³²)`, so `r` is bit-identical |
| `f4`: `x.wrapping_add(y)` → `y.wrapping_add(x)` | integer addition is commutative |
| `f13`: `min < g` → `min <= g` | only differs for `+0.0` vs `-0.0`; `min` is read only by `delta = max - min`, and `max - (+0.0)` vs `max - (-0.0)` differ only when `max` is `±0.0`, in which case `delta == 0` and the guard returns the same `{0, 0, max}` |
| `agglom`: drop the `!isnan(f4_r)` guard | `f4` builds `(1023 << 52) \| mantissa` and subtracts `1.0`, so its result is always in `[0, 1)` — never NaN. The guard is dead code in the C too |
| `agglom`: `f5_r as f64` → `f5_r as i32 as f64` | `f5` clears bits 16..31, so its result is always in `0..=0xFFFF` and the signed/unsigned conversions coincide |

Mutants that WERE caught include: every NaN-payload operand ordering that is
observable (`c2Dot`, `lm_dot2`), removing the NaN quieting, both halves of the
`cvttss2si` integer-indefinite emulation, "fixing" the `h < 120.0f && h < 180.0f`
typo in `f11`, each `f3` overflow-avoidance arm and the floor correction's sign,
the xorshift constants and mantissa shift in `f4`, `f7`'s `channels`/`bitdepth`
predicates and its `/8`, `f10`'s row shift and wrapping add, every `f12` sector
permutation, `f13`'s `max`/`delta`/hue-wrap comparisons, `f2`'s AABB/CIRCLE
argument swap, `c2Clampv`'s argument order, `c2AABBtoAABB`'s lane selection and
`|` vs `&`, and the individual `!isnan(...)` guards in `agglom`.
