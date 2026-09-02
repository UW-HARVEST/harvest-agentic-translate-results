# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from the branch structure of `c_src/src/lib.c`. There are
no runtime option structs, no `#ifdef`s, and no Cargo features — the library is
a pure-function collection, so the "configuration" axes are:

* **entry-point level**: the low-level leaf helpers (`c2V`, `c2Maxv`, `c2Minv`,
  `c2Clampv`, `c2Sub`, `c2Dot`), the mid-level shape tests
  (`c2CircletoCircle`, `c2CircletoAABB`, `c2AABBtoAABB`), the dispatcher (`f2`),
  the standalone kernels (`f3`, `f4`, `f5`, `f7`, `f9`, `f10`, `f11`, `f12`,
  `f13`), and the one-shot convenience wrapper (`agglom`). All 20 are tested
  directly through the `.so`, not only `agglom`.
* **enum / mode axis**: `C2_TYPE` for `f2` (`typeA` × `typeB`).
* **input-shape axis**: sign and magnitude classes, zero / negative-zero,
  subnormal, `INT_MIN`, unsigned-wrap magnitudes, the six hue sectors, the six
  `switch` arms, the half-float exponent classes, and NaN/inf.

Every row is exercised with **many randomized inputs from a fixed seed**
(`SplitMix64`, seed `0x243F6A8885A308D3`), not one hand-picked value, and both
`.so`s are compared **bit-for-bit** (`to_bits()`), so NaN payload and `-0.0`
differences are caught.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| **C1** | `c2V` | random finite f32 pairs (round-trip of both fields) | [x] |
| **C2** | `c2V` | non-finite: ±0.0, ±inf, quiet/signalling NaN with varied payloads | [x] |
| **C3** | `c2Maxv`, `c2Minv` | random finite pairs; both orderings (`a.x>b.x` and `a.x<b.x`) | [x] |
| **C4** | `c2Maxv`, `c2Minv` | tie case `a == b`, and `+0.0` vs `-0.0` (compare is false → picks `b`) | [x] |
| **C5** | `c2Maxv`, `c2Minv` | one operand NaN (compare false → picks `b` unconditionally), both NaN | [x] |
| **C6** | `c2Clampv` | `a` inside `[lo,hi]`; `a` below `lo`; `a` above `hi`; inverted range `lo > hi` | [x] |
| **C7** | `c2Clampv` | NaN in `a` / `lo` / `hi`; ±inf bounds | [x] |
| **C8** | `c2Sub` | random finite pairs; `inf - inf` → NaN; NaN operands (src1 selection) | [x] |
| **C9** | `c2Dot` | random finite pairs (checks the mul/add operand order for rounding) | [x] |
| **C10** | `c2Dot` | operands where **both** products are NaN with different payloads — pins the `ADDSS` src1-vs-src2 NaN selection | [x] |
| **C11** | `c2Dot` | overflow to ±inf (`3e38 * 3e38`), and `inf + (-inf)` → NaN | [x] |
| **C12** | `c2CircletoCircle` | overlapping circles (`d2 < r2`) → 1 | [x] |
| **C13** | `c2CircletoCircle` | disjoint circles → 0; exactly touching (`d2 == r2`, strict `<`) → 0 | [x] |
| **C14** | `c2CircletoCircle` | negative radius (`A.r + B.r < 0`, squared so `r2 > 0`); zero radius | [x] |
| **C15** | `c2CircletoCircle` | NaN / inf coordinates and radii | [x] |
| **C16** | `c2CircletoAABB` | circle centre **inside** the box (`L == A.p` → `d2 == 0`) | [x] |
| **C17** | `c2CircletoAABB` | centre outside on each of the 8 sides/corners (clamp active on x, y, both) | [x] |
| **C18** | `c2CircletoAABB` | inverted box (`min > max`) — `c2Clampv` still runs, no validation | [x] |
| **C19** | `c2CircletoAABB` | NaN in centre / radius / box bounds | [x] |
| **C20** | `c2AABBtoAABB` | overlapping; disjoint on each of the 4 axes independently (`d0..d3`) | [x] |
| **C21** | `c2AABBtoAABB` | edge-touching (`B.max.x == A.min.x`, strict `<` → still "collide") | [x] |
| **C22** | `c2AABBtoAABB` | all-NaN box → all 4 compares false → returns `1` | [x] |
| **C23** | `f2` | `typeA=CIRCLE(0)`, `typeB=CIRCLE(0)` → `c2CircletoCircle(*A, *B)`; random circles | [x] |
| **C24** | `f2` | `typeA=CIRCLE(0)`, `typeB=AABB(1)` → `c2CircletoAABB(*A, *B)`; random circle+box | [x] |
| **C25** | `f2` | `typeA=AABB(1)`, `typeB=CIRCLE(0)` → **argument-swapped** `c2CircletoAABB(*B, *A)`; verifies the swap | [x] |
| **C26** | `f2` | `typeA=AABB(1)`, `typeB=AABB(1)` → `c2AABBtoAABB(*A, *B)`; random boxes | [x] |
| **C27** | `f2` | aliasing: `A == B` (same pointer) for each of the 4 valid combinations | [x] |
| **C28** | `f3` | `v1 >= 0, v2 > 0` (fast path `v1 / v2`) — random magnitudes incl. `1`, `INT_MAX` | [x] |
| **C29** | `f3` | `v1 >= 0, v2 < 0` (`v2 != INT_MIN`) — negative-quotient correction path | [x] |
| **C30** | `f3` | `v1 < 0` (`!= INT_MIN`), `v2 > 0` | [x] |
| **C31** | `f3` | `v1 < 0` (`!= INT_MIN`), `v2 < 0` (`!= INT_MIN`) | [x] |
| **C32** | `f3` | exact multiples (`r == 0`) vs inexact (`r != 0`) in every sign quadrant — selects the `q` vs `q ± 1` floor correction | [x] |
| **C33** | `f3` | `|v1| < |v2|` (quotient 0, floor → `-1`) in every sign quadrant | [x] |
| **C34** | `f3` | full random sweep over all four quadrants, cross-checked against the C | [x] |
| **C35** | `f4` | random 128-bit states, **single** call (one `cn_rnd_next` step) | [x] |
| **C36** | `f4` | random states, **long chains** (1000 successive calls on the same handle) — verifies the in-place state mutation, not just the return value | [x] |
| **C37** | `f4` | boundary states: `{0,0}`, `{0,1}`, `{1,0}`, `{u64::MAX,u64::MAX}`, `{1<<63, 1}` | [x] |
| **C38** | `f4` | the mutated `cn_rnd_t` is read back and compared byte-for-byte after each call | [x] |
| **C39** | `f5` | random `u32` with only low 16 bits set (the "intended" domain) | [x] |
| **C40** | `f5` | random full-width `u32` (high 16 bits are silently dropped) | [x] |
| **C41** | `f5` | exhaustive over all 65536 low-16-bit values | [x] |
| **C42** | `f7` | `channels == 2` (selects the 2nd **and** 3rd summand, zeroes the 1st), `bitdepth == 32` | [x] |
| **C43** | `f7` | `channels == 2`, `bitdepth != 32` (the `bitdepth + 1` sub-term activates) | [x] |
| **C44** | `f7` | `channels != 2` (1, 3, 8, …) — only the 1st summand; `bitdepth == 32` and `!= 32` | [x] |
| **C45** | `f7` | `channels == 0` (the `channels * (channels != 2)` product is 0) | [x] |
| **C46** | `f7` | realistic FLAC shapes: blocksize ∈ {1, 16, 4096, 65535}, bitdepth ∈ {8,16,24,32} × channels ∈ {1,2,8} | [x] |
| **C47** | `f7` | unsigned-wrap magnitudes (`u32::MAX`, `1<<31`, `1<<16`) in each of the 3 args | [x] |
| **C48** | `f9` | non-degenerate triangle, `p` **inside** → `u,v` in `[0,1]` | [x] |
| **C49** | `f9` | `p` outside the triangle (negative / `>1` barycentrics) | [x] |
| **C50** | `f9` | `p` exactly at `p1`, `p2`, `p3` (barycentric `(0,0)`, `(0,1)`, `(1,0)`) | [x] |
| **C51** | `f9` | degenerate: `p1 == p2 == p3`, and collinear `p1,p2,p3` → `invDenom = ±inf` | [x] |
| **C52** | `f9` | fully random f32 quadruples (16 random floats), incl. huge/tiny magnitudes → catches the mul/add operand-order and rounding differences | [x] |
| **C53** | `f9` | NaN in any of the 8 coordinates — pins NaN payload through 5 dot products, a subtract, a divide and 4 multiplies | [x] |
| **C54** | `f10` | **exhaustive** over all 65536 `uint16_t` inputs, compared bit-for-bit | [x] |
| **C55** | `f10` | explicit sub-classes verified inside the exhaustive sweep: `n=0` (zero+subnormal half), `n=1..30` (normal), `n=31` (half inf/NaN), `n=32` (negative zero/subnormal), `n=33..62` (negative normal), `n=63` (negative inf/NaN) | [x] |
| **C56** | `f11` | `s == 0` early-out (`+0.0` and `-0.0`), random `h`, `l` | [x] |
| **C57** | `f11` | hue sector `[0,60)` — incl. `h == 0.0`, `h == -0.0` (which passes `h >= 0.0f`) | [x] |
| **C58** | `f11` | hue sector `[60,120)` | [x] |
| **C59** | `f11` | hue in `[120,180)` — falls through to the final `else` because of the `h < 120.0f && h < 180.0f` typo | [x] |
| **C60** | `f11` | hue sector `[180,240)` | [x] |
| **C61** | `f11` | hue sector `[240,300)` | [x] |
| **C62** | `f11` | hue sector `[300,360)` | [x] |
| **C63** | `f11` | `h < 0`, `h >= 360` (incl. huge `h` feeding `fmodf`) → final `else` | [x] |
| **C64** | `f11` | `l` outside `[0,1]` and `s` outside `[0,1]` (no clamping in C) | [x] |
| **C65** | `f11` | `h`/`s`/`l` NaN and ±inf | [x] |
| **C66** | `f11` | fully random f32 triples over the whole f32 range | [x] |
| **C67** | `f11` | `dest` aliases `src` (same buffer passed twice) — exercises the write-ordering | [x] |
| **C68** | `f12` | `s == 0` early-out, random `h`, `v` | [x] |
| **C69** | `f12` | `i == 0` (`h ∈ [0,60)`) | [x] |
| **C70** | `f12` | `i == 1` (`h ∈ [60,120)`) | [x] |
| **C71** | `f12` | `i == 2` (`h ∈ [120,180)`) | [x] |
| **C72** | `f12` | `i == 3` (`h ∈ [180,240)`) | [x] |
| **C73** | `f12` | `i == 4` (`h ∈ [240,300)`) | [x] |
| **C74** | `f12` | `i == 5` and `i >= 6` and `i < 0` → `default:` arm | [x] |
| **C75** | `f12` | `h` at the exact sector boundaries `0,60,120,180,240,300,360` and just below each (`nextafter`) | [x] |
| **C76** | `f12` | `h` so large that `(int)floorf(h/60)` is out of `int` range (`1e30`, `-1e30`, `inf`) — the `cvttss2si` `INT_MIN` case | [x] |
| **C77** | `f12` | `h`/`s`/`v` NaN | [x] |
| **C78** | `f12` | fully random f32 triples over the whole f32 range | [x] |
| **C79** | `f12` | `dest` aliases `src` | [x] |
| **C80** | `f13` | `r` is the max (first `==` branch), with `g > b` and `g < b` (positive and negative hue before correction) | [x] |
| **C81** | `f13` | `g` is the max (second branch) | [x] |
| **C82** | `f13` | `b` is the max (final `else`) | [x] |
| **C83** | `f13` | ties: `r == g == max`, `r == b == max`, `g == b == max` — the `==` chain order decides | [x] |
| **C84** | `f13` | `delta == 0` (`r == g == b`), incl. all-zero and all-negative | [x] |
| **C85** | `f13` | `max == 0` with `delta != 0` (all-negative input, e.g. `{-1,-2,0}`) | [x] |
| **C86** | `f13` | `h < 0` before the `+= 360` correction (exercised by C80 with `g < b`) | [x] |
| **C87** | `f13` | values outside `[0,1]`, and ±inf (`delta = inf - -inf`) | [x] |
| **C88** | `f13` | NaN in any position — every `<`/`>`/`==` compare is false, selects specific branches | [x] |
| **C89** | `f13` | fully random f32 triples over the whole f32 range | [x] |
| **C90** | `f13` | `dest` aliases `src` | [x] |
| **C91** | `agglom` | all 33 args random over the **full** bit range of their types (u32/u16/u64 uniform, f32 from random bits incl. NaN/inf/subnormal) | [x] |
| **C92** | `agglom` | all 33 args "realistic": circles/boxes in `[-10,10]`, hue `[0,360)`, s/l/v `[0,1]`, blocksize/bitdepth/channels FLAC-legal | [x] |
| **C93** | `agglom` | targeted: `f3_2 == 0`, `f3_1 == INT_MIN`, `f3_2 == INT_MIN` combinations | [x] |
| **C94** | `agglom` | targeted: `f4` state `{0,0}`; `f10_1` sweeping the half inf/NaN encodings; `f7` args forcing `u32` wrap | [x] |
| **C95** | `agglom` | targeted: each of `f11`/`f12`/`f13` sub-triples set to `s == 0` / NaN so the `isnan` filters fire (and confirm `±inf` is *not* filtered) | [x] |
| **C96** | `agglom` | degenerate `f9` triangle inside `agglom` → `inf` propagates into the `f64` accumulator un-filtered | [x] |
| **C97** | composed pipeline | `c2Sub` → `c2Dot` → compare, driven through `f2` with random data (the real consumer path, not per-wrapper) | [x] |
| **C98** | composed pipeline | `c2Minv`/`c2Maxv` → `c2Clampv` → `c2Sub` → `c2Dot` chained across the FFI boundary, feeding each stage's C output into the next Rust call and vice-versa | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table and no optional
dependencies, so the only build configuration is the default one. Verified
mechanically:

```
$ python3 -c "import tomllib;print(tomllib.load(open('Cargo.toml','rb')).get('features'))"
None
```

The full matrix is therefore `{default}` = `{--no-default-features}` =
`{--all-features}`. All three are run by `phase_d.sh`, and because the crate
has no features the only *real* remaining configuration axis is the build
profile, so `phase_d.sh` also runs the whole matrix under **both** `debug` and
`release` (6 runs total). That axis is not cosmetic: `debug` enables Rust's
overflow checks, which would panic on any place the C relies on wrapping
arithmetic (`f3`'s `INT_MIN` paths, `f7`'s `u32` products, `f10`'s table add)
if those had been translated with plain operators instead of `wrapping_*`.
