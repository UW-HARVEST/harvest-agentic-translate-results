# Dynamic symbol surface

Generated from:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libgjk_cache_lib.so
```

The C shared object has 31 globally defined dynamic symbols. `sqrtf` is its
only strong undefined symbol and is supplied by `libm`. The Rust shared object
has no undefined project symbols; its undefined symbols are platform/runtime
imports.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `c2V` | `T` | `c2V` | [x] |
| 2 | `c2Mulvs` | `T` | `c2Mulvs` | [x] |
| 3 | `c2Maxv` | `T` | `c2Maxv` | [x] |
| 4 | `c2Minv` | `T` | `c2Minv` | [x] |
| 5 | `c2Clampv` | `T` | `c2Clampv` | [x] |
| 6 | `c2Sub` | `T` | `c2Sub` | [x] |
| 7 | `c2Dot` | `T` | `c2Dot` | [x] |
| 8 | `c2RotIdentity` | `T` | `c2RotIdentity` | [x] |
| 9 | `c2xIdentity` | `T` | `c2xIdentity` | [x] |
| 10 | `c2BBVerts` | `T` | `c2BBVerts` | [x] |
| 11 | `c2MakeProxy` | `T` | `c2MakeProxy` | [x] |
| 12 | `c2Len` | `T` | `c2Len` | [x] |
| 13 | `c2Det2` | `T` | `c2Det2` | [x] |
| 14 | `c2GJKSimplexMetric` | `T` | `c2GJKSimplexMetric` | [x] |
| 15 | `c2Mulrv` | `T` | `c2Mulrv` | [x] |
| 16 | `c2Add` | `T` | `c2Add` | [x] |
| 17 | `c2Mulxv` | `T` | `c2Mulxv` | [x] |
| 18 | `c22` | `T` | `c22` | [x] |
| 19 | `c23` | `T` | `c23` | [x] |
| 20 | `c2Neg` | `T` | `c2Neg` | [x] |
| 21 | `c2Skew` | `T` | `c2Skew` | [x] |
| 22 | `c2CCW90` | `T` | `c2CCW90` | [x] |
| 23 | `c2D` | `T` | `c2D` | [x] |
| 24 | `c2Support` | `T` | `c2Support` | [x] |
| 25 | `c2Witness` | `T` | `c2Witness` | [x] |
| 26 | `c2Div` | `T` | `c2Div` | [x] |
| 27 | `c2Norm` | `T` | `c2Norm` | [x] |
| 28 | `c2L` | `T` | `c2L` | [x] |
| 29 | `c2MulrvT` | `T` | `c2MulrvT` | [x] |
| 30 | `c2GJK` | `T` | `c2GJK` | [x] |
| 31 | `gjk_cache` | `T` | `gjk_cache` | [x] |

Missing C symbols in Rust: **0**.

