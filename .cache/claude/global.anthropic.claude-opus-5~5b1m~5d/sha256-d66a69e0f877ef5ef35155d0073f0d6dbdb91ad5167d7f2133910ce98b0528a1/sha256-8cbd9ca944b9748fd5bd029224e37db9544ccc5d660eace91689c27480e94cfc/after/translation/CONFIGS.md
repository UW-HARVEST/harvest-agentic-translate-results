# CONFIGS.md — Phase B configuration surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

There are **no compile-time options** — `grep -c '#if' c_src/src/lib.c` returns
`0`, i.e. the translation unit contains no conditional compilation at all — and
**no runtime configuration struct**. The branching axes are therefore:

| axis | values the C distinguishes | evidence |
|------|---------------------------|----------|
| `A1` `C2_TYPE typeA` | `CIRCLE(0)`, `AABB(1)`, out-of-range | `switch (typeA)` in `f2` |
| `A2` `C2_TYPE typeB` | `CIRCLE(0)`, `AABB(1)`, out-of-range | nested `switch (typeB)` ×2 |
| `A3` shape overlap outcome | overlapping / disjoint / touching (`d2 < r2` strict) | `return d2 < r2` |
| `A4` `f3` sign quadrant | `v1>=0`/`v1<0`/`v1==INT_MIN` × `v2>0`/`v2<0`/`v2==INT_MIN`/`v2==0` | 9-way `if`/`else` ladder |
| `A5` `f3` remainder sign | `r >= 0` vs `r < 0` (floor correction) | trailing `if (r >= 0)` |
| `A6` `cn_rnd_t` seed shape | `{0,0}`, `{max,max}`, one-word-zero, random | `cn_rnd_next` xorshift |
| `A7` `f5` input width | ≤ 0xFFFF vs > 0xFFFF (masks are 16-bit) | `0xAAAA`/`0x5555`… masks |
| `A8` `f7` `channels` | `== 2` vs `!= 2` (incl. `0`, `1`, `3`, huge) | `(channels != 2)` / `(channels == 2)` |
| `A9` `f7` `bitdepth` | `== 32` vs `!= 32` (incl. `0`, `8`, `16`, `24`, `33`) | `(bitdepth != 32)` |
| `A10` `f7` magnitude | small (no overflow) vs `u32`-overflowing | unsigned wrap |
| `A11` `f9` triangle shape | non-degenerate / collinear / coincident points | `1.0f/(dot00*dot11 - dot01*dot01)` |
| `A12` `f10` half-float class | zero, subnormal (`n==0`), normal, Inf/NaN (`n==31`/`63`), negative (`n>=32`) | `m__offset[n]` / `m__exponent[n]` tables |
| `A13` `f11` `s` | `== 0` (early out) vs `!= 0` | `if (s == 0)` |
| `A14` `f11` `h` sector | `[0,60)`, `[60,120)`, `<120 && <180` (**buggy branch, catches all `h<120` incl. negatives**), `[180,240)`, `[240,300)`, `[300,360)`, else | 7-way `if`/`else if` ladder |
| `A15` `f12` `s` | `== 0` (early out) vs `!= 0` | `if (s == 0)` |
| `A16` `f12` `i` | `0`,`1`,`2`,`3`,`4`, `default` | `switch (i)` |
| `A17` `f13` early-out | `delta == 0` ‖ `max == 0` vs neither | `if (delta == 0 \|\| max == 0)` |
| `A18` `f13` which channel is max | `r == max`, `g == max`, else (`b`) | 3-way `if`/`else if`/`else` |
| `A19` `f13` hue wrap | `h < 0` after `h *= 60` vs not | `if (h < 0) h += 360` |
| `A20` float class of any input | normal, `-0.0`, subnormal, `±inf`, `NaN` (incl. distinct payloads) | every float compare / arith |
| `A21` entry-point level | low-level (`c2V`…`c2Dot`, `c2*to*`) / mid (`f2`…`f13`) / aggregate (`agglom`) | `nm -D` surface |

Feature combinations: `translation/Cargo.toml` has **no `[features]`** table, so
the only configuration is the default. (Phase D re-runs the same suite under
`--no-default-features`, which is identical here.)

## Configuration rows

Every row is exercised with **many randomized inputs** (fixed seed
`0x243F6A8885A308D3`, SplitMix64) unless marked *exhaustive* or *fixed*.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random finite `(x, y)`; also `±0.0`, subnormal, `±inf`, `NaN` payloads | [x] |
| 2 | `c2Maxv` | random finite pairs — componentwise max | [x] |
| 3 | `c2Maxv` | one/both components `NaN` (asymmetric ternary), `±inf`, `±0.0` | [x] |
| 4 | `c2Minv` | random finite pairs — componentwise min | [x] |
| 5 | `c2Minv` | one/both components `NaN`, `±inf`, `±0.0` | [x] |
| 6 | `c2Clampv` | `lo <= hi` well-formed box, `a` inside / on edge / outside each side | [x] |
| 7 | `c2Clampv` | inverted box `lo > hi`; and `NaN` in `a`/`lo`/`hi` | [x] |
| 8 | `c2Sub` | random finite; `inf - inf` → `NaN`; `NaN` operands both sides | [x] |
| 9 | `c2Dot` | random finite; `0 * inf` → `NaN`; `NaN` payload propagation; huge values → `inf` | [x] |
| 10 | `c2CircletoCircle` | overlapping circles (random, positive radii) | [x] |
| 11 | `c2CircletoCircle` | disjoint circles; exactly-touching (`d2 == r2`, strict `<` → 0) | [x] |
| 12 | `c2CircletoCircle` | zero radius, negative radius (`r2 = (rA+rB)²` ≥ 0 quirk), `NaN`/`inf` radius | [x] |
| 13 | `c2CircletoAABB` | circle centre inside the box (clamp is identity) | [x] |
| 14 | `c2CircletoAABB` | centre outside — each of the 8 outside regions (edges + corners) | [x] |
| 15 | `c2CircletoAABB` | degenerate box (`min == max`), inverted box (`min > max`), `NaN` coords | [x] |
| 16 | `c2AABBtoAABB` | overlapping boxes (random) | [x] |
| 17 | `c2AABBtoAABB` | disjoint on x only / y only / both; touching edges (strict `<`) | [x] |
| 18 | `c2AABBtoAABB` | `NaN` coordinates → all `<` false → returns `1` | [x] |
| 19 | `f2` | `typeA=CIRCLE, typeB=CIRCLE` — random circle pairs | [x] |
| 20 | `f2` | `typeA=CIRCLE, typeB=AABB` — random circle + box (`c2CircletoAABB(A,B)`) | [x] |
| 21 | `f2` | `typeA=AABB, typeB=CIRCLE` — **swapped-argument path** `c2CircletoAABB(*B, *A)` | [x] |
| 22 | `f2` | `typeA=AABB, typeB=AABB` — random box pairs | [x] |
| 23 | `f3` | `v1 >= 0, v2 > 0` (plain truncating divide, early `return`) | [x] |
| 24 | `f3` | `v1 >= 0, v2 < 0` (`v2 != INT_MIN`) — negative quotient + floor correction | [x] |
| 25 | `f3` | `v1 < 0` (`!= INT_MIN`), `v2 > 0` — floor correction path | [x] |
| 26 | `f3` | `v1 < 0` (`!= INT_MIN`), `v2 < 0` (`!= INT_MIN`) | [x] |
| 27 | `f3` | `v1 == INT_MIN`, `v2 > 0` — the `-(v1+v2)` overflow-avoidance branch | [x] |
| 28 | `f3` | `v1 == INT_MIN`, `v2 < 0` (`!= INT_MIN`) — the `-(v1-v2)` branch | [x] |
| 29 | `f3` | `v2 == INT_MIN` with `v1 >= 0`, `v1 < 0`, `v1 == INT_MIN` (3 sub-cases) | [x] |
| 30 | `f3` | `v1 == ±1`, `v2 == ±1`, `v1 == INT_MAX`, `v2 == INT_MAX`, exact multiples (`r == 0`) | [x] |
| 31 | `f4` | random 128-bit seeds — single call | [x] |
| 32 | `f4` | **stateful sequence**: same seed, 256 successive calls; asserts the mutated `cn_rnd_t` state matches after every step | [x] |
| 33 | `f4` | seed `{0,0}`, `{0,1}`, `{1,0}`, `{u64::MAX, u64::MAX}`, `{u64::MAX, 0}` (fixed) | [x] |
| 34 | `f5` | random `u32` (both ≤ `0xFFFF` and `> 0xFFFF`, high bits discarded) | [x] |
| 35 | `f5` | *exhaustive* over all `0..=0xFFFF`, plus `0xFFFF0000`-style high-bit-only values | [x] |
| 36 | `f7` | `channels == 2`, `bitdepth == 32`, small `blocksize` | [x] |
| 37 | `f7` | `channels == 2`, `bitdepth != 32` (8/16/20/24/33), small `blocksize` | [x] |
| 38 | `f7` | `channels != 2` (0, 1, 3, 8), `bitdepth == 32` | [x] |
| 39 | `f7` | `channels != 2`, `bitdepth != 32` | [x] |
| 40 | `f7` | overflowing magnitudes: `blocksize`/`channels`/`bitdepth` random full `u32` (unsigned wrap + `/8`) | [x] |
| 41 | `f7` | all zeros; `blocksize == 0`; `channels == 0`; `bitdepth == 0`; each at `u32::MAX` | [x] |
| 42 | `f9` | non-degenerate triangle, `p` strictly inside | [x] |
| 43 | `f9` | non-degenerate triangle, `p` outside / on an edge / at a vertex | [x] |
| 44 | `f9` | collinear `p1,p2,p3` (denominator `0` → `invDenom = ±inf`) | [x] |
| 45 | `f9` | all four points identical (`0 * inf` → `NaN`) | [x] |
| 46 | `f9` | coordinates including `±inf`, `NaN` (distinct payloads), `±0.0`, subnormals, huge finite (overflow to `inf`) | [x] |
| 47 | `f9` | fully random `u32`-bit-pattern coordinates (all float classes mixed) | [x] |
| 48 | `f10` | *exhaustive* over **all 65536** `uint16_t` values — covers zero, subnormal (`n==0`), every normal exponent, Inf, NaN, and all negatives (`n>=32`); bitwise-compared | [x] |
| 49 | `f11` | `s == 0` early-out, with `l` random / `±inf` / `NaN` / `±0.0` | [x] |
| 50 | `f11` | `s == -0.0` (still `== 0` in float compare) → early-out | [x] |
| 51 | `f11` | `h ∈ [0,60)` sector, random `s != 0`, random `l` | [x] |
| 52 | `f11` | `h ∈ [60,120)` sector | [x] |
| 53 | `f11` | `h ∈ [120,180)` — the range the C bug renders **unreachable** → final `else` | [x] |
| 54 | `f11` | `h < 0` (negative hue) → hits the buggy third branch `h<120 && h<180` | [x] |
| 55 | `f11` | `h ∈ [180,240)` sector | [x] |
| 56 | `f11` | `h ∈ [240,300)` sector | [x] |
| 57 | `f11` | `h ∈ [300,360)` sector | [x] |
| 58 | `f11` | `h >= 360` → final `else` | [x] |
| 59 | `f11` | `h = ±inf`, `h = NaN` (both payload signs) | [x] |
| 60 | `f11` | `s`/`l` extreme: `s = inf`, `l = inf`, `s`/`l` `NaN`, subnormals, huge (so `fmodf` and `fabsf` see extremes) | [x] |
| 61 | `f11` | fully random `u32`-bit-pattern `src[0..3]` (all classes) | [x] |
| 62 | `f11` | `dest == src` (**aliasing**: same buffer passed for both) | [x] |
| 63 | `f11` | `dest` overlapping `src` at offset ±1 element | [x] |
| 64 | `f12` | `s == 0` early-out (and `s == -0.0`), random `v` incl. `NaN`/`inf` | [x] |
| 65 | `f12` | `i == 0` (`h ∈ [0,60)`) | [x] |
| 66 | `f12` | `i == 1` (`h ∈ [60,120)`) | [x] |
| 67 | `f12` | `i == 2` (`h ∈ [120,180)`) | [x] |
| 68 | `f12` | `i == 3` (`h ∈ [180,240)`) | [x] |
| 69 | `f12` | `i == 4` (`h ∈ [240,300)`) | [x] |
| 70 | `f12` | `i == 5` → `default:` (`h ∈ [300,360)`) | [x] |
| 71 | `f12` | `i` negative (`h < 0`) → `default:`; and `h` exactly `-0.0` | [x] |
| 72 | `f12` | `h/60` beyond `int` range / `±inf` / `NaN` → `cvttss2si` = `INT_MIN` → `default:` | [x] |
| 73 | `f12` | `s`/`v` extremes: `inf`, `NaN`, subnormal, `s > 1`, `s < 0`, `v < 0` | [x] |
| 74 | `f12` | fully random `u32`-bit-pattern `src[0..3]` | [x] |
| 75 | `f12` | `dest == src` aliasing, and ±1-element overlap | [x] |
| 76 | `f13` | `delta == 0` (`r == g == b`, nonzero) → `{0,0,v}` | [x] |
| 77 | `f13` | `max == 0` (black / all ≤ 0) → early out | [x] |
| 78 | `f13` | `r == max` with `g >= b` (hue `[0,60)`) | [x] |
| 79 | `f13` | `r == max` with `g < b` → `h < 0` → `+= 360` wrap | [x] |
| 80 | `f13` | `g == max` (hue around 120) | [x] |
| 81 | `f13` | `b == max` (else branch, hue around 240) | [x] |
| 82 | `f13` | ties: `r == g > b`, `g == b > r`, `r == b > g` (tie-breaking order of the `if` ladder) | [x] |
| 83 | `f13` | components `NaN` (each position, and multiple) — min/max ternary asymmetry | [x] |
| 84 | `f13` | components `±inf` (`inf - inf` → `NaN` delta), `±0.0`, subnormals, negatives | [x] |
| 85 | `f13` | fully random `u32`-bit-pattern `src[0..3]` | [x] |
| 86 | `f13` | `dest == src` aliasing, and ±1-element overlap | [x] |
| 87 | `f11`→`f12`→`f13` | **composed pipeline**: HSL→RGB, RGB→HSV, HSV→RGB round trips chained on the same buffers | [x] |
| 88 | `agglom` | all 33 arguments fully random bit patterns (all float classes, full-range ints) — bitwise `f64` compare | [x] |
| 89 | `agglom` | all-zero arguments | [x] |
| 90 | `agglom` | "sane" arguments: finite coords, valid hues `[0,360)`, `s`/`l`/`v` in `[0,1]`, `channels ∈ {1,2,3}`, `bitdepth ∈ {8,16,24,32}` | [x] |
| 91 | `agglom` | arguments driving each `isnan` guard: `f9` degenerate, `f10_1` = half-NaN, `f11`/`f12`/`f13` NaN-producing | [x] |
| 92 | `agglom` | arguments producing `±inf` contributions (not skipped by the `isnan` guards) | [x] |
| 93 | `agglom` | `f3_1`/`f3_2` at `INT_MIN`/`INT_MAX`/`0`; `f5_1`/`f7_*` at `u32::MAX`; `f4_*` at `u64::MAX`/`0` | [x] |
| 94 | `agglom` | *sane-hue sweep*: `f11_2`/`f12_2`/`f13_*` swept across all 7 `f11` sectors × 6 `f12` `i` values | [x] |

**Binary executable:** the project builds no driver binary — `c_src/CMakeLists.txt`
declares only `add_library(... SHARED src/lib.c)` (no `add_executable`), and
`translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]` (no
`[[bin]]`, no `src/main.rs`). The stdout-comparison item is therefore N/A.

## Verification result

All 94 rows pass, in **both** the debug and the release cdylib:

```
$ ./run_diff_tests.sh
### feature combination: default
  phase_b_lowlevel   15 passed   (rows 1–41)
  phase_b_float      17 passed   (rows 42–87)
  phase_b_agglom      7 passed   (rows 88–94)
  phase_b_stress      9 passed   (high-volume randomized top-up)
  phase_c_errors     44 passed
  phase_d_symbols     5 passed
--- re-running against the RELEASE cdylib ---
  ... identical results
```

Every row is driven with many randomized inputs from a fixed SplitMix64 seed
(`0x243F6A8885A308D3`), plus deterministic boundary values. Two axes are covered
**exhaustively** rather than by sampling:

- `f10`: all 65536 `uint16_t` inputs (rows 48, 24) — bit-compared.
- `f5`: all 65536 low-16-bit values (row 35); since the C masks discard bits
  above bit 15, this is exhaustive over the function's entire effective domain.

`tests/phase_b_stress.rs` adds ~180 million further differential comparisons
(30 M iterations per function in release) to reduce the chance that a rare
value-dependent path went unsampled.

### Comparison strength

All float results are compared with `to_bits()`, i.e. **bit-for-bit**, so
signed zeros and NaN payloads are part of the contract — not just numeric
equality. `f4` additionally asserts the mutated `cn_rnd_t` state matches after
every one of 512 successive calls.
