# Dynamic Symbol Surface

Reference library:
`c_src/build/libharvest-work-OPYRmD.so`

Command:

```text
nm -D --defined-only libharvest-work-OPYRmD.so
```

The C shared object has 28 defined public symbols. The final Rust release
build exports every one with the exact same spelling.

| # | C symbol | ELF type | Rust export |
|---|----------|----------|-------------|
| 1 | `c2AABBtoAABB` | `T` | [x] |
| 2 | `c2AABBtoPoint` | `T` | [x] |
| 3 | `c2Absv` | `T` | [x] |
| 4 | `c2Add` | `T` | [x] |
| 5 | `c2CCW90` | `T` | [x] |
| 6 | `c2CastRay` | `T` | [x] |
| 7 | `c2CircleToPoint` | `T` | [x] |
| 8 | `c2Div` | `T` | [x] |
| 9 | `c2Dot` | `T` | [x] |
| 10 | `c2Len` | `T` | [x] |
| 11 | `c2Maxv` | `T` | [x] |
| 12 | `c2Minv` | `T` | [x] |
| 13 | `c2MulmvT` | `T` | [x] |
| 14 | `c2Mulrv` | `T` | [x] |
| 15 | `c2MulrvT` | `T` | [x] |
| 16 | `c2Mulvs` | `T` | [x] |
| 17 | `c2MulxvT` | `T` | [x] |
| 18 | `c2Norm` | `T` | [x] |
| 19 | `c2RaytoAABB` | `T` | [x] |
| 20 | `c2RaytoCapsule` | `T` | [x] |
| 21 | `c2RaytoCircle` | `T` | [x] |
| 22 | `c2RaytoPoly` | `T` | [x] |
| 23 | `c2RotIdentity` | `T` | [x] |
| 24 | `c2Skew` | `T` | [x] |
| 25 | `c2Sub` | `T` | [x] |
| 26 | `c2V` | `T` | [x] |
| 27 | `c2xIdentity` | `T` | [x] |
| 28 | `poly_ray` | `T` | [x] |

Undefined C dynamic symbols are runtime/toolchain dependencies, not library
API: `sqrtf`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`.

Final bidirectional `comm` audit: 28 C symbols, 28 Rust symbols, 0 missing,
and 0 extra. `ldd -r` reports 0 unresolved Rust relocations.
