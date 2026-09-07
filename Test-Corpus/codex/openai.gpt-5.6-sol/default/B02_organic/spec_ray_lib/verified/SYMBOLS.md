# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-29Su3v.so
nm -D --defined-only target/release/libspec_ray_lib.so
```

The C shared library exports 22 public symbols. All 22 are currently present
under the exact same name in the Rust shared library.

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
| 14 | `c2Mulvs` | present |
| 15 | `c2Norm` | present |
| 16 | `c2RaytoAABB` | present |
| 17 | `c2RaytoCapsule` | present |
| 18 | `c2RaytoCircle` | present |
| 19 | `c2Skew` | present |
| 20 | `c2Sub` | present |
| 21 | `c2V` | present |
| 22 | `spec_ray` | present |

Missing C symbols in Rust: **0**

Completion gate: [x] exact 22-symbol dynamic export set, with no C-only or
Rust-only symbols.
