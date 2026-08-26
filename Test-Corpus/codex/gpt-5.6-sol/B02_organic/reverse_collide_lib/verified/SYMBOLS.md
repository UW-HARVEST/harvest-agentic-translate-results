# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The C and Rust release libraries each export 38 defined API symbols. The
sorted `comm` diff is empty in both directions.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `c22` | [x] |
| 2 | `c23` | [x] |
| 3 | `c2AABBtoAABB` | [x] |
| 4 | `c2AABBtoCapsule` | [x] |
| 5 | `c2Add` | [x] |
| 6 | `c2BBVerts` | [x] |
| 7 | `c2CCW90` | [x] |
| 8 | `c2CapsuletoCapsule` | [x] |
| 9 | `c2CircletoAABB` | [x] |
| 10 | `c2CircletoCapsule` | [x] |
| 11 | `c2CircletoCircle` | [x] |
| 12 | `c2Clampv` | [x] |
| 13 | `c2Collided` | [x] |
| 14 | `c2D` | [x] |
| 15 | `c2Det2` | [x] |
| 16 | `c2Div` | [x] |
| 17 | `c2Dot` | [x] |
| 18 | `c2GJK` | [x] |
| 19 | `c2GJKSimplexMetric` | [x] |
| 20 | `c2L` | [x] |
| 21 | `c2Len` | [x] |
| 22 | `c2MakeProxy` | [x] |
| 23 | `c2Maxv` | [x] |
| 24 | `c2Minv` | [x] |
| 25 | `c2Mulrv` | [x] |
| 26 | `c2MulrvT` | [x] |
| 27 | `c2Mulvs` | [x] |
| 28 | `c2Mulxv` | [x] |
| 29 | `c2Neg` | [x] |
| 30 | `c2Norm` | [x] |
| 31 | `c2RotIdentity` | [x] |
| 32 | `c2Skew` | [x] |
| 33 | `c2Sub` | [x] |
| 34 | `c2Support` | [x] |
| 35 | `c2V` | [x] |
| 36 | `c2Witness` | [x] |
| 37 | `c2xIdentity` | [x] |
| 38 | `reverse_collide` | [x] |

The C library's only strong undefined dynamic dependency is the libc/libm
symbol `sqrtf`. Its remaining undefined entries are weak ELF runtime symbols.
The Rust library has no undefined non-runtime project symbols.
