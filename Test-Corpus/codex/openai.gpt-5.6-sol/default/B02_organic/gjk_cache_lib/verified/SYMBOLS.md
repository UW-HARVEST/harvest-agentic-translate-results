# Exported Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-rzwpTA.so
```

Rust comparison command:

```text
nm -D --defined-only target/release/libgjk_cache_lib.so
```

| # | C symbol | C type | Rust export |
|---|----------|--------|-------------|
| 1 | `c22` | `T` | present |
| 2 | `c23` | `T` | present |
| 3 | `c2Add` | `T` | present |
| 4 | `c2BBVerts` | `T` | present |
| 5 | `c2CCW90` | `T` | present |
| 6 | `c2Clampv` | `T` | present |
| 7 | `c2D` | `T` | present |
| 8 | `c2Det2` | `T` | present |
| 9 | `c2Div` | `T` | present |
| 10 | `c2Dot` | `T` | present |
| 11 | `c2GJK` | `T` | present |
| 12 | `c2GJKSimplexMetric` | `T` | present |
| 13 | `c2L` | `T` | present |
| 14 | `c2Len` | `T` | present |
| 15 | `c2MakeProxy` | `T` | present |
| 16 | `c2Maxv` | `T` | present |
| 17 | `c2Minv` | `T` | present |
| 18 | `c2Mulrv` | `T` | present |
| 19 | `c2MulrvT` | `T` | present |
| 20 | `c2Mulvs` | `T` | present |
| 21 | `c2Mulxv` | `T` | present |
| 22 | `c2Neg` | `T` | present |
| 23 | `c2Norm` | `T` | present |
| 24 | `c2RotIdentity` | `T` | present |
| 25 | `c2Skew` | `T` | present |
| 26 | `c2Sub` | `T` | present |
| 27 | `c2Support` | `T` | present |
| 28 | `c2V` | `T` | present |
| 29 | `c2Witness` | `T` | present |
| 30 | `c2xIdentity` | `T` | present |
| 31 | `gjk_cache` | `T` | present |

Missing C symbols in Rust: **0**.

