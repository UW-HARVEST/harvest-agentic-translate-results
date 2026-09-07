# CONFIGS.md — Phase B configuration surface table

Axes derived mechanically from the C source (`c_src/src/lib.c`), not from
assumptions:

**Runtime options / modes.** The library has no init struct, no global state, no
`#ifdef`, and no setter. Its single mode selector is the `C2_TYPE typeB`
argument of `c2Collided` (3 valid values, `lib.c:105-113`). `circle_collide`
hard-codes its own shapes and drives all three modes in sequence.

**Branch points the C actually distinguishes.**
* `c2Maxv` / `c2Minv` (`lib.c:43-51`): two *independent* per-component ternaries
  → 4 taken/not-taken combinations each.
* `c2Clampv` (`lib.c:53`): composition of the two above → 16 combinations of
  below-lo / inside / above-hi per axis (9 meaningful 2-D regions).
* `c2CircletoCapsule` (`lib.c:87-99`): 3 mutually exclusive arms —
  `da < 0` (before-A region), `da >= 0 && db < 0` (side/projection region,
  the ONLY arm that divides), `da >= 0 && db >= 0` (after-B region).
* `c2Collided` (`lib.c:105`): 3 valid switch arms + `default`.
* `circle_collide` (`lib.c:115-143`): 3 predicates packed into bits 0/1/2 → 8
  reachable result values.

**Input shapes the code special-cases.** point/degenerate vs extended AABB;
inverted AABB; zero-length vs non-zero capsule segment; axis-aligned vs oblique
segment; zero / negative / huge / infinite / NaN radius; `±0.0`; denormals;
values at the exact tangency boundary `d2 == r2` (where `<` is false).

**Entry points.** All 12 exported symbols are exercised directly through the
`.so`, including the lowest-level ones (`c2V`, `c2Sub`, `c2Dot`, `c2Mulvs`,
`c2Maxv`, `c2Minv`, `c2Clampv`) — not just `circle_collide`. Every row uses
many randomized inputs from a fixed-seed PRNG (SplitMix64, seed `0x2545F491_4F6CDD1D`)
plus the hand-picked boundary values named in the row.

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random finite `(x,y)`; then `±0.0`, denormal, `±inf`, NaN, `±FLT_MAX/MIN` — bitwise round-trip | [x] |
| 2 | `c2Sub` | random finite pairs; then equal operands (exact zero), sign-crossing, `inf-inf`, denormal cancellation | [x] |
| 3 | `c2Dot` | random finite pairs; then orthogonal (exact 0), antiparallel, magnitudes spanning `1e-30..1e30` (over/underflow of the sum) | [x] |
| 4 | `c2Mulvs` | random `c2v` × random scalar; then `b=0`, `b=-0.0`, `b=inf`, `b=NaN`, `b` denormal, overflow to `inf` | [x] |
| 5 | `c2Maxv` | all 4 per-component ternary outcomes: `(a.x>b.x, a.y>b.y)` ∈ {TT,TF,FT,FF} + random | [x] |
| 6 | `c2Maxv` | equality / `+0.0` vs `-0.0` (comparison false → `b` wins) + NaN in each of the 4 slots | [x] |
| 7 | `c2Minv` | all 4 per-component ternary outcomes + random | [x] |
| 8 | `c2Minv` | equality / signed-zero / NaN in each of the 4 slots | [x] |
| 9 | `c2Clampv` | `a` in all 9 regions relative to the `lo..hi` box (below/inside/above per axis), random boxes | [x] |
| 10 | `c2Clampv` | degenerate box `lo == hi`; inverted box `lo > hi`; NaN in `a`/`lo`/`hi` | [x] |
| 11 | `c2CircletoCircle` | random circles, overlapping (result 1) | [x] |
| 12 | `c2CircletoCircle` | random circles, separated (result 0) | [x] |
| 13 | `c2CircletoCircle` | exact tangency `d2 == r2` (strict `<` → 0); concentric; zero radius; negative radius; huge radius overflow | [x] |
| 14 | `c2CircletoAABB` | circle centre INSIDE the box (clamp is identity, `d2 == 0`) | [x] |
| 15 | `c2CircletoAABB` | circle centre in each of the 4 edge regions (clamped on one axis only) | [x] |
| 16 | `c2CircletoAABB` | circle centre in each of the 4 corner regions (clamped on both axes) | [x] |
| 17 | `c2CircletoAABB` | degenerate box (`min == max`), thin box (zero width, non-zero height), inverted box | [x] |
| 18 | `c2CircletoAABB` | `A.r == 0` (never collides), negative `A.r`, huge `A.r`, random large-scale boxes | [x] |
| 19 | `c2CircletoCapsule` | arm 1: `da < 0` — circle beyond endpoint `a` | [x] |
| 20 | `c2CircletoCapsule` | arm 2: `da >= 0 && db < 0` — circle beside the segment (the division path) | [x] |
| 21 | `c2CircletoCapsule` | arm 3: `da >= 0 && db >= 0` — circle beyond endpoint `b` | [x] |
| 22 | `c2CircletoCapsule` | arm boundaries: `da == 0` exactly, `db == 0` exactly (both `<` tests false) | [x] |
| 23 | `c2CircletoCapsule` | degenerate segment `a == b` (`dot(n,n) == 0`), horizontal, vertical, and oblique segments | [x] |
| 24 | `c2CircletoCapsule` | zero / negative / huge radii; denormal-length segment (`da/dot(n,n)` overflow); random capsules at scale `1e-6..1e6` | [x] |
| 25 | `c2Collided` | `typeB = C2_TYPE_CIRCLE` (0) with random `c2Circle` pairs — must equal `c2CircletoCircle` | [x] |
| 26 | `c2Collided` | `typeB = C2_TYPE_AABB` (1) with random circle + AABB — must equal `c2CircletoAABB` | [x] |
| 27 | `c2Collided` | `typeB = C2_TYPE_CAPSULE` (2) with random circle + capsule — must equal `c2CircletoCapsule` | [x] |
| 28 | `c2Collided` | aliasing shape: same buffer passed as both `A` and `B` (self-collision), for all 3 valid types | [x] |
| 29 | `c2Collided` | over-sized/heterogeneous backing buffer (`B` points into a 32-byte buffer whose tail holds garbage) for all 3 types — tail must not be read | [x] |
| 30 | `circle_collide` | random `(x,y,r)` over the shapes' bounding region — full sweep of all 8 reachable packed results | [x] |
| 31 | `circle_collide` | targeted inputs hitting each bit alone: bit0 (circle at `(-70,0) r20`), bit1 (AABB `[-40,-15]²`), bit2 (capsule `(-40,40)-(-20,100) r10`) | [x] |
| 32 | `circle_collide` | multi-bit overlaps (bits 0+1, 1+2, 0+1+2) and result `0`; grid sweep over `x,y ∈ [-120,40]`, `r ∈ {0,1,5,20,100}` | [x] |
| 33 | `circle_collide` | extreme scalars: `±0.0`, denormals, `±FLT_MAX`, `±inf`, NaN, huge `r` | [x] |
| 34 | composed pipeline | `c2Clampv`→`c2Sub`→`c2Dot` re-implemented via the `.so`'s own low-level exports and compared against `c2CircletoAABB` from the *other* library (cross-library composition) | [x] |
| 35 | bit-exact float returns | for every `c2v`/`float`-returning export, compare raw `to_bits()` (not `==`), so `-0.0` vs `+0.0` and NaN payloads are caught | [x] |
| 36 | feature combo | all rows above re-run under `--no-default-features` (the crate declares no `[features]`, so this is the single available combination) | [x] |

No binary/driver target exists in either build (`CMakeLists.txt` builds only a
`SHARED` library; `Cargo.toml` declares only `crate-type = ["cdylib"]`), so the
stdout-comparison item is not applicable.
