# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C shared object
(`c_src/build/libharvest-work-3c68uJ.so`, name derived from the parent
directory by `CMakeLists.txt`) vs. the Rust cdylib
(`translation/target/release/libgen_ray_lib.so`).

Regenerate with:

```sh
nm -D --defined-only c_src/build/*.so                  | awk '{print $3}' | sort > c_syms.txt
nm -D --defined-only translation/target/*/libgen_ray_lib.so | awk '{print $3}' | sort > r_syms.txt
comm -23 c_syms.txt r_syms.txt   # missing from Rust  -> MUST be empty
comm -13 c_syms.txt r_syms.txt   # extra in Rust
```

`c_src/src/lib.c` has exactly two `static inline` helpers
(`c2SignedDistPointToPlane_OneDimensional`, `c2RayToPlane_OneDimensional`);
those have internal linkage in C and therefore correctly have no exported
counterpart in Rust. Every other function in `lib.c` has external linkage and
appears below.

## Symbol table (22 C symbols, 22 Rust symbols, 0 missing, 0 extra)

| # | symbol | C signature | in C `.so` | in Rust `.so` | status |
|---|--------|-------------|-----------|---------------|--------|
| 1 | `c2V` | `c2v c2V(float, float)` | yes | yes | OK |
| 2 | `c2Dot` | `float c2Dot(c2v, c2v)` | yes | yes | OK |
| 3 | `c2Len` | `float c2Len(c2v)` | yes | yes | OK |
| 4 | `c2Add` | `c2v c2Add(c2v, c2v)` | yes | yes | OK |
| 5 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | yes | yes | OK |
| 6 | `c2Mulvs` | `c2v c2Mulvs(c2v, float)` | yes | yes | OK |
| 7 | `c2Div` | `c2v c2Div(c2v, float)` | yes | yes | OK |
| 8 | `c2Norm` | `c2v c2Norm(c2v)` | yes | yes | OK |
| 9 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | yes | yes | OK |
| 10 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | yes | yes | OK |
| 11 | `c2Skew` | `c2v c2Skew(c2v)` | yes | yes | OK |
| 12 | `c2Absv` | `c2v c2Absv(c2v)` | yes | yes | OK |
| 13 | `c2RaytoCircle` | `int c2RaytoCircle(c2Ray, c2Circle, c2Raycast*)` | yes | yes | OK |
| 14 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB, c2AABB)` | yes | yes | OK |
| 15 | `c2RaytoAABB` | `int c2RaytoAABB(c2Ray, c2AABB, c2Raycast*)` | yes | yes | OK |
| 16 | `c2CCW90` | `c2v c2CCW90(c2v)` | yes | yes | OK |
| 17 | `c2MulmvT` | `c2v c2MulmvT(c2m, c2v)` | yes | yes | OK |
| 18 | `c2AABBtoPoint` | `int c2AABBtoPoint(c2AABB, c2v)` | yes | yes | OK |
| 19 | `c2CircleToPoint` | `int c2CircleToPoint(c2Circle, c2v)` | yes | yes | OK |
| 20 | `c2RaytoCapsule` | `int c2RaytoCapsule(c2Ray, c2Capsule, c2Raycast*)` | yes | yes | OK |
| 21 | `c2CastRay` | `int c2CastRay(c2Ray, const void*, C2_TYPE, c2Raycast*)` | yes | yes | OK |
| 22 | `gen_ray` | `int gen_ray(c2Raycast*, c2Raycast*, c2Raycast*, float x18)` | yes | yes | OK |

## Internal-linkage C functions (correctly NOT exported)

| C function | linkage | Rust counterpart |
|---|---|---|
| `c2SignedDistPointToPlane_OneDimensional` | `static inline` | private `signed_dist_point_to_plane_one_dimensional` |
| `c2RayToPlane_OneDimensional` | `static inline` | private `ray_to_plane_one_dimensional` |

## Undefined (imported) symbols

The C `.so` imports `sqrtf` from `libm`. The Rust `.so` implements the same
operation with `f32::sqrt` (a single `sqrtss`, bit-identical to glibc's
`sqrtf`, including the `-NaN` indefinite produced for negative arguments), so
it has no `libm` dependency. No non-libc symbol is undefined in the Rust
`.so`.

## Types crossing the FFI boundary (SysV AMD64 classification)

| type | size | classification | passed in |
|---|---|---|---|
| `c2v` {f32,f32} | 8 | SSE | one xmm (packed) |
| `c2Raycast` {f32,c2v} | 12 | SSE,SSE | xmm0,xmm1 (return: xmm0/xmm1) |
| `c2Circle` {c2v,f32} | 12 | SSE,SSE | xmm0,xmm1 |
| `c2AABB` {c2v,c2v} | 16 | SSE,SSE | xmm0,xmm1 |
| `c2Capsule` {c2v,c2v,f32} | 20 | MEMORY | stack |
| `c2Ray` {c2v,c2v,f32} | 20 | MEMORY | stack |
| `c2m` {c2v,c2v} | 16 | SSE,SSE | xmm0,xmm1 |

All are `#[repr(C)]` in Rust with identical field order, so the Rust
`extern "C"` declarations use the same classification. Verified indirectly by
the differential tests: every struct-by-value argument and return value is
compared bit-for-bit.

## Completion

- [x] `nm -D` diff C -> Rust is **empty** (0 missing symbols).
- [x] `nm -D` diff Rust -> C is **empty** (0 extra symbols).
- [x] No stubs / `unimplemented!()` — every symbol is a real translation.
- [x] 0 undefined non-libc symbols in the Rust `.so`.
