# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  : `c_src/build/libharvest-work-mnIJO3.so` (gcc 11.5, default CMake build type ⇒ `-O0`)
* Rust: `translation/target/release/libomni_manifold_lib.so` (`cdylib`, `opt-level = 2`)

Regenerate / re-verify with:

```sh
nm -D --defined-only c_src/build/lib*.so                        | awk '{print $3}' | sort > /tmp/c.txt
nm -D --defined-only translation/target/release/*.so            | awk '{print $3}' | sort > /tmp/r.txt
comm -23 /tmp/c.txt /tmp/r.txt      # must be EMPTY  (C symbols missing in Rust)
```

## Status

| metric | value |
|--------|-------|
| symbols exported by C `.so`      | 46 |
| symbols exported by Rust `.so`   | 46 (same set) |
| **C symbols missing from Rust**  | **0** |
| Rust undefined non-libc symbols  | 0 (only glibc + `_Unwind_*`/`__cxa_*` runtime) |

`c_src` has exactly one translation unit (`src/lib.c`) and it is fully translated;
no module was skipped, so no additional C had to be translated in this phase.

## Table

`OK` = present in the Rust `.so` under the exact same name.

| # | symbol | kind | C signature | Rust |
|---|--------|------|-------------|------|
| 1 | `c2V` | vec ctor | `c2v c2V(float,float)` | OK |
| 2 | `c2Mulvs` | vec | `c2v c2Mulvs(c2v,float)` | OK |
| 3 | `c2Maxv` | vec | `c2v c2Maxv(c2v,c2v)` | OK |
| 4 | `c2Minv` | vec | `c2v c2Minv(c2v,c2v)` | OK |
| 5 | `c2Clampv` | vec | `c2v c2Clampv(c2v,c2v,c2v)` | OK |
| 6 | `c2Sub` | vec | `c2v c2Sub(c2v,c2v)` | OK |
| 7 | `c2Dot` | vec | `float c2Dot(c2v,c2v)` | OK |
| 8 | `c2Dist` | halfspace | `float c2Dist(c2h,c2v)` | OK |
| 9 | `c2PlaneAt` | poly | `c2h c2PlaneAt(const c2Poly*,int)` | OK |
| 10 | `c2RotIdentity` | xform | `c2r c2RotIdentity(void)` | OK |
| 11 | `c2xIdentity` | xform | `c2x c2xIdentity(void)` | OK |
| 12 | `c2BBVerts` | aabb | `void c2BBVerts(c2v*,c2AABB*)` | OK |
| 13 | `c2MakeProxy` | gjk | `void c2MakeProxy(const void*,C2_TYPE,c2Proxy*)` | OK |
| 14 | `c2Len` | vec | `float c2Len(c2v)` | OK |
| 15 | `c2Det2` | vec | `float c2Det2(c2v,c2v)` | OK |
| 16 | `c2GJKSimplexMetric` | gjk | `float c2GJKSimplexMetric(c2Simplex*)` | OK |
| 17 | `c2Mulrv` | xform | `c2v c2Mulrv(c2r,c2v)` | OK |
| 18 | `c2MulrvT` | xform | `c2v c2MulrvT(c2r,c2v)` | OK |
| 19 | `c2Add` | vec | `c2v c2Add(c2v,c2v)` | OK |
| 20 | `c2Mulxv` | xform | `c2v c2Mulxv(c2x,c2v)` | OK |
| 21 | `c2MulxvT` | xform | `c2v c2MulxvT(c2x,c2v)` | OK |
| 22 | `c2Intersect` | clip | `c2v c2Intersect(c2v,c2v,float,float)` | OK |
| 23 | `c2Div` | vec | `c2v c2Div(c2v,float)` | OK |
| 24 | `c2Norm` | vec | `c2v c2Norm(c2v)` | OK |
| 25 | `c2Neg` | vec | `c2v c2Neg(c2v)` | OK |
| 26 | `c2CCW90` | vec | `c2v c2CCW90(c2v)` | OK |
| 27 | `c22` | gjk | `void c22(c2Simplex*)` | OK |
| 28 | `c23` | gjk | `void c23(c2Simplex*)` | OK |
| 29 | `c2Skew` | vec | `c2v c2Skew(c2v)` | OK |
| 30 | `c2D` | gjk | `c2v c2D(c2Simplex*)` | OK |
| 31 | `c2Support` | gjk | `int c2Support(const c2v*,int,c2v)` | OK |
| 32 | `c2Witness` | gjk | `void c2Witness(c2Simplex*,c2v*,c2v*)` | OK |
| 33 | `c2L` | gjk | `c2v c2L(c2Simplex*)` | OK |
| 34 | `c2GJK` | gjk | `float c2GJK(const void*,C2_TYPE,const c2x*,const void*,C2_TYPE,const c2x*,c2v*,c2v*,int,int*,c2GJKCache*)` | OK |
| 35 | `c2Absv` | vec | `c2v c2Absv(c2v)` | OK |
| 36 | `c2CircletoCircleManifold` | manifold | `void (c2Circle,c2Circle,c2Manifold*)` | OK |
| 37 | `c2CircletoAABBManifold` | manifold | `void (c2Circle,c2AABB,c2Manifold*)` | OK |
| 38 | `c2CircletoCapsuleManifold` | manifold | `void (c2Circle,c2Capsule,c2Manifold*)` | OK |
| 39 | `c2AABBtoAABBManifold` | manifold | `void (c2AABB,c2AABB,c2Manifold*)` | OK |
| 40 | `c2CapsuletoPolyManifold` | manifold | `void (c2Capsule,const c2Poly*,const c2x*,c2Manifold*)` | OK |
| 41 | `c2Norms` | poly | `void c2Norms(c2v*,c2v*,int)` | OK |
| 42 | `c2AABBtoCapsuleManifold` | manifold | `void (c2AABB,c2Capsule,c2Manifold*)` | OK |
| 43 | `c2CapsuletoCapsuleManifold` | manifold | `void (c2Capsule,c2Capsule,c2Manifold*)` | OK |
| 44 | `c2Collide` | dispatch | `void (const void*,C2_TYPE,const void*,C2_TYPE,c2Manifold*)` | OK |
| 45 | `ptr_from_parts` | helper | `void* ptr_from_parts(C2_TYPE,float,float,float,float,float)` | OK |
| 46 | `omni_manifold` | public API | `void omni_manifold(c2Manifold*,C2_TYPE,f,f,f,f,f,C2_TYPE,f,f,f,f,f)` | OK |

## `static` (deliberately NOT exported by either side)

| C symbol | note |
|----------|------|
| `c2Clip` | `static int c2Clip(c2v*,c2h)` — private, exercised via `c2CapsuletoPolyManifold` |
| `c2SidePlanes` | `static int` — private |
| `c2SidePlanesFromPoly` | `static int` — private |
| `c2KeepDeep` | `static void` — private |
| `c2Incident` | `static void` — private |

Both `.so`s export none of these — verified: `comm -13 c.txt r.txt` is also empty,
i.e. the Rust `.so` exports no *extra* `c2*` symbols either.
