# CONFIGS.md — Configuration-surface table (valid inputs)

## Axes the C code actually branches on

The library has **no runtime option/flag/mode setters** and **no `#ifdef`** —
`c_src/src/lib.c` contains zero preprocessor conditionals and `CMakeLists.txt`
defines no options. The configuration space is therefore:

**Axis 1 — entry point** (all 22 exported symbols, from the lowest-level vector
helpers up to `gen_ray`; the call hierarchy is
`gen_ray` → `c2CastRay` → `c2Rayto{Circle,AABB,Capsule}` →
`{c2AABBtoAABB, c2AABBtoPoint, c2CircleToPoint, c2RaytoCircle}` →
`{c2V, c2Dot, c2Len, c2Add, c2Sub, c2Mulvs, c2Div, c2Norm, c2Minv, c2Maxv, c2Skew, c2Absv, c2CCW90, c2MulmvT}`).

**Axis 2 — shape "mode" selector**: `c2CastRay`'s `C2_TYPE` argument, the only
enum in the library: `C2_TYPE_CIRCLE=0`, `C2_TYPE_AABB=1`, `C2_TYPE_CAPSULE=2`.
This reinterprets the `const void *B` payload, so it is a genuine mode flag.

**Axis 3 — float value class** per input component, because every branch in the
library is an IEEE comparison whose result changes for these classes:
normal-positive, normal-negative, `+0.0`, `-0.0`, subnormal, `+inf`, `-inf`,
`NaN` (quiet, both sign bits, non-default payload). The ternary
`min`/`max`/`abs` idioms (`a<b?a:b`, `a<0?-a:a`) are **not** `f32::min`/`abs`
and diverge exactly on `NaN` and `-0.0`, so these classes are load-bearing.

**Axis 4 — geometric configuration** (which `if` the raycast body takes):

* circle: line-miss / hit-in-front / hit-behind-origin / hit-beyond-`A.t` /
  tangent / origin-inside;
* AABB: bbox-reject / SAT-reject / each of the 4 winning planes
  (`-x`, `+x`, `-y`, `+y`) / axis-aligned ray / zero-length ray / inverted box;
* capsule: origin-in-slab / origin-in-cap-A / origin-in-cap-B / crossing-axis /
  lateral-entry via cap A / via cap B / side-hit on `+r` face / `-r` face /
  fall-through miss / zero-length axis / zero radius.

**Axis 5 — `A.t` magnitude**: `0`, tiny, `1`, huge, `inf` — it scales `p1` and
the reported `out->t`.

Rows below are the cross-product of these axes **pruned to the combinations the
C actually distinguishes**. Every row is driven through the `.so` exports of
both libraries with **many randomised inputs (fixed seed `0x5EED_C2A1`)**, not a
single hand-picked value, and compared **bit-for-bit** (`to_bits()`), so a NaN
with a different sign or payload fails.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random finite pairs + all special classes (±0, ±inf, subnormal, NaN payloads) | [x] |
| 2 | `c2Dot` | random finite vectors, mixed magnitudes (1e-30 … 1e30) — catches operand-order NaN and rounding-order divergence | [x] |
| 3 | `c2Dot` | one/both components special: ±0, ±inf, NaN — `inf*0` ⇒ NaN, `inf + -inf` ⇒ NaN | [x] |
| 4 | `c2Len` | random finite vectors; overflow-to-inf case (`1e30`); NaN and ±0 inputs | [x] |
| 5 | `c2Add`, `c2Sub` | random finite; `inf + -inf`; NaN in either operand (operand-order sensitive); `-0.0 + 0.0` | [x] |
| 6 | `c2Mulvs` | random finite scalar; scalar `= 0` with `inf` component; scalar `= inf` with `0` component; NaN scalar | [x] |
| 7 | `c2Div` | random finite divisor; divisor `= 0` (⇒ `1/0 = inf`, then `0*inf = NaN`); divisor `= inf`; divisor `= -0.0`; NaN | [x] |
| 8 | `c2Norm` | random finite vectors; **zero vector** (⇒ NaN,NaN); axis-aligned zero-component vector (⇒ NaN,±inf); huge vector (`c2Len` overflows to inf ⇒ `1/inf = 0` ⇒ `0`); NaN | [x] |
| 9 | `c2Minv`, `c2Maxv` | random finite; **equal** components; `+0.0` vs `-0.0` (ternary keeps `b`); NaN in `a`; NaN in `b`; ±inf | [x] |
| 10 | `c2Skew`, `c2CCW90` | random finite; `±0.0` (negation must produce the opposite zero); NaN (sign bit must flip, payload preserved) | [x] |
| 11 | `c2Absv` | random finite; `-0.0` (ternary returns `-0.0`, unlike `fabsf`); `+0.0`; negative NaN (`a<0` false ⇒ sign bit **kept**) | [x] |
| 12 | `c2MulmvT` | random finite `c2m` × `c2v`; rows containing ±inf/NaN; identity and zero matrix | [x] |
| 13 | `c2AABBtoAABB` | overlapping, touching-edge, touching-corner, fully-contained, all 4 separations, inverted box, NaN coordinate | [x] |
| 14 | `c2AABBtoPoint` | inside, exactly on each of the 4 edges (`<`/`>` boundary), outside on each side, NaN point, inverted box | [x] |
| 15 | `c2CircleToPoint` | strictly inside, exactly on boundary (rejects), outside, `r = 0`, `r < 0`, NaN | [x] |
| 16 | `c2RaytoCircle` | direct low-level call: random rays × random circles, mostly-hitting distribution | [x] |
| 17 | `c2RaytoCircle` | hit in front, within `A.t` (the only `return 1` path) — verifies `out->t` and `out->n` bits | [x] |
| 18 | `c2RaytoCircle` | `A.d` **not** normalised (nothing in C requires it) — changes the `t`/`A.t` relation | [x] |
| 19 | `c2RaytoCircle` | `A.t` ∈ {0, 1e-6, 1, 1e6, inf}; `A.t` negative (⇒ `t <= A.t` never true) | [x] |
| 20 | `c2RaytoCircle` | tangent ray (`disc ≈ 0`) and origin-on-circle (`c == 0`) boundary values | [x] |
| 21 | `c2RaytoCircle` | `out->n = c2Norm(impact - p)` with `impact == p` (ray through a zero-radius circle) ⇒ NaN normal | [x] |
| 22 | `c2RaytoAABB` | direct low-level call: random rays × random boxes | [x] |
| 23 | `c2RaytoAABB` | each of the 4 winning-plane branches (`-x`,`+x`,`-y`,`+y`) forced by ray direction | [x] |
| 24 | `c2RaytoAABB` | tie between planes (ray hits an exact corner) — exercises the `>=` chain's first-match order | [x] |
| 25 | `c2RaytoAABB` | axis-aligned ray (`d = (1,0)` / `(0,1)`) ⇒ `da == db` on two planes ⇒ the `d != 0` guard | [x] |
| 26 | `c2RaytoAABB` | ray whose bbox overlaps but whose line misses (SAT `d > 0` path) | [x] |
| 27 | `c2RaytoAABB` | zero-length ray (`A.t == 0`), degenerate box (`min == max`), inverted box | [x] |
| 28 | `c2RaytoAABB` | `A.t` ∈ {0, tiny, 1, huge, inf}; NaN in `A.d` (all four `tN` NaN ⇒ no hit) | [x] |
| 29 | `c2RaytoCapsule` | direct low-level call: random rays × random capsules | [x] |
| 30 | `c2RaytoCapsule` | origin inside the axis slab ⇒ early `return 1`, `out->t == 0` | [x] |
| 31 | `c2RaytoCapsule` | origin inside end-cap A / end-cap B ⇒ early `return 1` | [x] |
| 32 | `c2RaytoCapsule` | crossing the axis (`yAe.x*yAp.x < 0`) with `|yAp.x| < B.r` ⇒ delegates to `c2RaytoCircle` on cap A (`yAp.y < 0`) | [x] |
| 33 | `c2RaytoCapsule` | same, `yAp.y >= 0` ⇒ delegates to cap B | [x] |
| 34 | `c2RaytoCapsule` | `|yAp.x| >= B.r`, computed `y <= 0` ⇒ delegates to cap A circle | [x] |
| 35 | `c2RaytoCapsule` | `|yAp.x| >= B.r`, `y >= yBb.y` ⇒ delegates to cap B circle | [x] |
| 36 | `c2RaytoCapsule` | `0 < y < yBb.y`, `c > 0` ⇒ side hit, `out->n = M.x` | [x] |
| 37 | `c2RaytoCapsule` | `0 < y < yBb.y`, `c <= 0` ⇒ side hit, `out->n = c2Skew(M.y)` | [x] |
| 38 | `c2RaytoCapsule` | axis orientation sweep: `b-a` pointing in all 8 octants + axis-aligned, so `M` covers all sign combinations | [x] |
| 39 | `c2RaytoCapsule` | **reversed** capsule (`b` "below" `a` ⇒ `yBb.y > 0` still, but slab max/min ordering) and `a == b` (zero-length ⇒ NaN `M`) | [x] |
| 40 | `c2RaytoCapsule` | `B.r` ∈ {0, tiny, moderate, huge}; `A.t` ∈ {0, tiny, 1, huge} | [x] |
| 41 | `c2RaytoCapsule` | `yAe.x == yAp.x` in the else-branch ⇒ unguarded `/0` in `t` | [x] |
| 42 | `c2CastRay` mode `C2_TYPE_CIRCLE` | random rays/circles through the dispatcher, payload read as `c2Circle` | [x] |
| 43 | `c2CastRay` mode `C2_TYPE_AABB` | random rays/boxes through the dispatcher | [x] |
| 44 | `c2CastRay` mode `C2_TYPE_CAPSULE` | random rays/capsules through the dispatcher | [x] |
| 45 | `c2CastRay` | payload buffer **larger** than the shape (over-read must not happen) and each mode reading the *same* 20-byte buffer, so the mode flag alone changes the result | [x] |
| 46 | `gen_ray` | full end-to-end pipeline, random 16-float parameter vectors, "generic" distribution | [x] |
| 47 | `gen_ray` | tuned distribution where the ray reliably **hits all three** shapes ⇒ `ret == 7`, all three `cast*` written | [x] |
| 48 | `gen_ray` | each single-shape hit bit isolated: `ret` ∈ {1, 2, 4} and each pair {3, 5, 6} and {0} | [x] |
| 49 | `gen_ray` | `mp == ray.p` ⇒ `c2Norm(0)` ⇒ NaN ray direction and NaN `ray.t` through all three shapes | [x] |
| 50 | `gen_ray` | special float classes injected into each of the 16 float parameters in turn (±0, ±inf, NaN, subnormal, `f32::MIN/MAX`) | [x] |
| 51 | `gen_ray` | inverted `bb`, zero-radius circle, zero-radius capsule, zero-length capsule, all combined | [x] |
| 52 | `gen_ray` | very large / very small coordinate magnitudes (1e±30) so `c2Dot` overflows to ±inf mid-pipeline | [x] |
| 53 | all 22 exports | **struct-ABI parity sweep**: `c2Ray`/`c2Capsule` (20 B, MEMORY class) and `c2Circle`/`c2AABB`/`c2m` (12/16 B, SSE class) passed by value, with sentinel bit patterns in every field, to prove the Rust `extern "C"` classification matches GCC's | [x] |

## Status

All 53 rows pass. Every row is driven through `dlopen`/`dlsym` on both `.so`s
with many randomised inputs (fixed seed `0x5EEDC2A1`) and compared with
`to_bits()`, so a NaN with a different sign or payload, or a `-0.0` where the C
produced `+0.0`, fails the test.

Test files: `tests/phase_b_vector.rs` (rows 1–12),
`tests/phase_b_predicates.rs` (rows 13–15 + harness self-check),
`tests/phase_b_circle.rs` (16–21), `tests/phase_b_aabb.rs` (22–28),
`tests/phase_b_capsule.rs` (29–41), `tests/phase_b_dispatch.rs` (42–45),
`tests/phase_b_gen_ray.rs` (46–53).

### Branch coverage is measured, not assumed

For the three raycast functions the tests *replay the C's own control flow* using
the C library's exported primitives (`c2Norm`, `c2MulmvT`, `c2AABBtoPoint`, …) to
classify which branch each input takes, then assert every branch was reached.
Observed counts from the last run:

* `c2RaytoCircle` — all 4 branches (`disc<0`, `t<0`, `t>A.t`, hit).
* `c2RaytoAABB` — all 6 (bbox reject, SAT reject, and each of the `-x`/`+x`/`-y`/`+y`
  winning planes).
* `c2RaytoCapsule` — all 10 (slab early-return, cap-A early, cap-B early, lateral
  delegate A/B, computed delegate A/B, side hit `M.x`, side hit `c2Skew(M.y)`,
  fall-through).
* `gen_ray` — all 8 return codes `0..=7` observed.
* `c2CastRay` — all 3 modes, plus 7 338 cases where the mode flag alone changed
  the result on an identical payload.

### Harness self-check (negative control)

`harness_self_check` in `tests/phase_b_predicates.rs` asserts the two `.so` files
are distinct, that the C one comes from `c_src/build`, that `Diff::finish` really
panics on a recorded divergence, and that `bits_eq` rejects `+0.0`/`-0.0` and
differently-signed NaNs. Without it, a harness bug could make every other test
vacuous.

### Feature combinations

`Cargo.toml` declares no `[features]` table, so `{default, --no-default-features}`
is the complete cross-product. `tests/feature_matrix.sh` extracts the feature
list from `Cargo.toml` (so it keeps working if features are added), builds the
power set, and for each combination re-checks symbol parity **and** runs the full
suite. Last run: 2/2 combinations, 22/22 symbols, 83 tests passed, 0 failed.
