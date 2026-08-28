# Dynamic symbol surface

Generated from:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-ERBdN0.so
nm -D --defined-only target/release/libomni_manifold_lib.so
```

Only defined global text/weak symbols (`T`/`W`) are part of this table. The C
library exports 46 such symbols.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `c22` | [x] |
| 2 | `c23` | [x] |
| 3 | `c2AABBtoAABBManifold` | [x] |
| 4 | `c2AABBtoCapsuleManifold` | [x] |
| 5 | `c2Absv` | [x] |
| 6 | `c2Add` | [x] |
| 7 | `c2BBVerts` | [x] |
| 8 | `c2CCW90` | [x] |
| 9 | `c2CapsuletoCapsuleManifold` | [x] |
| 10 | `c2CapsuletoPolyManifold` | [x] |
| 11 | `c2CircletoAABBManifold` | [x] |
| 12 | `c2CircletoCapsuleManifold` | [x] |
| 13 | `c2CircletoCircleManifold` | [x] |
| 14 | `c2Clampv` | [x] |
| 15 | `c2Collide` | [x] |
| 16 | `c2D` | [x] |
| 17 | `c2Det2` | [x] |
| 18 | `c2Dist` | [x] |
| 19 | `c2Div` | [x] |
| 20 | `c2Dot` | [x] |
| 21 | `c2GJK` | [x] |
| 22 | `c2GJKSimplexMetric` | [x] |
| 23 | `c2Intersect` | [x] |
| 24 | `c2L` | [x] |
| 25 | `c2Len` | [x] |
| 26 | `c2MakeProxy` | [x] |
| 27 | `c2Maxv` | [x] |
| 28 | `c2Minv` | [x] |
| 29 | `c2Mulrv` | [x] |
| 30 | `c2MulrvT` | [x] |
| 31 | `c2Mulvs` | [x] |
| 32 | `c2Mulxv` | [x] |
| 33 | `c2MulxvT` | [x] |
| 34 | `c2Neg` | [x] |
| 35 | `c2Norm` | [x] |
| 36 | `c2Norms` | [x] |
| 37 | `c2PlaneAt` | [x] |
| 38 | `c2RotIdentity` | [x] |
| 39 | `c2Skew` | [x] |
| 40 | `c2Sub` | [x] |
| 41 | `c2Support` | [x] |
| 42 | `c2V` | [x] |
| 43 | `c2Witness` | [x] |
| 44 | `c2xIdentity` | [x] |
| 45 | `omni_manifold` | [x] |
| 46 | `ptr_from_parts` | [x] |

Missing from Rust:

```text
(none)
```
