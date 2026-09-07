# Dynamic symbol surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-pZoI7S.so
nm -D --defined-only target/release/libgjk_lib.so
```

| C symbol | Rust export |
|----------|-------------|
| `c22` | present |
| `c23` | present |
| `c2Add` | present |
| `c2BBVerts` | present |
| `c2CCW90` | present |
| `c2Clampv` | present |
| `c2D` | present |
| `c2Det2` | present |
| `c2Div` | present |
| `c2Dot` | present |
| `c2GJK` | present |
| `c2GJKSimplexMetric` | present |
| `c2L` | present |
| `c2Len` | present |
| `c2MakeProxy` | present |
| `c2Maxv` | present |
| `c2Minv` | present |
| `c2Mulrv` | present |
| `c2MulrvT` | present |
| `c2Mulvs` | present |
| `c2Mulxv` | present |
| `c2Neg` | present |
| `c2Norm` | present |
| `c2RotIdentity` | present |
| `c2Skew` | present |
| `c2Sub` | present |
| `c2Support` | present |
| `c2V` | present |
| `c2Witness` | present |
| `c2xIdentity` | present |
| `gjk` | present |

Missing C symbols in Rust: **0**.

