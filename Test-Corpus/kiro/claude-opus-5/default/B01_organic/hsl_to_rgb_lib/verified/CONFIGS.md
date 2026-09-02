# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

### Public entry points

`c_src/include/lib.h` in full:

```c
void hsl_to_rgb(float *dest, const float *src);
```

One entry point. It is simultaneously the highest- and lowest-level public
function: there is no convenience wrapper layered over an internal core, no
`static` helper, and no second `.c` file in `CMakeLists.txt`. `nm -D` confirms a
single exported non-libc symbol. So "exercise the low-level entry points, not
just the wrappers" is satisfied by construction — every row calls `hsl_to_rgb`
directly through the `.so`.

### Runtime options / modes / flags

**None.** There is no context struct, no init call, no setter, no global, no
`#ifdef`, and no `#if` in either the header or the source. `grep -c '#if' `
returns 0. All behavioural variation is driven purely by the *values* in `src`.

### Input shapes the code special-cases

Both parameters are fixed-arity `float[3]`, so there is no size/count/width/
element-type/byte-order/format axis. The axes are the value-domain branches the
compiled code actually takes:

- **Axis S — saturation path** (`if (s == 0)`, `ucomiss`; note `-0.0` and `+0.0`
  both compare equal, NaN compares unordered → false):
  `S0` = `s` is `±0.0` (fast path), `S1` = `s` is anything else (slow path).
- **Axis H — which of the seven hue arms is selected**. Derived from lines
  19–43 *as written*, including the arm-3 typo (`h < 120.0f` where the pattern
  implies `h >= 120.0f`), which makes arm 3 reachable only for `h < 0` and
  leaves `[120,180)` to the terminal `else`:
  `H1` = `[0,60)` → `(c+m, x+m, m)`
  `H2` = `[60,120)` → `(x+m, c+m, m)`
  `H3` = `h < 0` → `(m, c+m, x+m)`  ← arm 3, reached only by negative hue
  `H4` = `[180,240)` → `(m, x+m, c+m)`
  `H5` = `[240,300)` → `(x+m, m, c+m)`
  `H6` = `[300,360)` → `(c+m, m, x+m)`
  `H7` = `[120,180)` ∪ `[360,inf)` ∪ `NaN` → `(m, m, m)` (terminal `else`)
- **Axis B — hue exactly on a branch threshold**: `0, 60, 120, 180, 240, 300,
  360` and their `nextafter` neighbours on both sides. `>=` is inclusive and `<`
  exclusive, so each threshold belongs to the arm above it.
- **Axis L — lightness regime** feeding `c = (1-|2l-1|)*s` and `m = l - 0.5c`:
  `l = 0`, `l ∈ (0,0.5)`, `l = 0.5` (`|2l-1| = 0`, so `c = s` exactly),
  `l ∈ (0.5,1)`, `l = 1`, `l < 0`, `l > 1`.
- **Axis F — the `fmodf(h/60, 2)` reduction**: `h/60` small, `h/60` large enough
  that `fmodf` performs many reductions, `h/60` exactly an even/odd integer
  (`x = 0` or `x = c`), and `h/60` subnormal.
- **Axis N — non-finite and edge bit patterns** in each of the three slots
  independently: `±0.0`, subnormal, `±inf`, quiet NaN (both signs, several
  payloads), signalling NaN, `±f32::MAX`. These matter because `addss`/`subss`/
  `mulss`/`divss` return the *destination* operand's NaN when both operands are
  NaN, while `fabsf` (`andps`) clears the sign bit without quieting — so output
  NaN sign/payload is operand-order-sensitive.
- **Axis A — `dest`/`src` aliasing**: disjoint, fully overlapping, and partially
  overlapping buffers.

### Pruning

The cross-product S × H × B × L × F × N × A is pruned to the combinations the
compiled code actually distinguishes. In particular `S0` short-circuits before
`h` is used at all, collapsing all of H/B/F under `S0` to a single row.

## Configuration-surface table

Every row is driven with **many randomized inputs** (fixed-seed xorshift64\*
PRNG, `RNG_SEED = 0x5DEECE66D`, `ITERS = 20000` samples per row unless the row's
domain is finite and enumerated exhaustively), and asserts the three output
`f32`s match **bit-for-bit** (`to_bits()`), not approximately.

| #  | entry point(s) | configuration (options set + input shape) | iters | test | [x] |
|----|----------------|-------------------------------------------|-------|------|-----|
| 1  | `hsl_to_rgb` | `S0`: `s = +0.0`, `h` and `l` uniformly random over all finite `f32` | 20000 | `cfg_row01_s_plus_zero_random_h_l` | [x] |
| 2  | `hsl_to_rgb` | `S0`: `s = -0.0`, `h` and `l` random (fast path must still trigger) | 20000 | `cfg_row02_s_minus_zero_random_h_l` | [x] |
| 3  | `hsl_to_rgb` | `S0`: `s = ±0.0`, `l` drawn from the edge-pattern pool (`±inf`, NaNs, sNaN, subnormals, `±MAX`) — fast path copies `l` verbatim | exhaustive | `cfg_row03_s_zero_edge_lightness` | [x] |
| 4  | `hsl_to_rgb` | `S1`+`H1`: `h ∈ [0,60)` random, `s ∈ (0,1]` random, `l ∈ [0,1]` random | 20000 | `cfg_row04_arm1_hue_0_60` | [x] |
| 5  | `hsl_to_rgb` | `S1`+`H2`: `h ∈ [60,120)` random, `s ∈ (0,1]`, `l ∈ [0,1]` | 20000 | `cfg_row05_arm2_hue_60_120` | [x] |
| 6  | `hsl_to_rgb` | `S1`+`H7`: `h ∈ [120,180)` random — the range the arm-3 typo orphans, output must be flat grey `(m,m,m)` | 20000 | `cfg_row06_arm7_hue_120_180_orphaned` | [x] |
| 7  | `hsl_to_rgb` | `S1`+`H4`: `h ∈ [180,240)` random, `s ∈ (0,1]`, `l ∈ [0,1]` | 20000 | `cfg_row07_arm4_hue_180_240` | [x] |
| 8  | `hsl_to_rgb` | `S1`+`H5`: `h ∈ [240,300)` random, `s ∈ (0,1]`, `l ∈ [0,1]` | 20000 | `cfg_row08_arm5_hue_240_300` | [x] |
| 9  | `hsl_to_rgb` | `S1`+`H6`: `h ∈ [300,360)` random, `s ∈ (0,1]`, `l ∈ [0,1]` | 20000 | `cfg_row09_arm6_hue_300_360` | [x] |
| 10 | `hsl_to_rgb` | `S1`+`H3`: `h < 0` random (arm 3, only reachable via negative hue), `s ∈ (0,1]`, `l ∈ [0,1]` | 20000 | `cfg_row10_arm3_negative_hue` | [x] |
| 11 | `hsl_to_rgb` | `S1`+`H7`: `h ∈ [360, 1e9]` random (above the wheel), `s ∈ (0,1]`, `l ∈ [0,1]` | 20000 | `cfg_row11_arm7_hue_above_360` | [x] |
| 12 | `hsl_to_rgb` | `S1`+`B`: `h` exactly on each threshold `{0,60,120,180,240,300,360}` and at `nextafter(t, ±inf)`, crossed with random `s ∈ (0,1]`, `l ∈ [0,1]` | 21×2000 | `cfg_row12_hue_threshold_boundaries` | [x] |
| 13 | `hsl_to_rgb` | `S1`+`L`: `l = 0.5` exactly (`\|2l-1\| = 0` → `c = s`, `m = l - 0.5s`), `h` random over `[0,360)`, `s ∈ (0,1]` | 20000 | `cfg_row13_lightness_exactly_half` | [x] |
| 14 | `hsl_to_rgb` | `S1`+`L`: `l ∈ {0.0, 1.0}` exactly (`c = 0`, `m = l`), random `h`, `s` | 2×10000 | `cfg_row14_lightness_at_endpoints` | [x] |
| 15 | `hsl_to_rgb` | `S1`+`L`: `l ∈ (0,0.5)` random (lower half), random `h ∈ [0,360)`, `s ∈ (0,1]` | 20000 | `cfg_row15_lightness_lower_half` | [x] |
| 16 | `hsl_to_rgb` | `S1`+`L`: `l ∈ (0.5,1)` random (upper half), random `h ∈ [0,360)`, `s ∈ (0,1]` | 20000 | `cfg_row16_lightness_upper_half` | [x] |
| 17 | `hsl_to_rgb` | `S1`+`L`: `l` outside `[0,1]` (`l ∈ [-100,0) ∪ (1,100]`) → negative `c`, out-of-gamut output | 20000 | `cfg_row17_lightness_outside_unit` | [x] |
| 18 | `hsl_to_rgb` | `S1`: `s` outside `(0,1]` (`s ∈ [-100,0) ∪ (1,100]`) → out-of-gamut `c` | 20000 | `cfg_row18_saturation_outside_unit` | [x] |
| 19 | `hsl_to_rgb` | `S1`+`F`: `h/60` exactly an even integer (`h = 120k`) → `fmodf = 0` → `x = 0`; and exactly odd (`h = 60+120k`) → `fmodf = ±1` → `x = c` | exhaustive over k | `cfg_row19_fmod_integer_multiples` | [x] |
| 20 | `hsl_to_rgb` | `S1`+`F`: `\|h\|` huge (`1e15 … f32::MAX`), so `fmodf(h/60, 2)` needs a long reduction; both signs | 20000 | `cfg_row20_fmod_huge_hue` | [x] |
| 21 | `hsl_to_rgb` | `S1`+`F`: `h` subnormal / tiny (`±1e-45 … ±1e-30`) → `h/60` subnormal or `±0` | 20000 | `cfg_row21_fmod_subnormal_hue` | [x] |
| 22 | `hsl_to_rgb` | `S1`+`N`: `h` from the edge-pattern pool (`±inf`, qNaN ±/payloads, sNaN, subnormal, `±MAX`) × random `s ∈ (0,1]`, `l ∈ [0,1]` | pool×2000 | `cfg_row22_edge_hue_patterns` | [x] |
| 23 | `hsl_to_rgb` | `S1`+`N`: `s` from the edge-pattern pool (excluding `±0`, which is row 3) × random `h`, `l` | pool×2000 | `cfg_row23_edge_saturation_patterns` | [x] |
| 24 | `hsl_to_rgb` | `S1`+`N`: `l` from the edge-pattern pool × random `h`, `s ∈ (0,1]` — the row that exposes NaN-sign/operand-order bugs, because `c`/`x` get sign 0 from `andps` while `m` keeps `l`'s sign | pool×2000 | `cfg_row24_edge_lightness_patterns` | [x] |
| 25 | `hsl_to_rgb` | `S1`+`N`: full cross-product of the edge-pattern pool in **all three** slots simultaneously | exhaustive (pool³) | `cfg_row25_edge_pattern_cross_product` | [x] |
| 26 | `hsl_to_rgb` | unconstrained fuzz: all three inputs are uniformly random 32-bit patterns (any class, any exponent, any NaN payload) | 300000 | `cfg_row26_uniform_bitpattern_fuzz` | [x] |
| 27 | `hsl_to_rgb` | `A`: `dest == src` (full aliasing), random inputs over both `S0` and `S1` | 20000 | `cfg_row27_alias_dest_equals_src` | [x] |
| 28 | `hsl_to_rgb` | `A`: `dest == src + 1` (partial forward overlap), random inputs | 20000 | `cfg_row28_alias_dest_offset_plus_one` | [x] |
| 29 | `hsl_to_rgb` | `A`: `dest == src - 1` (partial backward overlap), random inputs | 20000 | `cfg_row29_alias_dest_offset_minus_one` | [x] |
| 30 | `hsl_to_rgb` | over-provisioned buffers: 8-float `dest` pre-filled with a sentinel, asserting exactly `dest[0..3]` is written and `dest[3..8]` is untouched by both libraries (no over-write on either path) | 20000 | `cfg_row30_no_out_of_bounds_write` | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one feature configuration to verify: the default. `--no-default-features` is
equivalent to the default here (there is no `default` feature and no optional
dependency). Both are nonetheless run by `verify.sh` to discharge the Phase D
requirement, and `verify.sh` enumerates the `[features]` power set
mechanically from `Cargo.toml` so it stays correct if features are ever added.

There is, however, a second real configuration axis that is *not* a cargo
feature: **the build profile**. `debug` and `release` differ in whether rustc's
UB checks are compiled in, which is observable across the FFI boundary on the
null-pointer rows (see the defect note at the end of `ERRORS.md`). `verify.sh`
therefore runs the whole suite under both profiles, and `mutation_check.sh`
mutates against both.

## Defect found by row 24

Row 24 (`l` from the edge-pattern pool) FAILED on the first run: 7198 of 50000
inputs diverged, e.g.

```
input: h=0xc3a46f8d(-328.8715) s=0x3f5c1fa6(0.8598579) l=0xffc00000(-NaN)
C   : [0xffc00000, 0x7fc00000, 0x7fc00000]
Rust: [0xffc00000, 0x7fc00000, 0xffc00000]   <-- third channel sign bit
```

Cause: in hue arms 3, 4, 5 and 6 the translation computed the third channel as
`add(m, x)` / `add(m, c)`, but the compiled C emits `movss xmm0, <x|c>` followed
by `addss xmm0, m` — i.e. `x`/`c` is the `addss` **destination** operand. SSE
returns the destination operand's NaN when both operands are NaN, and here both
*are* NaN with different sign bits: `fabsf` is a bare `andps` so `c` and `x` come
out sign-positive, while `m = l - 0.5*c` re-propagates `l` and keeps `l`'s sign.
Fixed in `src/lib.rs` by matching the disassembly. Regression-guarded by
`mutation_check.sh` mutations 1-4.

