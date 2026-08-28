# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-UzAYkl.so
nm -D --defined-only target/release/libgen_ray_lib.so
```

The C library has 22 globally defined functions. All 22 are present in the
Rust library with the exact same names.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `c2V` | `T` | `c2V` | [x] |
| 2 | `c2Dot` | `T` | `c2Dot` | [x] |
| 3 | `c2Len` | `T` | `c2Len` | [x] |
| 4 | `c2Add` | `T` | `c2Add` | [x] |
| 5 | `c2Sub` | `T` | `c2Sub` | [x] |
| 6 | `c2Mulvs` | `T` | `c2Mulvs` | [x] |
| 7 | `c2Div` | `T` | `c2Div` | [x] |
| 8 | `c2Norm` | `T` | `c2Norm` | [x] |
| 9 | `c2Minv` | `T` | `c2Minv` | [x] |
| 10 | `c2Maxv` | `T` | `c2Maxv` | [x] |
| 11 | `c2Skew` | `T` | `c2Skew` | [x] |
| 12 | `c2Absv` | `T` | `c2Absv` | [x] |
| 13 | `c2RaytoCircle` | `T` | `c2RaytoCircle` | [x] |
| 14 | `c2AABBtoAABB` | `T` | `c2AABBtoAABB` | [x] |
| 15 | `c2RaytoAABB` | `T` | `c2RaytoAABB` | [x] |
| 16 | `c2CCW90` | `T` | `c2CCW90` | [x] |
| 17 | `c2MulmvT` | `T` | `c2MulmvT` | [x] |
| 18 | `c2AABBtoPoint` | `T` | `c2AABBtoPoint` | [x] |
| 19 | `c2CircleToPoint` | `T` | `c2CircleToPoint` | [x] |
| 20 | `c2RaytoCapsule` | `T` | `c2RaytoCapsule` | [x] |
| 21 | `c2CastRay` | `T` | `c2CastRay` | [x] |
| 22 | `gen_ray` | `T` | `gen_ray` | [x] |

The C library's only strong undefined function is `sqrtf@GLIBC_2.2.5`.
The remaining undefined entries are weak ELF runtime hooks. There are no
missing C symbols and no undefined non-libc implementation symbols in Rust.
