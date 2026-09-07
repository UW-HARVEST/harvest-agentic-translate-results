# Exported symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-eNZ0uT.so
nm -D --defined-only target/release/libcapsule_lib.so
```

The C library exports 38 defined public symbols. The Rust library exports every
one with the exact same name.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `c22` | present |
| 2 | `c23` | present |
| 3 | `c2AABBtoAABB` | present |
| 4 | `c2AABBtoCapsule` | present |
| 5 | `c2Add` | present |
| 6 | `c2BBVerts` | present |
| 7 | `c2CCW90` | present |
| 8 | `c2CapsuletoCapsule` | present |
| 9 | `c2CircletoAABB` | present |
| 10 | `c2CircletoCapsule` | present |
| 11 | `c2CircletoCircle` | present |
| 12 | `c2Clampv` | present |
| 13 | `c2Collided` | present |
| 14 | `c2D` | present |
| 15 | `c2Det2` | present |
| 16 | `c2Div` | present |
| 17 | `c2Dot` | present |
| 18 | `c2GJK` | present |
| 19 | `c2GJKSimplexMetric` | present |
| 20 | `c2L` | present |
| 21 | `c2Len` | present |
| 22 | `c2MakeProxy` | present |
| 23 | `c2Maxv` | present |
| 24 | `c2Minv` | present |
| 25 | `c2Mulrv` | present |
| 26 | `c2MulrvT` | present |
| 27 | `c2Mulvs` | present |
| 28 | `c2Mulxv` | present |
| 29 | `c2Neg` | present |
| 30 | `c2Norm` | present |
| 31 | `c2RotIdentity` | present |
| 32 | `c2Skew` | present |
| 33 | `c2Sub` | present |
| 34 | `c2Support` | present |
| 35 | `c2V` | present |
| 36 | `c2Witness` | present |
| 37 | `c2xIdentity` | present |
| 38 | `capsule` | present |

Missing C-defined symbols in Rust: **0**.

The only undefined strong dependency in the C shared object is libc/libm
`sqrtf`. Rust has runtime/libc undefined dependencies but no undefined
project-library symbols.
