# CONFIGS.md — configuration surface table (Phase B)

Derived **mechanically** from `c_src/src/lib.c`: every runtime option the public
API can set, every `switch`/`if` the code branches on, and every input *shape*
the code special-cases.

## Axes the C code actually branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `C2_TYPE typeA` | `C2_TYPE_CIRCLE`=0, `C2_TYPE_AABB`=1, `C2_TYPE_CAPSULE`=2 | `c2MakeProxy` switch, `c2Collided` outer switch |
| `C2_TYPE typeB` | same 3 | `c2MakeProxy`, `c2Collided` inner switches |
| proxy shape | `radius`/`count` per type: circle → `(r, 1)`, AABB → `(0, 4)`, capsule → `(r, 2)` | `c2MakeProxy` |
| `const c2x *ax_ptr` | `NULL` (→ `c2xIdentity()`), identity, pure rotation, pure translation, rotation+translation, non-unit `c2r` | `if (!ax_ptr)`, `c2Mulxv`, `c2MulrvT` |
| `const c2x *bx_ptr` | same 6 | `if (!bx_ptr)` |
| `int use_radius` | `0` (skip radius block) / non-zero (enter it) | `else if (use_radius)` |
| radius sub-branch | `dist > rA+rB && dist > FLT_EPSILON` (shrink) vs else (midpoint) vs `a==b` after shrink (`dist=0`) | `c2GJK` tail |
| `c2v *outA`, `c2v *outB`, `int *iterations` | `NULL` / non-`NULL` | `if (outA)`, `if (outB)`, `if (iterations)` |
| `c2GJKCache *cache` | `NULL` / `count == 0` (cold) / `count != 0` (warm, replayed) / carried across repeated calls | `if (cache)`, `!!cache->count`, `cache_was_read` |
| simplex `count` | 1 (vertex), 2 (edge → `c22`), 3 (triangle → `c23` → `hit`) | main-loop `switch (s.count)` |
| `c22` outcome | vertex-`a` / vertex-`b` / edge | 3 arms |
| `c23` outcome | vertex-`a` / vertex-`b` / vertex-`c` / edge-`ab` / edge-`bc` / edge-`ca` / interior | 7 arms |
| `c2D` outcome | `count==1` → `-a`; `count==2` & `det>0` → `c2Skew`; `count==2` & `det<=0` → `c2CCW90`; else `(0,0)` | `c2D` |
| loop exit | `hit` (count==3) / `d1 > d0` / `dot(d,d) < eps²` / duplicate support / `iter == 20` | 5 exits |
| geometric relation | disjoint-far, disjoint-near, exactly touching, shallow overlap, deep overlap, one contained in the other, coincident | drives all of the above |
| degenerate shapes | circle `r == 0`, AABB `min == max`, AABB `min > max` (inverted), capsule `a == b`, capsule `r == 0` | no guards — flow through normally |
| magnitude | ~1e-4, ~1, ~1e2, ~1e6 coordinates; subnormals | float rounding / `FLT_EPSILON` and `d1 > d0` tests |
| signed zero | `+0.0` vs `-0.0` operands | `?:` in `c2Maxv`/`c2Minv`, `sqrtf(-0.0)` |
| `c2Support` count | 1, 2, 4 (the three proxy counts), plus 3/5/8 for direct calls | `for (i = 1; i < count; ++i)` |

Feature axis: `translation/Cargo.toml` declares **no** `[features]` table, so the
only build configurations are `default` (= empty), `--no-default-features`, and
`--all-features` (= empty). All three are verified — see `run_all.sh`.

## Rows

Every row is exercised with **many randomized inputs** (fixed-seed xorshift64\*
PRNG, `tests/common/mod.rs::Rng`) driving both `.so`s and compared bit-for-bit
(`f32::to_bits`, `i32`, and full struct byte images).

Test file: `tests/phase_b_valid.rs`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `c2V` | random `(x,y)` incl. `±0`, subnormals, huge, `±inf`, NaN | [x] |
| 2 | `c2Mulvs` | random vector × random scalar (incl. `0`, `-0`, `inf`, NaN) | [x] |
| 3 | `c2Add`, `c2Sub` | random vector pairs, all magnitude classes | [x] |
| 4 | `c2Dot`, `c2Det2` | random vector pairs; cancellation cases (`a == b`, `a == -b`, orthogonal) | [x] |
| 5 | `c2Maxv`, `c2Minv` | random pairs; `a==b`; `+0/-0`; one NaN component; both NaN | [x] |
| 6 | `c2Clampv` | random `a` with well-ordered box, `a` inside / left / right / above / below, `lo == hi`, inverted `lo > hi` | [x] |
| 7 | `c2Len` | random vectors; zero vector; huge (overflow to `inf`); subnormal; `-0` components | [x] |
| 8 | `c2Div`, `c2Norm` | random vector ÷ random scalar; unit-length input; huge/tiny input | [x] |
| 9 | `c2Neg`, `c2Skew`, `c2CCW90` | random vectors incl. `±0`, `±inf`, NaN (sign-bit propagation) | [x] |
| 10 | `c2RotIdentity`, `c2xIdentity` | no inputs — exact bit pattern of the returned struct | [x] |
| 11 | `c2Mulrv`, `c2MulrvT` | random `c2r` (unit rotations from `sin`/`cos`, non-unit, zero, negative) × random `c2v`; round-trip `c2MulrvT(r, c2Mulrv(r, v))` | [x] |
| 12 | `c2Mulxv` | random `c2x` (identity / rot-only / trans-only / rot+trans / non-unit `c2r`) × random `c2v` | [x] |
| 13 | `c2BBVerts` | random well-ordered AABB; `min == max`; inverted AABB; huge/tiny extents — all 4 output verts compared | [x] |
| 14 | `c2MakeProxy` | `type = C2_TYPE_CIRCLE`, random circle (incl. `r == 0`, `r < 0`) — full 72-byte proxy image compared | [x] |
| 15 | `c2MakeProxy` | `type = C2_TYPE_AABB`, random AABB (well-ordered / inverted / degenerate) — full proxy image | [x] |
| 16 | `c2MakeProxy` | `type = C2_TYPE_CAPSULE`, random capsule (incl. `a == b`, `r == 0`) — full proxy image | [x] |
| 17 | `c2MakeProxy` | pre-dirtied output proxy (non-zero garbage) for each of the 3 valid types — verifies only the fields the C writes are changed | [x] |
| 18 | `c2GJKSimplexMetric` | `count == 1` (→ `0`), `count == 2` (→ `c2Len`), `count == 3` (→ `c2Det2`), random simplex vertices | [x] |
| 19 | `c22` | random 2-vertex simplex, uniformly random positions ⇒ hits all 3 arms; full 152-byte simplex image compared | [x] |
| 20 | `c22` | 2-vertex simplex constructed so the origin projects strictly inside the segment (edge arm) | [x] |
| 21 | `c22` | 2-vertex simplex with origin beyond `a` (`v<=0` arm) and beyond `b` (`u<=0` arm), randomized within each region | [x] |
| 22 | `c23` | random 3-vertex simplex, uniform positions ⇒ mixes all 7 arms; full simplex image compared | [x] |
| 23 | `c23` | triangle **containing** the origin (interior arm, `count = 3`) | [x] |
| 24 | `c23` | origin in each of the 3 vertex regions (arms 1–3), randomized | [x] |
| 25 | `c23` | origin in each of the 3 edge regions (arms 4–6), randomized | [x] |
| 26 | `c23` | clockwise vs counter-clockwise winding (sign of `area`) for the same point set | [x] |
| 27 | `c2D` | `count == 1`, `count == 2` with `det > 0` (`c2Skew`), `count == 2` with `det < 0` (`c2CCW90`), `count == 3` | [x] |
| 28 | `c2L` | `count == 1`; `count == 2` with random barycentric `u`/`div` (incl. `div` not equal to `u+v`) | [x] |
| 29 | `c2Witness` | `count == 1`, `2`, `3` with random `sA`/`sB`/`u`/`div` | [x] |
| 30 | `c2Support` | `count == 1` (proxy-circle shape) with random direction | [x] |
| 31 | `c2Support` | `count == 2` (proxy-capsule shape), random verts/direction; tie cases | [x] |
| 32 | `c2Support` | `count == 4` (proxy-AABB shape), random verts/direction | [x] |
| 33 | `c2Support` | `count == 3, 5, 8` (direct low-level call, longer arrays) | [x] |
| 34 | `c2AABBtoAABB` | random pairs across all 4 separating-axis tests; touching (`max.x == min.x`); nested; identical; inverted | [x] |
| 35 | `c2CircletoCircle` | random pairs: disjoint, touching (`d == rA+rB`), overlapping, coincident centres, `r == 0`, nested | [x] |
| 36 | `c2CircletoAABB` | random circle × random AABB: centre inside / outside on each of 8 Voronoi regions / exactly on an edge / on a corner; `r == 0`; degenerate AABB | [x] |
| 37 | `c2CircletoCapsule` | random circle × random capsule hitting all 3 arms (`da<0`, `db<0`, else); axis-aligned, diagonal, degenerate (`a==b`) capsules | [x] |
| 38 | `c2AABBtoCapsule` | random AABB × random capsule (GJK path, `use_radius = 1`) — disjoint / touching / overlapping / capsule inside AABB | [x] |
| 39 | `c2CapsuletoCapsule` | random capsule pairs (GJK path): parallel, crossing, collinear, coincident, degenerate | [x] |
| 40 | `c2Collided` | `(CIRCLE, CIRCLE)` random shapes | [x] |
| 41 | `c2Collided` | `(CIRCLE, AABB)` random shapes | [x] |
| 42 | `c2Collided` | `(CIRCLE, CAPSULE)` random shapes | [x] |
| 43 | `c2Collided` | `(AABB, CIRCLE)` — note the C **swaps** the arguments | [x] |
| 44 | `c2Collided` | `(AABB, AABB)` random shapes | [x] |
| 45 | `c2Collided` | `(AABB, CAPSULE)` random shapes | [x] |
| 46 | `c2Collided` | `(CAPSULE, CIRCLE)` — C swaps arguments | [x] |
| 47 | `c2Collided` | `(CAPSULE, AABB)` — C swaps arguments | [x] |
| 48 | `c2Collided` | `(CAPSULE, CAPSULE)` random shapes | [x] |
| 49 | `c2GJK` | all 9 `(typeA, typeB)` combinations, `ax_ptr = bx_ptr = NULL`, `use_radius = 1`, all out-params non-NULL, `cache = NULL`; random shapes | [x] |
| 50 | `c2GJK` | all 9 type combinations, `use_radius = 0` | [x] |
| 51 | `c2GJK` | all 9 type combinations, `ax_ptr` = explicit identity, `bx_ptr` = explicit identity (must equal the NULL case) | [x] |
| 52 | `c2GJK` | all 9 type combinations, `ax` = pure rotation, `bx` = `NULL` | [x] |
| 53 | `c2GJK` | all 9 type combinations, `ax` = pure translation, `bx` = pure translation | [x] |
| 54 | `c2GJK` | all 9 type combinations, `ax` and `bx` = rotation + translation (random angle/offset) | [x] |
| 55 | `c2GJK` | all 9 type combinations with a **non-unit** `c2r` (`c*c + s*s != 1`) — the code never normalises | [x] |
| 56 | `c2GJK` | all 9 type combinations, `cache` non-NULL and zeroed (cold), single call; cache write-back compared field by field | [x] |
| 57 | `c2GJK` | all 9 type combinations, `cache` carried over **3 successive calls** with slightly moved shapes (warm-start replay path, `cache_was_read = 1`) | [x] |
| 58 | `c2GJK` | all 9 type combinations, `cache` warm + transforms changed between calls (cached indices applied to new transforms) | [x] |
| 59 | `c2GJK` | shapes far apart (large `dist`, converges in few iterations) vs deeply overlapping (`hit` path) vs exactly touching | [x] |
| 60 | `c2GJK` | degenerate proxies: zero-radius circle, `min == max` AABB, `a == b` capsule, in every pairing | [x] |
| 61 | `c2GJK` | huge (`~1e6`) and tiny (`~1e-4`) coordinate magnitudes, both same-scale and mixed-scale | [x] |
| 62 | `c2GJK` | `outA`/`outB`/`iterations` selectively NULL while the others are set (2⁴ subset sweep) | [x] |
| 63 | `c2GJK` | AABB-vs-AABB with all 4 verts (`count = 4`) so `c2Support` sees the widest proxy and `iter` grows | [x] |
| 64 | `aabb` | random `(min_x,min_y,max_x,max_y)`; well-ordered, inverted, degenerate; values chosen to make each of the 3 result bits flip independently | [x] |
| 65 | `aabb` | exhaustive-ish grid sweep over the region touched by the 3 hard-coded shapes (circle at `(-70,0) r20`, AABB `(-40,-40)..(-15,-15)`, capsule `(-40,40)..(-20,100) r10`) so all 8 bitmask values are observed | [x] |

## Coverage summary

`tests/phase_b_valid.rs`: **67 tests, all passing** against both `.so`s, in `dev`
and `release`, under every feature combination, and re-run with 3 extra random
seeds per combination (`C2_DIFF_SEED=1..3` — set it to any value to soak the same
rows with completely different inputs). Rows 1–65: **65/65 checked**.

Each row drives thousands of randomized inputs (`ITERS = 4000` for the scalar and
simplex rows; 9 type pairs × hundreds of shapes for the `c2GJK` rows). Comparison
is always bit-exact: `f32::to_bits` for scalars, field-by-field bit equality for
`c2v` / `c2r` / `c2x` / `c2Proxy` / `c2Simplex` / `c2GJKCache`.

Several rows additionally **assert coverage** rather than just equality, so a
generator that silently stops reaching a branch fails the test:

* `row19`–`row21`: all three `c22` arms must be observed.
* `row22`–`row26`: all three vertex arms, all three edge arms and the interior arm
  of `c23` must be observed.
* `row27`: both the `c2Skew` and the `c2CCW90` arm of `c2D`.
* `row34`–`row48`: both "collided" and "not collided" outcomes.
* `row49`–`row61`: both zero and non-zero `c2GJK` distances.
* `row63`: `*iterations >= 2` at least once (the GJK loop really iterates).
* `row64`/`row65`: at least 4 / 6 distinct `aabb()` bitmask values.

## Deliberate non-configuration

`c2GJK` caches whose `iA`/`iB` entries are `>= proxy.count` are **not** part of the
valid configuration surface: the C reads uninitialised stack memory there. See
`ERRORS.md` row U4. Every cache the library itself produces satisfies the
constraint, and rows 57/58 exercise exactly that (library-produced) warm-start
path.
