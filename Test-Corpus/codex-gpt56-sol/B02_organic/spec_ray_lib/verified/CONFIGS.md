# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` section and `c_src/CMakeLists.txt` defines no
options or conditional sources. There is exactly one valid feature
combination:

| # | Cargo feature combination | CMake configuration | [x] |
|---|---------------------------|----------------------|-----|
| 1 | empty (`--no-default-features --features ''`) | default, PIC enabled | [x] |

## Runtime and Input Configurations

There are no mutable runtime options. The only mode is the private `C2_TYPE`
integer consumed by `c2CastRay`: circle (`0`), AABB (`1`), or capsule (`2`).
Rows below are the branch-distinct cross-product of public entry points,
dispatch modes, and input shapes found in `c_src/src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary finite scalar pair, including signed zero | [x] |
| 2 | `c2Dot`, `c2Len` | finite vectors: zero, axis-aligned, and general | [x] |
| 3 | `c2Add`, `c2Sub`, `c2Mulvs`, `c2Div` | finite vectors with negative/zero/positive scalar (nonzero divisor) | [x] |
| 4 | `c2Norm` | nonzero axis-aligned and general vectors | [x] |
| 5 | `c2Norm`, `c2Div` | zero vector or zero divisor, producing C floating-point infinities/NaNs | [x] |
| 6 | `c2Minv`, `c2Maxv` | each component ordered less/equal/greater independently | [x] |
| 7 | `c2Skew`, `c2CCW90`, `c2Absv` | negative, signed-zero, and positive components | [x] |
| 8 | `c2MulmvT` | zero, identity-like, and general matrix/vector values | [x] |
| 9 | `c2AABBtoAABB` | overlapping boxes, containment, and edge/corner touching | [x] |
| 10 | `c2AABBtoPoint` | point inside and on each min/max boundary | [x] |
| 11 | `c2CircleToPoint` | point strictly inside, exactly on, and outside circle | [x] |
| 12 | `c2RaytoCircle` | secant hit with `0 < t < A.t` | [x] |
| 13 | `c2RaytoCircle` | tangent hit (`disc == 0`) and hits at `t == 0` / `t == A.t` | [x] |
| 14 | `c2RaytoCircle` | miss classes: negative discriminant, hit behind start, hit past `A.t` | [x] |
| 15 | `c2RaytoAABB` | broad-phase miss and separating-axis miss | [x] |
| 16 | `c2RaytoAABB` | hit selecting min-X normal (`t0` wins) | [x] |
| 17 | `c2RaytoAABB` | hit selecting max-X normal (`t1` wins) | [x] |
| 18 | `c2RaytoAABB` | hit selecting min-Y normal (`t2` wins) | [x] |
| 19 | `c2RaytoAABB` | hit selecting max-Y normal (`t3`/else wins) | [x] |
| 20 | `c2RaytoAABB` | start inside/on box, zero-length, axis-aligned, and diagonal segment | [x] |
| 21 | `c2RaytoCapsule` | ray starts inside rectangular shaft (`c2AABBtoPoint`) | [x] |
| 22 | `c2RaytoCapsule` | ray starts strictly inside endpoint A circle | [x] |
| 23 | `c2RaytoCapsule` | ray starts strictly inside endpoint B circle | [x] |
| 24 | `c2RaytoCapsule` | no transverse crossing/approach; final miss | [x] |
| 25 | `c2RaytoCapsule` | starts within shaft width with local `yAp.y < 0`; delegate endpoint A | [x] |
| 26 | `c2RaytoCapsule` | starts within shaft width with local `yAp.y >= 0`; delegate endpoint B | [x] |
| 27 | `c2RaytoCapsule` | transverse crossing intersects below shaft (`y <= 0`); delegate endpoint A | [x] |
| 28 | `c2RaytoCapsule` | transverse crossing intersects above shaft (`y >= yBb.y`); delegate endpoint B | [x] |
| 29 | `c2RaytoCapsule` | transverse crossing hits positive-radius shaft side (`c > 0`) | [x] |
| 30 | `c2RaytoCapsule` | transverse crossing hits negative-radius shaft side (`c <= 0`) | [x] |
| 31 | `c2CastRay` | type `0`, circle payload; hit and miss | [x] |
| 32 | `c2CastRay` | type `1`, AABB payload; hit and miss | [x] |
| 33 | `c2CastRay` | type `2`, capsule payload; hit and miss | [x] |
| 34 | `spec_ray` | normalized finite segment toward mouse; circle hit and miss | [x] |
| 35 | `spec_ray` | mouse equals ray origin, producing zero-vector normalization | [x] |
