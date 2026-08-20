# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically:

```sh
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translated_rust && cargo build --no-default-features

nm -D --defined-only c_src/build/libtranslated_rust.so  | awk '$2~/^[TWDB]$/{print $3}' | sort > c_syms.txt
nm -D --defined-only target/debug/libpoly_ray_lib.so    | awk '$2~/^[TWDB]$/{print $3}' | sort > rust_syms.txt
comm -23 c_syms.txt rust_syms.txt      # must be EMPTY
```

* C `.so`:    `c_src/build/libtranslated_rust.so`
* Rust `.so`: `target/debug/libpoly_ray_lib.so`  (`[lib] name = "poly_ray_lib"`, `crate-type = ["cdylib"]`)

The C library is a single translation unit (`c_src/src/lib.c`) plus one public
header (`c_src/include/lib.h`).  There are **no untranslated C source files** —
`src/lib.rs` covers 100% of `lib.c`.

## Symbol table

28 dynamic symbols are defined by the C `.so`.  All 28 are defined by the Rust
`.so` under the identical name.

| # | C symbol | C signature | exported by Rust `.so` | Rust item |
|---|----------|-------------|------------------------|-----------|
| 1  | `c2V`             | `c2v c2V(float,float)`                                  | yes | `c2V` |
| 2  | `c2Dot`           | `float c2Dot(c2v,c2v)`                                  | yes | `c2Dot` |
| 3  | `c2Len`           | `float c2Len(c2v)`                                      | yes | `c2Len` |
| 4  | `c2Add`           | `c2v c2Add(c2v,c2v)`                                    | yes | `c2Add` |
| 5  | `c2Sub`           | `c2v c2Sub(c2v,c2v)`                                    | yes | `c2Sub` |
| 6  | `c2Mulvs`         | `c2v c2Mulvs(c2v,float)`                                | yes | `c2Mulvs` |
| 7  | `c2Div`           | `c2v c2Div(c2v,float)`                                  | yes | `c2Div` |
| 8  | `c2Norm`          | `c2v c2Norm(c2v)`                                       | yes | `c2Norm` |
| 9  | `c2Minv`          | `c2v c2Minv(c2v,c2v)`                                   | yes | `c2Minv` |
| 10 | `c2Maxv`          | `c2v c2Maxv(c2v,c2v)`                                   | yes | `c2Maxv` |
| 11 | `c2Skew`          | `c2v c2Skew(c2v)`                                       | yes | `c2Skew` |
| 12 | `c2Absv`          | `c2v c2Absv(c2v)`                                       | yes | `c2Absv` |
| 13 | `c2CCW90`         | `c2v c2CCW90(c2v)`                                      | yes | `c2CCW90` |
| 14 | `c2MulmvT`        | `c2v c2MulmvT(c2m,c2v)`                                 | yes | `c2MulmvT` |
| 15 | `c2RotIdentity`   | `c2r c2RotIdentity(void)`                               | yes | `c2RotIdentity` |
| 16 | `c2xIdentity`     | `c2x c2xIdentity(void)`                                 | yes | `c2xIdentity` |
| 17 | `c2Mulrv`         | `c2v c2Mulrv(c2r,c2v)`                                  | yes | `c2Mulrv` |
| 18 | `c2MulrvT`        | `c2v c2MulrvT(c2r,c2v)`                                 | yes | `c2MulrvT` |
| 19 | `c2MulxvT`        | `c2v c2MulxvT(c2x,c2v)`                                 | yes | `c2MulxvT` |
| 20 | `c2AABBtoAABB`    | `int c2AABBtoAABB(c2AABB,c2AABB)`                       | yes | `c2AABBtoAABB` |
| 21 | `c2AABBtoPoint`   | `int c2AABBtoPoint(c2AABB,c2v)`                         | yes | `c2AABBtoPoint` |
| 22 | `c2CircleToPoint` | `int c2CircleToPoint(c2Circle,c2v)`                     | yes | `c2CircleToPoint` |
| 23 | `c2RaytoCircle`   | `int c2RaytoCircle(c2Ray,c2Circle,c2Raycast*)`          | yes | `c2RaytoCircle` |
| 24 | `c2RaytoAABB`     | `int c2RaytoAABB(c2Ray,c2AABB,c2Raycast*)`              | yes | `c2RaytoAABB` |
| 25 | `c2RaytoCapsule`  | `int c2RaytoCapsule(c2Ray,c2Capsule,c2Raycast*)`        | yes | `c2RaytoCapsule` |
| 26 | `c2RaytoPoly`     | `int c2RaytoPoly(c2Ray,const c2Poly*,const c2x*,c2Raycast*)` | yes | `c2RaytoPoly` |
| 27 | `c2CastRay`       | `int c2CastRay(c2Ray,const void*,const c2x*,C2_TYPE,c2Raycast*)` | yes | `c2CastRay` |
| 28 | `poly_ray`        | `int poly_ray(c2Raycast*,c2Raycast*)`                   | yes | `poly_ray` |

## Not exported by either library (`static inline` in C)

These are internal to `lib.c` and correctly have **no** `#[no_mangle]` wrapper
in Rust (they are private `fn`s).  They are covered indirectly through
`c2RaytoAABB`.

| C symbol | reason |
|----------|--------|
| `c2SignedDistPointToPlane_OneDimensional` | `static inline` — not in `nm -D` of the C `.so` |
| `c2RayToPlane_OneDimensional`             | `static inline` — not in `nm -D` of the C `.so` |

## Undefined (imported) symbols

| library | non-libc undefined symbols |
|---------|----------------------------|
| C `.so`    | none (`sqrtf`, `__cxa_finalize`, `__gmon_start__`, `_ITM_*` only) |
| Rust `.so` | none (glibc + `libgcc` `_Unwind_*` only) |

## Result

```
comm -23 c_syms.txt rust_syms.txt   ->  (empty)
```

**0 missing symbols. 0 undefined non-libc symbols.** Phase A / Phase D symbol
parity: **PASS**.
