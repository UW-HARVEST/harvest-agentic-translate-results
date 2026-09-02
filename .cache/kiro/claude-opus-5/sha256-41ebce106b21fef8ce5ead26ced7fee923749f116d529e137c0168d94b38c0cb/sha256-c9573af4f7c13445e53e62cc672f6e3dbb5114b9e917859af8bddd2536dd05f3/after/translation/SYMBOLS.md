# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-f13xZN.so | awk '$2=="T"||$2=="W"{print $3}' | sort
nm -D --defined-only translation/target/release/libpoly_ray_lib.so | awk '$2=="T"||$2=="W"{print $3}' | sort
```

The C `.so` exports **28** dynamic text symbols. All 28 are exported by the
Rust `cdylib` with identical names. `comm -23 c_syms rust_syms` is **empty**.

| # | symbol | C signature (from `src/lib.c` / `include/lib.h`) | in C `.so` | in Rust `.so` |
|---|--------|--------------------------------------------------|------------|---------------|
| 1 | `c2V` | `c2v c2V(float,float)` | yes | yes |
| 2 | `c2Dot` | `float c2Dot(c2v,c2v)` | yes | yes |
| 3 | `c2Len` | `float c2Len(c2v)` | yes | yes |
| 4 | `c2Add` | `c2v c2Add(c2v,c2v)` | yes | yes |
| 5 | `c2Sub` | `c2v c2Sub(c2v,c2v)` | yes | yes |
| 6 | `c2Mulvs` | `c2v c2Mulvs(c2v,float)` | yes | yes |
| 7 | `c2Div` | `c2v c2Div(c2v,float)` | yes | yes |
| 8 | `c2Norm` | `c2v c2Norm(c2v)` | yes | yes |
| 9 | `c2Minv` | `c2v c2Minv(c2v,c2v)` | yes | yes |
| 10 | `c2Maxv` | `c2v c2Maxv(c2v,c2v)` | yes | yes |
| 11 | `c2Skew` | `c2v c2Skew(c2v)` | yes | yes |
| 12 | `c2Absv` | `c2v c2Absv(c2v)` | yes | yes |
| 13 | `c2CCW90` | `c2v c2CCW90(c2v)` | yes | yes |
| 14 | `c2MulmvT` | `c2v c2MulmvT(c2m,c2v)` | yes | yes |
| 15 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | yes | yes |
| 16 | `c2xIdentity` | `c2x c2xIdentity(void)` | yes | yes |
| 17 | `c2Mulrv` | `c2v c2Mulrv(c2r,c2v)` | yes | yes |
| 18 | `c2MulrvT` | `c2v c2MulrvT(c2r,c2v)` | yes | yes |
| 19 | `c2MulxvT` | `c2v c2MulxvT(c2x,c2v)` | yes | yes |
| 20 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB,c2AABB)` | yes | yes |
| 21 | `c2AABBtoPoint` | `int c2AABBtoPoint(c2AABB,c2v)` | yes | yes |
| 22 | `c2CircleToPoint` | `int c2CircleToPoint(c2Circle,c2v)` | yes | yes |
| 23 | `c2RaytoCircle` | `int c2RaytoCircle(c2Ray,c2Circle,c2Raycast*)` | yes | yes |
| 24 | `c2RaytoAABB` | `int c2RaytoAABB(c2Ray,c2AABB,c2Raycast*)` | yes | yes |
| 25 | `c2RaytoCapsule` | `int c2RaytoCapsule(c2Ray,c2Capsule,c2Raycast*)` | yes | yes |
| 26 | `c2RaytoPoly` | `int c2RaytoPoly(c2Ray,const c2Poly*,const c2x*,c2Raycast*)` | yes | yes |
| 27 | `c2CastRay` | `int c2CastRay(c2Ray,const void*,const c2x*,C2_TYPE,c2Raycast*)` | yes | yes |
| 28 | `poly_ray` | `int poly_ray(c2Raycast*,c2Raycast*)` | yes | yes |

## Deliberately NOT exported (matches C)

These are `static inline` in `src/lib.c`, so the C `.so` does not export them
either. The Rust translation keeps them private (`#[inline(always)] fn`):

- `c2SignedDistPointToPlane_OneDimensional`
- `c2RayToPlane_OneDimensional`

## Undefined-symbol check

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports (`malloc`, `memcpy`, `_Unwind_*`, `__cxa_finalize`, …). **0 missing or
undefined non-libc symbols.** The C `.so`'s only non-libc-runtime import is
`sqrtf`, which Rust satisfies with the intrinsic `f32::sqrt` (same `sqrtss`
instruction), so no import is required.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
feature combination that exists is the default (empty) one. Verified with:

```
grep -n '\[features\]' translation/Cargo.toml   # no match
```

`cargo test --no-default-features` is therefore identical to `cargo test`, and
both are run in the harness (`run_all.sh`).
