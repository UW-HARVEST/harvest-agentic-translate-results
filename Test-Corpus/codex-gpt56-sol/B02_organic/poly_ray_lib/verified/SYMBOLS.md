# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The C shared object has 28 defined public symbols. The only strong undefined
symbol is the libc/libm dependency `sqrtf`; the remaining undefined entries are
weak ELF runtime hooks. The completion column was checked after Phase D
repeated the comparison following all changes and tests.

| # | C symbol | C type | C source | Rust export present | Phase D |
|---|----------|--------|----------|---------------------|---------|
| 1 | `c2V` | `T` | `c_src/src/lib.c:49` | yes | [x] |
| 2 | `c2Dot` | `T` | `c_src/src/lib.c:56` | yes | [x] |
| 3 | `c2Len` | `T` | `c_src/src/lib.c:60` | yes | [x] |
| 4 | `c2Add` | `T` | `c_src/src/lib.c:64` | yes | [x] |
| 5 | `c2Sub` | `T` | `c_src/src/lib.c:70` | yes | [x] |
| 6 | `c2Mulvs` | `T` | `c_src/src/lib.c:76` | yes | [x] |
| 7 | `c2Div` | `T` | `c_src/src/lib.c:82` | yes | [x] |
| 8 | `c2Norm` | `T` | `c_src/src/lib.c:86` | yes | [x] |
| 9 | `c2Minv` | `T` | `c_src/src/lib.c:90` | yes | [x] |
| 10 | `c2Maxv` | `T` | `c_src/src/lib.c:95` | yes | [x] |
| 11 | `c2Skew` | `T` | `c_src/src/lib.c:100` | yes | [x] |
| 12 | `c2Absv` | `T` | `c_src/src/lib.c:107` | yes | [x] |
| 13 | `c2RaytoCircle` | `T` | `c_src/src/lib.c:111` | yes | [x] |
| 14 | `c2AABBtoAABB` | `T` | `c_src/src/lib.c:129` | yes | [x] |
| 15 | `c2RaytoAABB` | `T` | `c_src/src/lib.c:156` | yes | [x] |
| 16 | `c2CCW90` | `T` | `c_src/src/lib.c:220` | yes | [x] |
| 17 | `c2MulmvT` | `T` | `c_src/src/lib.c:227` | yes | [x] |
| 18 | `c2AABBtoPoint` | `T` | `c_src/src/lib.c:234` | yes | [x] |
| 19 | `c2CircleToPoint` | `T` | `c_src/src/lib.c:242` | yes | [x] |
| 20 | `c2RaytoCapsule` | `T` | `c_src/src/lib.c:248` | yes | [x] |
| 21 | `c2RotIdentity` | `T` | `c_src/src/lib.c:311` | yes | [x] |
| 22 | `c2xIdentity` | `T` | `c_src/src/lib.c:318` | yes | [x] |
| 23 | `c2Mulrv` | `T` | `c_src/src/lib.c:325` | yes | [x] |
| 24 | `c2MulrvT` | `T` | `c_src/src/lib.c:329` | yes | [x] |
| 25 | `c2MulxvT` | `T` | `c_src/src/lib.c:333` | yes | [x] |
| 26 | `c2RaytoPoly` | `T` | `c_src/src/lib.c:337` | yes | [x] |
| 27 | `c2CastRay` | `T` | `c_src/src/lib.c:367` | yes | [x] |
| 28 | `poly_ray` | `T` | `c_src/src/lib.c:381` | yes | [x] |
