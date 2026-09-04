# Dynamic symbol surface

Mechanically captured from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-lWqf1W.so
nm -D --defined-only target/release/libpoly_ray_lib.so
```

The C shared object has 28 defined public dynamic symbols. Rust exports all 28
with exact names.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `c2AABBtoAABB` | present |
| 2 | `c2AABBtoPoint` | present |
| 3 | `c2Absv` | present |
| 4 | `c2Add` | present |
| 5 | `c2CCW90` | present |
| 6 | `c2CastRay` | present |
| 7 | `c2CircleToPoint` | present |
| 8 | `c2Div` | present |
| 9 | `c2Dot` | present |
| 10 | `c2Len` | present |
| 11 | `c2Maxv` | present |
| 12 | `c2Minv` | present |
| 13 | `c2MulmvT` | present |
| 14 | `c2Mulrv` | present |
| 15 | `c2MulrvT` | present |
| 16 | `c2Mulvs` | present |
| 17 | `c2MulxvT` | present |
| 18 | `c2Norm` | present |
| 19 | `c2RaytoAABB` | present |
| 20 | `c2RaytoCapsule` | present |
| 21 | `c2RaytoCircle` | present |
| 22 | `c2RaytoPoly` | present |
| 23 | `c2RotIdentity` | present |
| 24 | `c2Skew` | present |
| 25 | `c2Sub` | present |
| 26 | `c2V` | present |
| 27 | `c2xIdentity` | present |
| 28 | `poly_ray` | present |

Completion gate: [x] Missing C-defined symbols in Rust: **0**.

The C object has one strong external function dependency, `sqrtf`; it is a
libm/libc runtime dependency rather than a symbol defined by this library.
