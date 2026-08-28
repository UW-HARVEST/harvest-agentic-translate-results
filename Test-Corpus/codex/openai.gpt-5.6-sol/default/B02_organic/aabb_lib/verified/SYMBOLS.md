# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-OooqjH.so
nm -D --defined-only target/release/libaabb_lib.so
```

The C shared object has 38 defined public symbols. All 38 are exported by the
Rust shared object with the exact same names.

| # | C symbol | Rust export |
|---:|----------|-------------|
| 1 | `aabb` | present |
| 2 | `c22` | present |
| 3 | `c23` | present |
| 4 | `c2AABBtoAABB` | present |
| 5 | `c2AABBtoCapsule` | present |
| 6 | `c2Add` | present |
| 7 | `c2BBVerts` | present |
| 8 | `c2CCW90` | present |
| 9 | `c2CapsuletoCapsule` | present |
| 10 | `c2CircletoAABB` | present |
| 11 | `c2CircletoCapsule` | present |
| 12 | `c2CircletoCircle` | present |
| 13 | `c2Clampv` | present |
| 14 | `c2Collided` | present |
| 15 | `c2D` | present |
| 16 | `c2Det2` | present |
| 17 | `c2Div` | present |
| 18 | `c2Dot` | present |
| 19 | `c2GJK` | present |
| 20 | `c2GJKSimplexMetric` | present |
| 21 | `c2L` | present |
| 22 | `c2Len` | present |
| 23 | `c2MakeProxy` | present |
| 24 | `c2Maxv` | present |
| 25 | `c2Minv` | present |
| 26 | `c2Mulrv` | present |
| 27 | `c2MulrvT` | present |
| 28 | `c2Mulvs` | present |
| 29 | `c2Mulxv` | present |
| 30 | `c2Neg` | present |
| 31 | `c2Norm` | present |
| 32 | `c2RotIdentity` | present |
| 33 | `c2Skew` | present |
| 34 | `c2Sub` | present |
| 35 | `c2Support` | present |
| 36 | `c2V` | present |
| 37 | `c2Witness` | present |
| 38 | `c2xIdentity` | present |

Missing C symbols in Rust: **0**.

The C object's strong undefined symbol is `sqrtf@GLIBC_2.2.5`, supplied by
libm/libc. Its other undefined entries are weak ELF runtime hooks.
