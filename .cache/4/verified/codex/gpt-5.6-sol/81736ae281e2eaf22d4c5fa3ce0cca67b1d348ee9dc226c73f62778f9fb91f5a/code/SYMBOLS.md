# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The C shared object exports 22 public symbols. The Rust shared object exports
all 22 with exact names.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `c2AABBtoAABB` | [x] |
| 2 | `c2AABBtoPoint` | [x] |
| 3 | `c2Absv` | [x] |
| 4 | `c2Add` | [x] |
| 5 | `c2CCW90` | [x] |
| 6 | `c2CastRay` | [x] |
| 7 | `c2CircleToPoint` | [x] |
| 8 | `c2Div` | [x] |
| 9 | `c2Dot` | [x] |
| 10 | `c2Len` | [x] |
| 11 | `c2Maxv` | [x] |
| 12 | `c2Minv` | [x] |
| 13 | `c2MulmvT` | [x] |
| 14 | `c2Mulvs` | [x] |
| 15 | `c2Norm` | [x] |
| 16 | `c2RaytoAABB` | [x] |
| 17 | `c2RaytoCapsule` | [x] |
| 18 | `c2RaytoCircle` | [x] |
| 19 | `c2Skew` | [x] |
| 20 | `c2Sub` | [x] |
| 21 | `c2V` | [x] |
| 22 | `gen_ray` | [x] |

Missing C symbols in Rust: **0**.

Undefined non-runtime symbols in Rust: **0**.
