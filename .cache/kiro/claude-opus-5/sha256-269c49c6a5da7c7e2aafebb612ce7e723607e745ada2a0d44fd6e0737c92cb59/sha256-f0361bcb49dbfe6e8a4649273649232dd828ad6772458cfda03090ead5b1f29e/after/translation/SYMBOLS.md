# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only c_src/build/libharvest-work-Qg9mDQ.so`
Rust side: `nm -D --defined-only translation/target/release/libgen_ray_lib.so`

`c_src` has exactly one translation unit (`src/lib.c`). The two `static inline`
helpers (`c2SignedDistPointToPlane_OneDimensional`,
`c2RayToPlane_OneDimensional`) have internal linkage and are therefore NOT in
the dynamic symbol table; they are private in the Rust crate too. No C source
file was skipped — the whole library is one file and it is fully translated.

## Exported symbol table (22 symbols)

| # | C symbol | C signature (from src/lib.c) | in Rust `.so` |
|---|----------|------------------------------|---------------|
| 1 | `c2V` | `c2v c2V(float, float)` | yes |
| 2 | `c2Dot` | `float c2Dot(c2v, c2v)` | yes |
| 3 | `c2Len` | `float c2Len(c2v)` | yes |
| 4 | `c2Add` | `c2v c2Add(c2v, c2v)` | yes |
| 5 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | yes |
| 6 | `c2Mulvs` | `c2v c2Mulvs(c2v, float)` | yes |
| 7 | `c2Div` | `c2v c2Div(c2v, float)` | yes |
| 8 | `c2Norm` | `c2v c2Norm(c2v)` | yes |
| 9 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | yes |
| 10 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | yes |
| 11 | `c2Skew` | `c2v c2Skew(c2v)` | yes |
| 12 | `c2Absv` | `c2v c2Absv(c2v)` | yes |
| 13 | `c2CCW90` | `c2v c2CCW90(c2v)` | yes |
| 14 | `c2MulmvT` | `c2v c2MulmvT(c2m, c2v)` | yes |
| 15 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB, c2AABB)` | yes |
| 16 | `c2AABBtoPoint` | `int c2AABBtoPoint(c2AABB, c2v)` | yes |
| 17 | `c2CircleToPoint` | `int c2CircleToPoint(c2Circle, c2v)` | yes |
| 18 | `c2RaytoCircle` | `int c2RaytoCircle(c2Ray, c2Circle, c2Raycast*)` | yes |
| 19 | `c2RaytoAABB` | `int c2RaytoAABB(c2Ray, c2AABB, c2Raycast*)` | yes |
| 20 | `c2RaytoCapsule` | `int c2RaytoCapsule(c2Ray, c2Capsule, c2Raycast*)` | yes |
| 21 | `c2CastRay` | `int c2CastRay(c2Ray, const void*, C2_TYPE, c2Raycast*)` | yes |
| 22 | `gen_ray` | `int gen_ray(c2Raycast*, c2Raycast*, c2Raycast*, 16x float)` | yes |

## Not exported (internal linkage in C, private in Rust) — parity by omission

| C symbol | reason |
|----------|--------|
| `c2SignedDistPointToPlane_OneDimensional` | `static inline` |
| `c2RayToPlane_OneDimensional` | `static inline` |

## Diff result

```
comm -23 c_syms.txt rust_syms.txt   ->   (empty)
```

0 symbols missing from the Rust `.so`.
`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports (`memcpy`, `malloc`, `_Unwind_*`, `__cxa_finalize`, …) — 0 undefined
non-libc symbols.

## ABI notes used by the differential tests

SysV AMD64 classification of the by-value struct parameters (identical for
`#[repr(C)]` Rust structs, which is why the test can declare the same
signatures):

| type | size | class |
|------|------|-------|
| `c2v` | 8 | SSE (one xmm half) |
| `c2Raycast` | 12 | SSE, SSE |
| `c2Circle` | 12 | SSE, SSE |
| `c2AABB` | 16 | SSE, SSE |
| `c2m` | 16 | SSE, SSE |
| `c2Capsule` | 20 | MEMORY (stack) |
| `c2Ray` | 20 | MEMORY (stack) |
