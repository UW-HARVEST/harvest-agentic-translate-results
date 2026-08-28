# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-ssxMbU.so
nm -D --defined-only target/release/libomni_collide_lib.so
```

The C shared object has 39 defined public symbols. The Rust shared object
exports all 39 with the same names.

| # | C symbol | ELF type | Rust export |
|---:|---|:---:|:---:|
| 1 | `c22` | T | present |
| 2 | `c23` | T | present |
| 3 | `c2AABBtoAABB` | T | present |
| 4 | `c2AABBtoCapsule` | T | present |
| 5 | `c2Add` | T | present |
| 6 | `c2BBVerts` | T | present |
| 7 | `c2CCW90` | T | present |
| 8 | `c2CapsuletoCapsule` | T | present |
| 9 | `c2CircletoAABB` | T | present |
| 10 | `c2CircletoCapsule` | T | present |
| 11 | `c2CircletoCircle` | T | present |
| 12 | `c2Clampv` | T | present |
| 13 | `c2Collided` | T | present |
| 14 | `c2D` | T | present |
| 15 | `c2Det2` | T | present |
| 16 | `c2Div` | T | present |
| 17 | `c2Dot` | T | present |
| 18 | `c2GJK` | T | present |
| 19 | `c2GJKSimplexMetric` | T | present |
| 20 | `c2L` | T | present |
| 21 | `c2Len` | T | present |
| 22 | `c2MakeProxy` | T | present |
| 23 | `c2Maxv` | T | present |
| 24 | `c2Minv` | T | present |
| 25 | `c2Mulrv` | T | present |
| 26 | `c2MulrvT` | T | present |
| 27 | `c2Mulvs` | T | present |
| 28 | `c2Mulxv` | T | present |
| 29 | `c2Neg` | T | present |
| 30 | `c2Norm` | T | present |
| 31 | `c2RotIdentity` | T | present |
| 32 | `c2Skew` | T | present |
| 33 | `c2Sub` | T | present |
| 34 | `c2Support` | T | present |
| 35 | `c2V` | T | present |
| 36 | `c2Witness` | T | present |
| 37 | `c2xIdentity` | T | present |
| 38 | `omni_collide` | T | present |
| 39 | `ptr_from_parts` | T | present |

Missing C symbols in Rust: **0**.

The C object's undefined dynamic symbols are only runtime/libc/libm symbols:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`, `malloc`, and `sqrtf`.
