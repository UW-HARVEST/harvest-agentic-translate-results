# Dynamic symbol surface

Generated from:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-GriyDD.so |
  awk '$2 ~ /^[TDBR]$/ {print $3}' | sort -u
```

C library: `../c_src/build/libharvest-work-GriyDD.so`

Rust library: `target/release/libgjk_cache_lib.so`

| # | C symbol | Rust export |
|---:|---|:---:|
| 1 | `c22` | [x] |
| 2 | `c23` | [x] |
| 3 | `c2Add` | [x] |
| 4 | `c2BBVerts` | [x] |
| 5 | `c2CCW90` | [x] |
| 6 | `c2Clampv` | [x] |
| 7 | `c2D` | [x] |
| 8 | `c2Det2` | [x] |
| 9 | `c2Div` | [x] |
| 10 | `c2Dot` | [x] |
| 11 | `c2GJK` | [x] |
| 12 | `c2GJKSimplexMetric` | [x] |
| 13 | `c2L` | [x] |
| 14 | `c2Len` | [x] |
| 15 | `c2MakeProxy` | [x] |
| 16 | `c2Maxv` | [x] |
| 17 | `c2Minv` | [x] |
| 18 | `c2Mulrv` | [x] |
| 19 | `c2MulrvT` | [x] |
| 20 | `c2Mulvs` | [x] |
| 21 | `c2Mulxv` | [x] |
| 22 | `c2Neg` | [x] |
| 23 | `c2Norm` | [x] |
| 24 | `c2RotIdentity` | [x] |
| 25 | `c2Skew` | [x] |
| 26 | `c2Sub` | [x] |
| 27 | `c2Support` | [x] |
| 28 | `c2V` | [x] |
| 29 | `c2Witness` | [x] |
| 30 | `c2xIdentity` | [x] |
| 31 | `gjk_cache` | [x] |

Missing C symbols in Rust: **0**

Undefined non-system symbols in Rust: **0**
