# CONFIGS.md — configuration surface table (valid inputs)

## Axes the C actually branches on

Derived from `c_src/src/lib.c` (`switch` / `if` / loop-bound greps) — the library
has **no** `#ifdef`, no globals, and no init call, so every axis is a runtime
argument.

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `C2_TYPE` of shape A | `CIRCLE`(0), `AABB`(1), `CAPSULE`(2) | `c2MakeProxy` l.114, `c2Collided` l.577 |
| `C2_TYPE` of shape B | `CIRCLE`, `AABB`, `CAPSULE` | `c2Collided` l.579/591/603 |
| proxy vertex count implied by type | 1 (circle), 4 (AABB), 2 (capsule) | `c2MakeProxy` |
| proxy radius implied by type | `c->r` (circle), `0` (AABB), `c->r` (capsule) | `c2MakeProxy` |
| `c2GJK use_radius` | `0` (raw Minkowski distance) vs `!= 0` (shrink by `rA+rB`) | l.482 |
| `c2GJK ax_ptr` / `bx_ptr` | `NULL` (identity) vs a real `c2x` (translation + rotation) | l.368/372 |
| `c2GJK outA` / `outB` / `iterations` | `NULL` vs a real out-pointer | l.510/512/514 |
| `c2GJK cache` | `NULL`; `count==0`; warm cache with `count` 1/2/3 | l.383/384 |
| `c2Simplex.count` (direct low-level entry) | 1, 2, 3, and out-of-range | `c22`/`c23`/`c2D`/`c2L`/`c2Witness`/`c2GJKSimplexMetric` |
| `c22` region | `v<=0` (vertex A) / `u<=0` (vertex B) / interior edge | l.191/195/200 |
| `c23` region | 7 branches: 3 vertex regions, 3 edge regions, 1 interior | l.222-261 |
| `c2D` orientation | `count==1`; `count==2` with `det>0` (`c2Skew`) vs `det<=0` (`c2CCW90`) | l.284-291 |
| `c2Support` count | 1, 2, 4, 8 (and 0 / >8 → ERRORS.md) | l.298 |
| geometric relation | disjoint far / disjoint near / touching / overlapping / identical / one inside the other | drives `hit`, the `dist>rA+rB` test, iteration count |
| shape degeneracy | zero radius; zero-extent AABB (`min==max`); zero-length capsule (`a==b`); collinear points | `c2Norm`/`c2Len` div-by-zero paths |
| float value shape | ordinary; `±0.0`; denormal; huge (`~1e38`, so `x*x` overflows); tiny (`<FLT_EPSILON`); mixed magnitudes | value-dependent SSE rounding |
| rotation in `c2x` | identity `{1,0}`; 90°; arbitrary non-unit `c2r` (never normalised by the C) | `c2Mulrv`/`c2MulrvT`/`c2Mulxv` |

Additionally, `c2GJK` carries one piece of **cross-call state**: `c2Proxy pA;`
and `c2Proxy pB;` are *uninitialised* locals, i.e. a fixed pair of stack slots
reused by every call from the same call site.  `c2MakeProxy` writes only
`verts[0 .. count)`, so an unvalidated `c2GJKCache` index at or beyond `count`
observes what an earlier call left there.  Rows 69/70 cover this; see the
"Scope" note in `ERRORS.md` for the part that is not reproducible.

`c2GJK` is the low-level entry point; `c2AABBtoCapsule` / `c2CapsuletoCapsule`
are the convenience wrappers over it, and `c2Collided` is the 9-way dispatcher.
`capsule()` is the single header-declared one-shot driver. All are covered below.

## Rows

Each row is exercised with **many** randomized inputs (seeded `SplitMix64`,
fixed seed per test so runs are reproducible), C vs Rust compared **bit-for-bit**
(`f32::to_bits`, whole-struct byte compare for out-params and caches).

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `c2V` | random `(x,y)` incl. `±0`, denormal, `±Inf`, NaN payloads | [x] |
| 2  | `c2Mulvs` | random vec × random scalar; scalar `0`, `-0`, `Inf`, NaN; overflow to `Inf` | [x] |
| 3  | `c2Maxv`, `c2Minv` | both orders, ties, `±0.0` pairs, NaN in either operand (C ternary picks `b` on NaN) | [x] |
| 4  | `c2Clampv` | `a` inside / below / above the box; inverted box (`lo>hi`); NaN in `a`/`lo`/`hi` | [x] |
| 5  | `c2Sub`, `c2Add` | random pairs; cancellation to `±0`; `Inf-Inf`; NaN either side | [x] |
| 6  | `c2Dot` | random pairs; catastrophic cancellation; overflow; `0*Inf`; NaN either side | [x] |
| 7  | `c2Det2` | random pairs; parallel (det `0`); antiparallel; overflow; NaN | [x] |
| 8  | `c2Len` | random vec; zero vec; huge vec (`Dot` overflows → `Inf`); NaN; `Inf` component | [x] |
| 9  | `c2Div`, `c2Norm` | random vec / random divisor; divisor `0`, `-0`, `Inf`, NaN; zero-length vec | [x] |
| 10 | `c2Neg`, `c2Skew`, `c2CCW90` | random vec; `±0.0` (sign of zero matters); NaN (sign bit flip) | [x] |
| 11 | `c2RotIdentity`, `c2xIdentity` | no inputs — bit-compare returned structs | [x] |
| 12 | `c2Mulrv`, `c2MulrvT` | identity rot; 90° rot; random unit rot; random **non-unit** rot; NaN/Inf in rot or vec | [x] |
| 13 | `c2Mulxv` | identity `c2x`; pure translation; pure rotation; both; non-unit rot; NaN | [x] |
| 14 | `c2BBVerts` | random AABB; `min==max`; inverted (`min>max`); NaN corners — compare all 4 out verts | [x] |
| 15 | `c2MakeProxy` | `type=CIRCLE` — check `radius`,`count`,`verts[0]`, full 72-byte struct | [x] |
| 16 | `c2MakeProxy` | `type=AABB` — `radius=0`, `count=4`, 4 verts | [x] |
| 17 | `c2MakeProxy` | `type=CAPSULE` — `radius=r`, `count=2`, 2 verts | [x] |
| 18 | `c2MakeProxy` | pre-dirtied output buffer + valid type: confirm the untouched tail bytes match | [x] |
| 19 | `c2Support` | `count=1` (circle proxy) | [x] |
| 20 | `c2Support` | `count=2` (capsule proxy), direction hitting each vertex, and a tie (`dot` equal → first wins) | [x] |
| 21 | `c2Support` | `count=4` (AABB proxy), all 8 octant directions + random | [x] |
| 22 | `c2Support` | `count=8`, random verts, random direction; NaN in `d` (no `dot > dmax` ever true → `0`) | [x] |
| 23 | `c2GJKSimplexMetric` | `count=1` → `0`; `count=2` → `c2Len`; `count=3` → `c2Det2`; random `p` fields | [x] |
| 24 | `c22` | `count=2`, `v<=0` region (A closest) — full 152-byte simplex compare | [x] |
| 25 | `c22` | `count=2`, `u<=0` region (B closest, `s->a = s->b` copy) | [x] |
| 26 | `c22` | `count=2`, interior-edge region (`u,v>0`, `div=u+v`) | [x] |
| 27 | `c22` | random simplexes (unbiased) so all three branches are hit by chance; NaN `p` fields | [x] |
| 28 | `c23` | branch 1: `vAB<=0 && uCA<=0` (vertex A) | [x] |
| 29 | `c23` | branch 2: `uAB<=0 && vBC<=0` (vertex B) | [x] |
| 30 | `c23` | branch 3: `uBC<=0 && vCA<=0` (vertex C) | [x] |
| 31 | `c23` | branch 4: edge AB (`wABC<=0`) | [x] |
| 32 | `c23` | branch 5: edge BC (`uABC<=0`) — with the `a=b; b=c` shuffle | [x] |
| 33 | `c23` | branch 6: edge CA (`vABC<=0`) — with the `b=a; a=c` shuffle | [x] |
| 34 | `c23` | branch 7: interior (origin inside the triangle), `div=u+v+w` | [x] |
| 35 | `c23` | random triangles incl. degenerate/collinear (`area==0`) and NaN | [x] |
| 36 | `c2D` | `count=1`; `count=2` with `det>0` (skew) and `det<=0` (ccw90); random | [x] |
| 37 | `c2L` | `count=1`; `count=2` with random `u`/`div`; `div` huge/tiny | [x] |
| 38 | `c2Witness` | `count=1`, `2`, `3` with random `sA`/`sB`/`u`/`div` — both out-vectors compared | [x] |
| 39 | `c2GJK` | CIRCLE↔CIRCLE, `use_radius=0`, no transforms, no cache, all out-params; random disjoint | [x] |
| 40 | `c2GJK` | CIRCLE↔CIRCLE, `use_radius=1`, random overlapping (hit path) | [x] |
| 41 | `c2GJK` | CIRCLE↔AABB, `use_radius` ∈ {0,1} × random near/far/overlapping | [x] |
| 42 | `c2GJK` | CIRCLE↔CAPSULE, `use_radius` ∈ {0,1} × random | [x] |
| 43 | `c2GJK` | AABB↔AABB, `use_radius` ∈ {0,1} × random (4 verts both sides ⇒ deepest `c23` recursion) | [x] |
| 44 | `c2GJK` | AABB↔CAPSULE, `use_radius` ∈ {0,1} × random | [x] |
| 45 | `c2GJK` | CAPSULE↔CAPSULE, `use_radius` ∈ {0,1} × random, incl. parallel & crossing capsules | [x] |
| 46 | `c2GJK` | any type pair, `ax_ptr` non-NULL (translation only), `bx_ptr` NULL | [x] |
| 47 | `c2GJK` | any type pair, both transforms non-NULL with random **unit** rotations | [x] |
| 48 | `c2GJK` | any type pair, both transforms with random **non-unit** `c2r` (C never normalises) | [x] |
| 49 | `c2GJK` | `outA=NULL, outB=NULL, iterations=NULL` — only the return value observable | [x] |
| 50 | `c2GJK` | `iterations` non-NULL: compare the iteration count itself across random shapes | [x] |
| 51 | `c2GJK` | fresh cache (`count=0`), non-NULL: compare the full 36-byte written-back cache | [x] |
| 52 | `c2GJK` | warm cache: call twice in a row on the same shapes, compare both results *and* both cache states | [x] |
| 53 | `c2GJK` | warm cache reused on *different* shapes (stale cache — exercises the inverted l.405 test) | [x] |
| 54 | `c2GJK` | hand-built cache with `count=1`, `2`, `3` and valid indices, random `metric`/`div` | [x] |
| 55 | `c2GJK` | identical shapes (A==B) — degenerate, `p == (0,0)`, `c2D` on a zero vector | [x] |
| 56 | `c2GJK` | zero-radius circles / zero-extent AABB (`min==max`) / zero-length capsule (`a==b`) | [x] |
| 57 | `c2GJK` | shapes exactly touching (`dist == rA+rB`) — boundary of the l.485 test | [x] |
| 58 | `c2GJK` | very large coordinates (`~1e18`, so `Dot` overflows to `Inf`) and very small (`~1e-30`) | [x] |
| 59 | `c2GJK` | full cross-product sweep: 3 typeA × 3 typeB × `use_radius`∈{0,1} × transforms∈{none,unit,non-unit} × cache∈{none,fresh,warm}, randomized | [x] |
| 60 | `c2AABBtoAABB` | random pairs: overlapping, separated on x only, on y only, touching edges, nested, inverted boxes | [x] |
| 61 | `c2CircletoCircle` | random: overlapping, touching (`d2 == r2` → strict `<` false), disjoint, nested, r=0, r<0 | [x] |
| 62 | `c2CircletoAABB` | random: centre inside, outside each of the 8 regions, on the boundary, r=0, inverted box | [x] |
| 63 | `c2CircletoCapsule` | random: `da<0` branch, `db<0` branch (perpendicular projection), else branch; degenerate capsule | [x] |
| 64 | `c2AABBtoCapsule` | random pairs (delegates to `c2GJK` with `use_radius=1`) | [x] |
| 65 | `c2CapsuletoCapsule` | random pairs, incl. parallel, crossing, coincident, zero-length | [x] |
| 66 | `c2Collided` | all 9 `(typeA, typeB)` combinations with randomized shapes of the matching kind | [x] |
| 67 | `c2Collided` | the argument-swapping asymmetric pairs (`AABB,CIRCLE` and `CAPSULE,AABB` reverse their args) | [x] |
| 69 | `c2GJK` (x2, sequential) | **`c2Proxy` slot persistence**: prime the slot with an AABB/AABB pair (fills `verts[0..4]`), then read those slots back via a CIRCLE/CIRCLE call whose cache indices are 1..3. The C's `c2Proxy pA/pB` are uninitialised locals in a reused stack slot, so the previous call's vertices must still be visible | [x] |
| 70 | `c2GJK` (x2, sequential) | the same, asserting the stale read is observably **different** from the zero-proxy answer, so the row cannot pass vacuously | [x] |
| 68 | `capsule` | the header-declared driver: randomized `(min_x,min_y,max_x,max_y,r)` over a wide range, plus a grid that provokes each of the 8 result bit-masks, plus `±0`/NaN/`±Inf`/denormal args | [x] |
