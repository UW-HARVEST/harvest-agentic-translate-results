# Configuration Surface

There are no Cargo features, C preprocessor feature flags, mutable options, or
byte-order/format choices. The runtime axes are IEEE-754 value classes,
geometric relation, ray interval, output pointer use, and `c2CastRay`'s shape
tag. Rows are derived from every exported entry point and the branches in
`src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C01 | `c2V` | arbitrary `x,y` bit patterns, including signed zero, infinity, and NaN | [x] |
| C02 | `c2Dot` | finite and IEEE-edge vector pairs | [x] |
| C03 | `c2Len` | zero, finite nonzero, overflow, infinity, and NaN vector | [x] |
| C04 | `c2Add` | finite and IEEE-edge vector pairs | [x] |
| C05 | `c2Sub` | finite and IEEE-edge vector pairs | [x] |
| C06 | `c2Mulvs` | finite and IEEE-edge vector/scalar pairs | [x] |
| C07 | `c2Div` | positive, negative, signed-zero, infinite, and NaN divisor | [x] |
| C08 | `c2Norm` | finite nonzero, zero, infinite, and NaN vector | [x] |
| C09 | `c2Minv` | each per-axis `<` outcome, equality, signed zero, and NaN | [x] |
| C10 | `c2Maxv` | each per-axis `>` outcome, equality, signed zero, and NaN | [x] |
| C11 | `c2Skew` | finite and IEEE-edge vector | [x] |
| C12 | `c2Absv` | all per-axis sign combinations, signed zero, and NaN | [x] |
| C13 | `c2CCW90` | finite and IEEE-edge vector | [x] |
| C14 | `c2MulmvT` | arbitrary finite and IEEE-edge matrix/vector | [x] |
| C15 | `c2AABBtoAABB` | overlapping, contained, or edge-touching boxes | [x] |
| C16 | `c2AABBtoAABB` | `B.max.x < A.min.x` | [x] |
| C17 | `c2AABBtoAABB` | `A.max.x < B.min.x` | [x] |
| C18 | `c2AABBtoAABB` | `B.max.y < A.min.y` | [x] |
| C19 | `c2AABBtoAABB` | `A.max.y < B.min.y` | [x] |
| C20 | `c2AABBtoPoint` | point inside or on any box boundary | [x] |
| C21 | `c2AABBtoPoint` | `B.x < A.min.x` | [x] |
| C22 | `c2AABBtoPoint` | `B.y < A.min.y` | [x] |
| C23 | `c2AABBtoPoint` | `B.x > A.max.x` | [x] |
| C24 | `c2AABBtoPoint` | `B.y > A.max.y` | [x] |
| C25 | `c2CircleToPoint` | point strictly inside (`d2 < r*r`) | [x] |
| C26 | `c2CircleToPoint` | point exactly on the radius (`d2 == r*r`) | [x] |
| C27 | `c2CircleToPoint` | point outside (`d2 > r*r`), including negative radius behavior | [x] |
| C28 | `c2RaytoCircle` | negative discriminant miss | [x] |
| C29 | `c2RaytoCircle` | intersection behind ray (`t < 0`) | [x] |
| C30 | `c2RaytoCircle` | intersection beyond finite ray interval (`t > A.t`) | [x] |
| C31 | `c2RaytoCircle` | hit, including tangent (`disc == 0`) and start-on-boundary | [x] |
| C32 | `c2RaytoAABB` | segment bounding box does not overlap target | [x] |
| C33 | `c2RaytoAABB` | bounding boxes overlap but separating-axis `d > 0` | [x] |
| C34 | `c2RaytoAABB` | hit selects `(-1,0)` normal, including tie priority | [x] |
| C35 | `c2RaytoAABB` | hit selects `(1,0)` normal | [x] |
| C36 | `c2RaytoAABB` | hit selects `(0,-1)` normal | [x] |
| C37 | `c2RaytoAABB` | hit selects `(0,1)` normal | [x] |
| C38 | `c2RaytoAABB` | all plane parameters are greater than one | [x] |
| C39 | `c2RaytoCapsule` | start inside rectangular body | [x] |
| C40 | `c2RaytoCapsule` | start inside endpoint-A circle only | [x] |
| C41 | `c2RaytoCapsule` | start inside endpoint-B circle only | [x] |
| C42 | `c2RaytoCapsule` | path never approaches capsule radius | [x] |
| C43 | `c2RaytoCapsule` | start within radius strip and branch to endpoint A | [x] |
| C44 | `c2RaytoCapsule` | start within radius strip and branch to endpoint B | [x] |
| C45 | `c2RaytoCapsule` | crossing candidate lies before body (`y <= 0`), dispatch endpoint A | [x] |
| C46 | `c2RaytoCapsule` | crossing candidate lies after body (`y >= yBb.y`), dispatch endpoint B | [x] |
| C47 | `c2RaytoCapsule` | side-wall hit with positive local x (`c > 0`) | [x] |
| C48 | `c2RaytoCapsule` | side-wall hit with non-positive local x (`c <= 0`) | [x] |
| C49 | `c2CastRay` | `typeB == C2_TYPE_CIRCLE` with circle storage | [x] |
| C50 | `c2CastRay` | `typeB == C2_TYPE_AABB` with AABB storage | [x] |
| C51 | `c2CastRay` | `typeB == C2_TYPE_CAPSULE` with capsule storage | [x] |
| C52 | `spec_ray` | generated finite ray hits circle | [x] |
| C53 | `spec_ray` | generated finite ray misses circle | [x] |
| C54 | `spec_ray` | mouse and ray origin coincide, producing zero-vector normalization | [x] |
| C55 | `spec_ray` | zero and negative circle radii | [x] |

No feature cross-product exists: `Cargo.toml` has no `[features]` section, so
the default/no-feature build is the sole configuration.
