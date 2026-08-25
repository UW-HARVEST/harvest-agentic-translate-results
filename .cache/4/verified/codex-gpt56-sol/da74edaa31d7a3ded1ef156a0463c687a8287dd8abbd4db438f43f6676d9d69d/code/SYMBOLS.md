# Exported Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libomni_manifold_lib.so
```

The default C build exports 46 library-defined symbols. The Rust library
exports every symbol with the exact same name. `comm -23` is empty.

- [x] Zero C-defined symbols are missing from Rust.
- [x] C has no undefined non-toolchain/non-libc/non-libm symbols.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `c22` | present |
| 2 | `c23` | present |
| 3 | `c2AABBtoAABBManifold` | present |
| 4 | `c2AABBtoCapsuleManifold` | present |
| 5 | `c2Absv` | present |
| 6 | `c2Add` | present |
| 7 | `c2BBVerts` | present |
| 8 | `c2CCW90` | present |
| 9 | `c2CapsuletoCapsuleManifold` | present |
| 10 | `c2CapsuletoPolyManifold` | present |
| 11 | `c2CircletoAABBManifold` | present |
| 12 | `c2CircletoCapsuleManifold` | present |
| 13 | `c2CircletoCircleManifold` | present |
| 14 | `c2Clampv` | present |
| 15 | `c2Collide` | present |
| 16 | `c2D` | present |
| 17 | `c2Det2` | present |
| 18 | `c2Dist` | present |
| 19 | `c2Div` | present |
| 20 | `c2Dot` | present |
| 21 | `c2GJK` | present |
| 22 | `c2GJKSimplexMetric` | present |
| 23 | `c2Intersect` | present |
| 24 | `c2L` | present |
| 25 | `c2Len` | present |
| 26 | `c2MakeProxy` | present |
| 27 | `c2Maxv` | present |
| 28 | `c2Minv` | present |
| 29 | `c2Mulrv` | present |
| 30 | `c2MulrvT` | present |
| 31 | `c2Mulvs` | present |
| 32 | `c2Mulxv` | present |
| 33 | `c2MulxvT` | present |
| 34 | `c2Neg` | present |
| 35 | `c2Norm` | present |
| 36 | `c2Norms` | present |
| 37 | `c2PlaneAt` | present |
| 38 | `c2RotIdentity` | present |
| 39 | `c2Skew` | present |
| 40 | `c2Sub` | present |
| 41 | `c2Support` | present |
| 42 | `c2V` | present |
| 43 | `c2Witness` | present |
| 44 | `c2xIdentity` | present |
| 45 | `omni_manifold` | present |
| 46 | `ptr_from_parts` | present |

The C library's undefined dynamic symbols are only toolchain/libc/libm
dependencies: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`, `malloc`, and `sqrtf`.
