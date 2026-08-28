# Configuration surface

There are no compile-time C modes and no Cargo features. The runtime axes are
the float comparison branches, geometric relationship/shape, three
`c2CastRay` type values, and the three-bit result composed by `gen_ray`.
Each row is a branch-distinct valid query family derived from `src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|---|
| C01 | `c2V` | arbitrary scalar components, including signed zero and IEEE special values | [x] |
| C02 | `c2Dot` | arbitrary finite vector pairs | [x] |
| C03 | `c2Len` | finite nonzero, zero, axis-aligned, and diagonal vectors | [x] |
| C04 | `c2Add` | arbitrary finite vector pairs | [x] |
| C05 | `c2Sub` | arbitrary finite vector pairs | [x] |
| C06 | `c2Mulvs` | arbitrary finite vectors and finite scalar | [x] |
| C07 | `c2Div` | arbitrary finite vector and nonzero finite divisor | [x] |
| C08 | `c2Norm` | nonzero finite axis-aligned and general vectors | [x] |
| C09 | `c2Minv` | A supplies both selected components | [x] |
| C10 | `c2Minv` | A supplies x; B supplies y | [x] |
| C11 | `c2Minv` | B supplies x; A supplies y | [x] |
| C12 | `c2Minv` | B supplies both selected components, including equal boundaries | [x] |
| C13 | `c2Maxv` | A supplies both selected components | [x] |
| C14 | `c2Maxv` | A supplies x; B supplies y | [x] |
| C15 | `c2Maxv` | B supplies x; A supplies y | [x] |
| C16 | `c2Maxv` | B supplies both selected components, including equal boundaries | [x] |
| C17 | `c2Skew` | arbitrary finite vector | [x] |
| C18 | `c2Absv` | x nonnegative, y nonnegative | [x] |
| C19 | `c2Absv` | x negative, y nonnegative | [x] |
| C20 | `c2Absv` | x nonnegative, y negative | [x] |
| C21 | `c2Absv` | x negative, y negative | [x] |
| C22 | `c2CCW90` | arbitrary finite vector | [x] |
| C23 | `c2MulmvT` | arbitrary finite 2x2 matrix and vector | [x] |
| C24 | `c2AABBtoAABB` | interior overlap/containment | [x] |
| C25 | `c2AABBtoAABB` | boundary-only edge or corner contact | [x] |
| C26 | `c2AABBtoPoint` | point inside or exactly on any boundary | [x] |
| C27 | `c2CircleToPoint` | point strictly inside positive-radius circle | [x] |
| C28 | `c2CircleToPoint` | point on or outside circle; strict `<` is false | [x] |
| C29 | `c2RaytoCircle` | two-root intersection, first root in `[0,A.t]` | [x] |
| C30 | `c2RaytoCircle` | tangent (`disc == 0`) in range | [x] |
| C31 | `c2RaytoCircle` | ray starts on boundary with accepted `t == 0` | [x] |
| C32 | `c2RaytoCircle` | negative discriminant miss | [x] |
| C33 | `c2RaytoCircle` | circle behind ray (`t < 0`) | [x] |
| C34 | `c2RaytoCircle` | first root beyond finite segment (`t > A.t`) | [x] |
| C35 | `c2RaytoAABB` | hit selected by `t0`: min-x face, normal `(-1,0)` | [x] |
| C36 | `c2RaytoAABB` | hit selected by `t1`: max-x face, normal `(1,0)` | [x] |
| C37 | `c2RaytoAABB` | hit selected by `t2`: min-y face, normal `(0,-1)` | [x] |
| C38 | `c2RaytoAABB` | hit selected by `t3`: max-y face, normal `(0,1)` | [x] |
| C39 | `c2RaytoAABB` | segment begins inside/on box | [x] |
| C40 | `c2RaytoAABB` | segment broadphase AABB is disjoint | [x] |
| C41 | `c2RaytoAABB` | broadphase overlaps but line separating-axis test rejects | [x] |
| C42 | `c2RaytoCapsule` | start inside rectangular body | [x] |
| C43 | `c2RaytoCapsule` | start inside cap A only | [x] |
| C44 | `c2RaytoCapsule` | start inside cap B only | [x] |
| C45 | `c2RaytoCapsule` | side hit on positive local x; normal is `M.x` | [x] |
| C46 | `c2RaytoCapsule` | side hit on negative local x; normal is `c2Skew(M.y)` | [x] |
| C47 | `c2RaytoCapsule` | crossing resolves through cap A | [x] |
| C48 | `c2RaytoCapsule` | crossing resolves through cap B | [x] |
| C49 | `c2RaytoCapsule` | outside body/caps with no side crossing | [x] |
| C50 | `c2RaytoCapsule` | degenerate capsule (`a == b`) accepted without validation | [x] |
| C51 | `c2CastRay` | `typeB == C2_TYPE_CIRCLE` (`0`) | [x] |
| C52 | `c2CastRay` | `typeB == C2_TYPE_AABB` (`1`) | [x] |
| C53 | `c2CastRay` | `typeB == C2_TYPE_CAPSULE` (`2`) | [x] |
| C54 | `gen_ray` | hit mask `0b000`: no shape hit | [x] |
| C55 | `gen_ray` | hit mask `0b001`: circle only | [x] |
| C56 | `gen_ray` | hit mask `0b010`: capsule only | [x] |
| C57 | `gen_ray` | hit mask `0b011`: circle + capsule | [x] |
| C58 | `gen_ray` | hit mask `0b100`: AABB only | [x] |
| C59 | `gen_ray` | hit mask `0b101`: circle + AABB | [x] |
| C60 | `gen_ray` | hit mask `0b110`: capsule + AABB | [x] |
| C61 | `gen_ray` | hit mask `0b111`: all three shapes | [x] |
| C62 | `gen_ray` | zero-length ray (`mouse == ray.p`) accepted without validation | [x] |
| C63 | circle/capsule entry points | zero and negative radii accepted without validation | [x] |
| C64 | AABB entry points | reversed min/max bounds accepted without validation | [x] |
| C65 | float/vector/shape entry points | NaN and infinity operands through comparison fall-through paths | [x] |
