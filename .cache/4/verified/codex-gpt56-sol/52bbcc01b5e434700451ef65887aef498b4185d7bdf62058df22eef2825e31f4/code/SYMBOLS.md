# Exported Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Inventory command:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so |
  awk '$2 ~ /^[TW]$/ { print $3 }' | sort -u
```

The C library exports 38 text symbols:

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `c22` | present |
| 2 | `c23` | present |
| 3 | `c2AABBtoAABB` | present |
| 4 | `c2AABBtoCapsule` | present |
| 5 | `c2Add` | present |
| 6 | `c2BBVerts` | present |
| 7 | `c2CCW90` | present |
| 8 | `c2CapsuletoCapsule` | present |
| 9 | `c2CircletoAABB` | present |
| 10 | `c2CircletoCapsule` | present |
| 11 | `c2CircletoCircle` | present |
| 12 | `c2Clampv` | present |
| 13 | `c2Collided` | present |
| 14 | `c2D` | present |
| 15 | `c2Det2` | present |
| 16 | `c2Div` | present |
| 17 | `c2Dot` | present |
| 18 | `c2GJK` | present |
| 19 | `c2GJKSimplexMetric` | present |
| 20 | `c2L` | present |
| 21 | `c2Len` | present |
| 22 | `c2MakeProxy` | present |
| 23 | `c2Maxv` | present |
| 24 | `c2Minv` | present |
| 25 | `c2Mulrv` | present |
| 26 | `c2MulrvT` | present |
| 27 | `c2Mulvs` | present |
| 28 | `c2Mulxv` | present |
| 29 | `c2Neg` | present |
| 30 | `c2Norm` | present |
| 31 | `c2RotIdentity` | present |
| 32 | `c2Skew` | present |
| 33 | `c2Sub` | present |
| 34 | `c2Support` | present |
| 35 | `c2V` | present |
| 36 | `c2Witness` | present |
| 37 | `c2xIdentity` | present |
| 38 | `capsule` | present |

Defined-symbol parity:

```text
C symbols missing from Rust: 0
```

The only strong undefined C symbol is the libc/libm function `sqrtf`. The Rust
library's undefined symbols are platform runtime/libc functions; it has no
undefined project symbol.

## Completion Gate

- [x] All 38 C text exports are present in Rust with exact names.
- [x] The C-to-Rust defined-symbol diff is empty in both directions.
- [x] `ldd -r` reports no unresolved symbol in either shared library.
