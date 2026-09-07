# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-uMUtVU.so
nm -D --defined-only target/release/libomni_collide_lib.so
```

The C shared object exports 39 defined public symbols. The Rust shared object
exports the same 39 names.

| # | C symbol | C kind | Rust export |
|---|----------|--------|-------------|
| 1 | `c22` | `T` | [x] |
| 2 | `c23` | `T` | [x] |
| 3 | `c2AABBtoAABB` | `T` | [x] |
| 4 | `c2AABBtoCapsule` | `T` | [x] |
| 5 | `c2Add` | `T` | [x] |
| 6 | `c2BBVerts` | `T` | [x] |
| 7 | `c2CCW90` | `T` | [x] |
| 8 | `c2CapsuletoCapsule` | `T` | [x] |
| 9 | `c2CircletoAABB` | `T` | [x] |
| 10 | `c2CircletoCapsule` | `T` | [x] |
| 11 | `c2CircletoCircle` | `T` | [x] |
| 12 | `c2Clampv` | `T` | [x] |
| 13 | `c2Collided` | `T` | [x] |
| 14 | `c2D` | `T` | [x] |
| 15 | `c2Det2` | `T` | [x] |
| 16 | `c2Div` | `T` | [x] |
| 17 | `c2Dot` | `T` | [x] |
| 18 | `c2GJK` | `T` | [x] |
| 19 | `c2GJKSimplexMetric` | `T` | [x] |
| 20 | `c2L` | `T` | [x] |
| 21 | `c2Len` | `T` | [x] |
| 22 | `c2MakeProxy` | `T` | [x] |
| 23 | `c2Maxv` | `T` | [x] |
| 24 | `c2Minv` | `T` | [x] |
| 25 | `c2Mulrv` | `T` | [x] |
| 26 | `c2MulrvT` | `T` | [x] |
| 27 | `c2Mulvs` | `T` | [x] |
| 28 | `c2Mulxv` | `T` | [x] |
| 29 | `c2Neg` | `T` | [x] |
| 30 | `c2Norm` | `T` | [x] |
| 31 | `c2RotIdentity` | `T` | [x] |
| 32 | `c2Skew` | `T` | [x] |
| 33 | `c2Sub` | `T` | [x] |
| 34 | `c2Support` | `T` | [x] |
| 35 | `c2V` | `T` | [x] |
| 36 | `c2Witness` | `T` | [x] |
| 37 | `c2xIdentity` | `T` | [x] |
| 38 | `omni_collide` | `T` | [x] |
| 39 | `ptr_from_parts` | `T` | [x] |

The C object's undefined dynamic dependencies are only `malloc`, `sqrtf`, and
standard ELF/libc runtime weak symbols. There are no undefined project symbols.

Missing Rust symbols: **0**
