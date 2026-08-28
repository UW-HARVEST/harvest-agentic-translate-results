# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-3V9gnL.so
nm -D --defined-only target/release/libspec_ray_lib.so
```

The C shared object exports 22 functions. The C header declares only
`spec_ray`, but the default ELF visibility also exports the 21 non-static
helpers in `src/lib.c`.

| # | C symbol | Rust export | Status |
|---|----------|-------------|--------|
| 1 | `c2V` | `c2V` | present |
| 2 | `c2Dot` | `c2Dot` | present |
| 3 | `c2Len` | `c2Len` | present |
| 4 | `c2Add` | `c2Add` | present |
| 5 | `c2Sub` | `c2Sub` | present |
| 6 | `c2Mulvs` | `c2Mulvs` | present |
| 7 | `c2Div` | `c2Div` | present |
| 8 | `c2Norm` | `c2Norm` | present |
| 9 | `c2Minv` | `c2Minv` | present |
| 10 | `c2Maxv` | `c2Maxv` | present |
| 11 | `c2Skew` | `c2Skew` | present |
| 12 | `c2Absv` | `c2Absv` | present |
| 13 | `c2RaytoCircle` | `c2RaytoCircle` | present |
| 14 | `c2AABBtoAABB` | `c2AABBtoAABB` | present |
| 15 | `c2RaytoAABB` | `c2RaytoAABB` | present |
| 16 | `c2CCW90` | `c2CCW90` | present |
| 17 | `c2MulmvT` | `c2MulmvT` | present |
| 18 | `c2AABBtoPoint` | `c2AABBtoPoint` | present |
| 19 | `c2CircleToPoint` | `c2CircleToPoint` | present |
| 20 | `c2RaytoCapsule` | `c2RaytoCapsule` | present |
| 21 | `c2CastRay` | `c2CastRay` | present |
| 22 | `spec_ray` | `spec_ray` | present |

Missing C symbols in Rust: **0**.

The C object has one strong external dependency, `sqrtf`. The optimized Rust
object has only runtime/libc undefined symbols, not missing symbols from this
library.

- [x] Every C dynamic symbol has an exact-name Rust export.
- [x] No C source module is missing from the translation.
