# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libgjk_lib.so
```

The C reference exports 31 defined dynamic symbols. The current Rust library
exports the same 31 names.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `c22` | `T` | `c22` | [x] |
| 2 | `c23` | `T` | `c23` | [x] |
| 3 | `c2Add` | `T` | `c2Add` | [x] |
| 4 | `c2BBVerts` | `T` | `c2BBVerts` | [x] |
| 5 | `c2CCW90` | `T` | `c2CCW90` | [x] |
| 6 | `c2Clampv` | `T` | `c2Clampv` | [x] |
| 7 | `c2D` | `T` | `c2D` | [x] |
| 8 | `c2Det2` | `T` | `c2Det2` | [x] |
| 9 | `c2Div` | `T` | `c2Div` | [x] |
| 10 | `c2Dot` | `T` | `c2Dot` | [x] |
| 11 | `c2GJK` | `T` | `c2GJK` | [x] |
| 12 | `c2GJKSimplexMetric` | `T` | `c2GJKSimplexMetric` | [x] |
| 13 | `c2L` | `T` | `c2L` | [x] |
| 14 | `c2Len` | `T` | `c2Len` | [x] |
| 15 | `c2MakeProxy` | `T` | `c2MakeProxy` | [x] |
| 16 | `c2Maxv` | `T` | `c2Maxv` | [x] |
| 17 | `c2Minv` | `T` | `c2Minv` | [x] |
| 18 | `c2Mulrv` | `T` | `c2Mulrv` | [x] |
| 19 | `c2MulrvT` | `T` | `c2MulrvT` | [x] |
| 20 | `c2Mulvs` | `T` | `c2Mulvs` | [x] |
| 21 | `c2Mulxv` | `T` | `c2Mulxv` | [x] |
| 22 | `c2Neg` | `T` | `c2Neg` | [x] |
| 23 | `c2Norm` | `T` | `c2Norm` | [x] |
| 24 | `c2RotIdentity` | `T` | `c2RotIdentity` | [x] |
| 25 | `c2Skew` | `T` | `c2Skew` | [x] |
| 26 | `c2Sub` | `T` | `c2Sub` | [x] |
| 27 | `c2Support` | `T` | `c2Support` | [x] |
| 28 | `c2V` | `T` | `c2V` | [x] |
| 29 | `c2Witness` | `T` | `c2Witness` | [x] |
| 30 | `c2xIdentity` | `T` | `c2xIdentity` | [x] |
| 31 | `gjk` | `T` | `gjk` | [x] |

The only strong undefined symbol in the C library is `sqrtf@GLIBC_2.2.5`,
provided by the explicitly linked system math library. The remaining undefined
entries are weak toolchain/runtime hooks. There are no missing C symbols in the
Rust library.

Completion gate: [x] zero missing C exports.
