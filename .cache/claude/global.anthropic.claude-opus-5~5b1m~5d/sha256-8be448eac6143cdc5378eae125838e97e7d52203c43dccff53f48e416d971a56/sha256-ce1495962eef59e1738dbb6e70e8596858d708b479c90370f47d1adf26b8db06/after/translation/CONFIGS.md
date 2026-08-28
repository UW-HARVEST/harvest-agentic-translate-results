# CONFIGS.md — Phase B configuration surface table

## Axes the C code actually branches on

Derived from `c_src/include/lib.h` + every `if` / `switch` / ternary in
`c_src/src/lib.c`. There are **no** `#ifdef`s, no global state, no init/teardown
and no runtime option structs — the only "mode" flag in the library is the
`C2_TYPE typeB` discriminant of `c2CastRay`. The remaining axes are input
*shapes*.

* **A. entry point** — all 22 exported symbols, from the lowest-level scalar
  helpers (`c2V`, `c2Dot`, `c2Len`, `c2Add`, `c2Sub`, `c2Mulvs`, `c2Div`,
  `c2Norm`, `c2Minv`, `c2Maxv`, `c2Skew`, `c2Absv`, `c2CCW90`, `c2MulmvT`),
  through the predicates (`c2AABBtoAABB`, `c2AABBtoPoint`, `c2CircleToPoint`),
  the three raycasters (`c2RaytoCircle`, `c2RaytoAABB`, `c2RaytoCapsule`), the
  dispatcher (`c2CastRay`) and the one-shot wrapper (`gen_ray`).
* **B. `C2_TYPE typeB` mode** (`c2CastRay` only): `C2_TYPE_CIRCLE=0`,
  `C2_TYPE_AABB=1`, `C2_TYPE_CAPSULE=2`, plus out-of-range (see ERRORS.md #38).
* **C. float value class** — the code's comparisons are all *ordered* (`comiss`)
  so each class takes a different path: normal, `±0.0`, subnormal, huge
  (`±3.4e38`, overflow-to-inf), `±inf`, quiet `NaN`, signalling `NaN`,
  negative-`NaN` (sign bit set — matters because the C uses the ternary
  `x < 0 ? -x : x` instead of `fabsf`, so `-NaN` and `-0.0` survive `c2Absv`).
* **D. geometric relation** — for each raycaster: miss, hit-front, hit-behind,
  hit-past-`A.t`, start-inside, tangent/grazing, axis-aligned, and degenerate
  (zero-radius, zero-length ray, `a == b` capsule, inverted AABB).
* **E. which of the 4 slab candidates wins** in `c2RaytoAABB` — the four
  `t0 >= t1 && ...` chains select normals `(-1,0)`, `(1,0)`, `(0,-1)`, `(0,1)`;
  all four must be reached, including the tie-break order.
* **F. which branch of `c2RaytoCapsule`** — early `c2AABBtoPoint` accept,
  end-cap-A `c2CircleToPoint` accept, end-cap-B accept, `|yAp.x| < B.r` →
  `Ca`/`Cb` delegate, slab-crossing → `y <= 0` → `Ca`, `y >= yBb.y` → `Cb`,
  side-wall hit with `c > 0` (`M.x`) or `c <= 0` (`c2Skew(M.y)`), and final
  reject.
* **G. `A.t` (ray length)** — `0`, negative, small, large, `inf`, `NaN`.
* **H. out-parameter aliasing** — distinct `c2Raycast*` vs. the same pointer
  passed for `cast1`/`cast2`/`cast3`.
* **I. NaN payload identity** — `mulss`/`addss`/`subss`/`divss` return the
  *destination* operand when **both** operands are NaN, so the C's operand order
  is observable, but only when the two NaNs carry *different* payloads. A pool of
  one shared `f32::NAN` can never expose it; rows 59–62 exist for this axis.

## Rows (each is a differential test run over many seeded random inputs)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
|  1 | `c2V` | random normals; then `±0.0`, `±inf`, qNaN, sNaN, `-NaN`, subnormals (bit-exact round-trip) | [x] |
|  2 | `c2Dot` | random normals (both lanes) | [x] |
|  3 | `c2Dot` | special-value cross product: `{±0,±inf,±NaN,sNaN,huge,subnormal}²` in both lanes (checks `inf*0`, `inf-inf`, NaN payload/operand order) | [x] |
|  4 | `c2Len` | random normals | [x] |
|  5 | `c2Len` | overflow (`3e38`), `inf`, NaN, `±0.0`, subnormal (checks `sqrtf` import vs. `sqrtss`) | [x] |
|  6 | `c2Add` / `c2Sub` | random normals | [x] |
|  7 | `c2Add` / `c2Sub` | special-value cross product incl. `inf + -inf`, `-0.0 + 0.0`, NaN+NaN (operand-order sensitive) | [x] |
|  8 | `c2Mulvs` | random vector × random scalar | [x] |
|  9 | `c2Mulvs` | special scalars `{±0,±inf,NaN,sNaN,huge,tiny}` × special vectors (`0*inf`, NaN dest order) | [x] |
| 10 | `c2Div` | random vector / random scalar (verifies the **reciprocal-multiply** quirk, not true division) | [x] |
| 11 | `c2Div` | `b ∈ {0.0, -0.0, ±inf, NaN, subnormal, huge}` (`1/0 = inf`, `1/inf = 0`, `1/subnormal = inf`) | [x] |
| 12 | `c2Norm` | random vectors | [x] |
| 13 | `c2Norm` | `(0,0)` → NaN, `(inf,x)`, `(NaN,x)`, subnormal, huge (overflowing `dot`) | [x] |
| 14 | `c2Minv` / `c2Maxv` | random normals | [x] |
| 15 | `c2Minv` / `c2Maxv` | `±0.0` and NaN in either operand (ternary min/max is **not** `fminf`/`fmaxf`: NaN → 2nd operand, `-0.0` vs `0.0` → 2nd) | [x] |
| 16 | `c2Skew` / `c2CCW90` | random normals + `±0.0`, `±inf`, qNaN/sNaN/`-NaN` (negation is a sign-bit flip, must preserve payload) | [x] |
| 17 | `c2Absv` | random normals + `-0.0` (kept negative!), `-NaN` (kept negative!), `-inf`, sNaN | [x] |
| 18 | `c2MulmvT` | random 2×2 matrix × random vector | [x] |
| 19 | `c2MulmvT` | special-value matrices/vectors (operand-order-sensitive `mulss`/`addss` destinations) | [x] |
| 20 | `c2AABBtoAABB` | random overlapping / random separated boxes | [x] |
| 21 | `c2AABBtoAABB` | axis-touching boxes (exact `==` edges), inverted boxes (`min > max`), degenerate (`min == max`) | [x] |
| 22 | `c2AABBtoAABB` | NaN / `±inf` / `±0.0` coordinates in either box | [x] |
| 23 | `c2AABBtoPoint` | random point inside / outside; on each of the 4 edges and 4 corners | [x] |
| 24 | `c2AABBtoPoint` | NaN / `±inf` point, inverted box, zero-area box | [x] |
| 25 | `c2CircleToPoint` | random inside / outside / exactly on the rim | [x] |
| 26 | `c2CircleToPoint` | `r ∈ {0, -1, inf, NaN, subnormal, huge}`, NaN point | [x] |
| 27 | `c2RaytoCircle` | random ray + random circle, unnormalised `A.d` (raw API — no normalisation enforced) | [x] |
| 28 | `c2RaytoCircle` | random ray with **normalised** `A.d` and `A.t` = distance-to-target (the shape `gen_ray` produces) | [x] |
| 29 | `c2RaytoCircle` | ray origin inside the circle; tangent ray; ray pointing away; `A.t ∈ {0, -1, inf, NaN}` | [x] |
| 30 | `c2RaytoCircle` | `r ∈ {0, -r, inf, NaN}`; circle centre at `±inf`; `A.d = (0,0)` | [x] |
| 31 | `c2RaytoAABB` | random ray + random box, unnormalised `A.d` | [x] |
| 32 | `c2RaytoAABB` | normalised `A.d`, `A.t` = target distance | [x] |
| 33 | `c2RaytoAABB` | forced normal-selection sweep: rays entering from -x, +x, -y, +y and exact 45° ties (covers all four `out->n` branches + tie-break order) | [x] |
| 34 | `c2RaytoAABB` | axis-aligned rays (`d = (±1,0)` / `(0,±1)`) → `da*db`/`da-db` zero-denominator paths | [x] |
| 35 | `c2RaytoAABB` | `A.t = 0` (zero-length ray), `A.t` huge, `A.t` NaN, `A.t` negative | [x] |
| 36 | `c2RaytoAABB` | degenerate / inverted box, box with NaN or `±inf` bounds, ray origin inside box | [x] |
| 37 | `c2RaytoCapsule` | random ray + random capsule, unnormalised `A.d` | [x] |
| 38 | `c2RaytoCapsule` | normalised `A.d`, `A.t` = target distance | [x] |
| 39 | `c2RaytoCapsule` | ray origin inside the `capsule_bb` slab (early `c2AABBtoPoint` accept) | [x] |
| 40 | `c2RaytoCapsule` | ray origin inside end-cap A / end-cap B (`c2CircleToPoint` accepts) | [x] |
| 41 | `c2RaytoCapsule` | `|yAp.x| < B.r` delegate branch, both `yAp.y < 0` (→`Ca`) and `>= 0` (→`Cb`) | [x] |
| 42 | `c2RaytoCapsule` | slab-crossing branch: `y <= 0` → `Ca`, `y >= yBb.y` → `Cb`, and the side-wall hit | [x] |
| 43 | `c2RaytoCapsule` | side-wall hit with `c > 0` (`out->n = M.x`) and with `c <= 0` (`out->n = c2Skew(M.y)`) | [x] |
| 44 | `c2RaytoCapsule` | degenerate axis `B.a == B.b`; `B.r ∈ {0, negative, inf, NaN}`; vertical / horizontal / 45° axes | [x] |
| 45 | `c2RaytoCapsule` | `d = yAe.x - yAp.x == 0` (unguarded division) → `t = ±inf`/NaN | [x] |
| 46 | `c2CastRay` | `typeB = C2_TYPE_CIRCLE` over the row-27/29/30 input population | [x] |
| 47 | `c2CastRay` | `typeB = C2_TYPE_AABB` over the row-31/33/35/36 input population | [x] |
| 48 | `c2CastRay` | `typeB = C2_TYPE_CAPSULE` over the row-37..45 input population | [x] |
| 49 | `gen_ray` | fully random 16 floats (all three shapes cast with a shared derived ray) | [x] |
| 50 | `gen_ray` | "realistic mouse-pick" population: `mp`/`r_p` in `[-100,100]`, radii in `(0,50]` — the intended use | [x] |
| 51 | `gen_ray` | degenerate ray (`mp == r_p`) → `ray.d = (NaN,NaN)` propagates into all three casts | [x] |
| 52 | `gen_ray` | each of the 8 hit-bitmask values `0..7` reached; return value compared exactly | [x] |
| 53 | `gen_ray` | special float values sprinkled into the 16 arguments (`±0`, `±inf`, NaN, sNaN, huge, subnormal) | [x] |
| 54 | `gen_ray` | out-parameter aliasing: `cast1 == cast2 == cast3` (last-writer-wins), and partial aliasing `cast1 == cast3` | [x] |
| 55 | `gen_ray` | axis-aligned / grazing configurations that make `ray.t` exactly `0` or negative | [x] |
| 56 | all raycasters | `*out` pre-filled with a sentinel bit pattern; asserted byte-identical afterwards on **both** hit and miss (catches "writes on reject" divergence) | [x] |
| 57 | `c2Dot`/`c2Len`/`c2Norm`/`c2Div`/`c2Mulvs`/`c2Add`/`c2Sub`/`c2MulmvT` | full-random **bit-pattern** fuzz (`u32` → `f32`, all classes incl. sNaN) — 200k+ cases | [x] |
| 58 | `c2RaytoCircle`/`c2RaytoAABB`/`c2RaytoCapsule`/`c2CastRay`/`gen_ray` | full-random bit-pattern fuzz of every float field | [x] |

| 59 | all scalar helpers | every float slot drawn independently from a pool of **48 mutually distinct NaN payloads** (both signs, quiet and signalling) — the only way the C's `mulss`/`addss` *destination* operand becomes observable | [x] |
| 60 | `c2RaytoCircle`/`c2RaytoAABB`/`c2RaytoCapsule` | the same distinct-NaN-payload pool at 6 different NaN densities (10 %, 20 %, 35 %, 50 %, 70 %, 90 %), so deep branches stay reachable while two NaNs meet | [x] |
| 61 | all three raycasters + `c2CastRay` | a known-hit geometric setup with 1–3 slots poisoned by distinct NaN payloads (keeps the side-wall / slab-crossing / four-normal branches live) | [x] |
| 62 | `gen_ray` | 1–4 of the 16 arguments poisoned with distinct NaN payloads on top of a jittered all-three-shapes-hit configuration | [x] |

All comparisons are **bit-exact** (`f32::to_bits`, `c_int` equality), never
epsilon-based.
