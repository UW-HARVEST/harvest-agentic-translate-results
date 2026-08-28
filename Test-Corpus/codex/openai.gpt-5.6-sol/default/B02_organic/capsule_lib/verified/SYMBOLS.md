# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-nTsGE1.so
nm -D --defined-only target/release/libcapsule_lib.so
```

| # | C symbol | kind | Rust export |
|---|----------|------|-------------|
| 1 | `c22` | `T` | present |
| 2 | `c23` | `T` | present |
| 3 | `c2AABBtoAABB` | `T` | present |
| 4 | `c2AABBtoCapsule` | `T` | present |
| 5 | `c2Add` | `T` | present |
| 6 | `c2BBVerts` | `T` | present |
| 7 | `c2CCW90` | `T` | present |
| 8 | `c2CapsuletoCapsule` | `T` | present |
| 9 | `c2CircletoAABB` | `T` | present |
| 10 | `c2CircletoCapsule` | `T` | present |
| 11 | `c2CircletoCircle` | `T` | present |
| 12 | `c2Clampv` | `T` | present |
| 13 | `c2Collided` | `T` | present |
| 14 | `c2D` | `T` | present |
| 15 | `c2Det2` | `T` | present |
| 16 | `c2Div` | `T` | present |
| 17 | `c2Dot` | `T` | present |
| 18 | `c2GJK` | `T` | present |
| 19 | `c2GJKSimplexMetric` | `T` | present |
| 20 | `c2L` | `T` | present |
| 21 | `c2Len` | `T` | present |
| 22 | `c2MakeProxy` | `T` | present |
| 23 | `c2Maxv` | `T` | present |
| 24 | `c2Minv` | `T` | present |
| 25 | `c2Mulrv` | `T` | present |
| 26 | `c2MulrvT` | `T` | present |
| 27 | `c2Mulvs` | `T` | present |
| 28 | `c2Mulxv` | `T` | present |
| 29 | `c2Neg` | `T` | present |
| 30 | `c2Norm` | `T` | present |
| 31 | `c2RotIdentity` | `T` | present |
| 32 | `c2Skew` | `T` | present |
| 33 | `c2Sub` | `T` | present |
| 34 | `c2Support` | `T` | present |
| 35 | `c2V` | `T` | present |
| 36 | `c2Witness` | `T` | present |
| 37 | `c2xIdentity` | `T` | present |
| 38 | `capsule` | `T` | present |

Defined-symbol counts: C = 38, Rust = 38. Missing C symbols in Rust = 0.

- [x] Final defined-symbol diff is empty.
- [x] No undefined non-runtime project symbols exist in the Rust library.

The C library's only strong undefined symbol is the libc/libm `sqrtf` symbol.
The remaining C undefined entries are weak ELF runtime symbols. Rust's undefined
entries are libc, pthread, unwinding, and compiler/runtime dependencies; there
are no undefined project symbols.
