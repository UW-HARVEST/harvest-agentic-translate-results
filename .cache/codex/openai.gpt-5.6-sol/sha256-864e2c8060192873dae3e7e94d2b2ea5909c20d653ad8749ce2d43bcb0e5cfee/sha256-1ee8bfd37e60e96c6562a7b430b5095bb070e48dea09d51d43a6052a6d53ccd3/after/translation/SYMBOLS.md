# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-3Nv5PV.so
nm -D --defined-only target/release/libaabb_lib.so
```

The C shared object exports 38 public functions. The Rust shared object exports
the same 38 names. `comm` over the sorted symbol-name sets reports no missing
or extra API symbols.

| # | C symbol | Rust export |
|---:|----------|:-----------:|
| 1 | `aabb` | [x] |
| 2 | `c22` | [x] |
| 3 | `c23` | [x] |
| 4 | `c2AABBtoAABB` | [x] |
| 5 | `c2AABBtoCapsule` | [x] |
| 6 | `c2Add` | [x] |
| 7 | `c2BBVerts` | [x] |
| 8 | `c2CCW90` | [x] |
| 9 | `c2CapsuletoCapsule` | [x] |
| 10 | `c2CircletoAABB` | [x] |
| 11 | `c2CircletoCapsule` | [x] |
| 12 | `c2CircletoCircle` | [x] |
| 13 | `c2Clampv` | [x] |
| 14 | `c2Collided` | [x] |
| 15 | `c2D` | [x] |
| 16 | `c2Det2` | [x] |
| 17 | `c2Div` | [x] |
| 18 | `c2Dot` | [x] |
| 19 | `c2GJK` | [x] |
| 20 | `c2GJKSimplexMetric` | [x] |
| 21 | `c2L` | [x] |
| 22 | `c2Len` | [x] |
| 23 | `c2MakeProxy` | [x] |
| 24 | `c2Maxv` | [x] |
| 25 | `c2Minv` | [x] |
| 26 | `c2Mulrv` | [x] |
| 27 | `c2MulrvT` | [x] |
| 28 | `c2Mulvs` | [x] |
| 29 | `c2Mulxv` | [x] |
| 30 | `c2Neg` | [x] |
| 31 | `c2Norm` | [x] |
| 32 | `c2RotIdentity` | [x] |
| 33 | `c2Skew` | [x] |
| 34 | `c2Sub` | [x] |
| 35 | `c2Support` | [x] |
| 36 | `c2V` | [x] |
| 37 | `c2Witness` | [x] |
| 38 | `c2xIdentity` | [x] |

Completion checks:

- [x] No C API symbol is missing from Rust.
- [x] No Rust API symbol is extra relative to C.
- [x] `ldd -r target/release/libaabb_lib.so` reports no unresolved symbol.
