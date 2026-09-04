# Dynamic symbol surface

Source of truth:

```text
nm -D --defined-only ../c_src/build/libharvest-work-18164a.so
```

Rust comparison:

```text
nm -D --defined-only target/release/libomni_collide_lib.so
```

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
| 38 | `omni_collide` | present |
| 39 | `ptr_from_parts` | present |

Undefined dynamic dependencies in the C library are only `malloc` and `sqrtf`
(plus toolchain weak/runtime symbols); neither is part of the library's public
defined-symbol surface.

Missing Rust symbols: **0**

Final Phase D verification (2026-09-03):

- C defined exports: 39
- Rust defined exports: 39
- Missing exports: 0
- Extra exports: 0
- `ldd -r` reports no unresolved symbols for either shared library.
