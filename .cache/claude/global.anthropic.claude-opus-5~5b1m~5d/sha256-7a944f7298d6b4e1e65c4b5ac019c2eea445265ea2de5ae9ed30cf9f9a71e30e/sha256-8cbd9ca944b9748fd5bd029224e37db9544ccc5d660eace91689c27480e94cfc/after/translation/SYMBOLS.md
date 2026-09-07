# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C shared object
`c_src/build/libharvest-work-UoQHxV.so`, compared against the Rust cdylib
`translation/target/release/libaabb_lib.so`.

Reproduce with:

```sh
nm -D --defined-only c_src/build/libharvest-work-UoQHxV.so | awk '$2=="T"{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libaabb_lib.so | awk '$2=="T"{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # missing in Rust  -> MUST be empty
comm -13 /tmp/c.syms /tmp/r.syms   # extra in Rust    -> is empty
```

## Result

* C exports (`T`):    **38**
* Rust exports (`T`): **38**
* Missing in Rust:    **0**
* Extra in Rust:      **0**

## Table

| # | symbol | C signature (from `c_src/src/lib.c`) | exported by C `.so` | exported by Rust `.so` |
|---|--------|--------------------------------------|---------------------|------------------------|
| 1 | `c2V` | `c2v c2V(float x, float y)` | yes | yes |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v a, float b)` | yes | yes |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v a, c2v b)` | yes | yes |
| 4 | `c2Minv` | `c2v c2Minv(c2v a, c2v b)` | yes | yes |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v a, c2v lo, c2v hi)` | yes | yes |
| 6 | `c2Sub` | `c2v c2Sub(c2v a, c2v b)` | yes | yes |
| 7 | `c2Dot` | `float c2Dot(c2v a, c2v b)` | yes | yes |
| 8 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | yes | yes |
| 9 | `c2xIdentity` | `c2x c2xIdentity(void)` | yes | yes |
| 10 | `c2BBVerts` | `void c2BBVerts(c2v *out, c2AABB *bb)` | yes | yes |
| 11 | `c2MakeProxy` | `void c2MakeProxy(const void *shape, C2_TYPE type, c2Proxy *p)` | yes | yes |
| 12 | `c2Len` | `float c2Len(c2v a)` | yes | yes |
| 13 | `c2Det2` | `float c2Det2(c2v a, c2v b)` | yes | yes |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex *s)` | yes | yes |
| 15 | `c2Mulrv` | `c2v c2Mulrv(c2r a, c2v b)` | yes | yes |
| 16 | `c2Add` | `c2v c2Add(c2v a, c2v b)` | yes | yes |
| 17 | `c2Mulxv` | `c2v c2Mulxv(c2x a, c2v b)` | yes | yes |
| 18 | `c22` | `void c22(c2Simplex *s)` | yes | yes |
| 19 | `c23` | `void c23(c2Simplex *s)` | yes | yes |
| 20 | `c2Neg` | `c2v c2Neg(c2v a)` | yes | yes |
| 21 | `c2Skew` | `c2v c2Skew(c2v a)` | yes | yes |
| 22 | `c2CCW90` | `c2v c2CCW90(c2v a)` | yes | yes |
| 23 | `c2D` | `c2v c2D(c2Simplex *s)` | yes | yes |
| 24 | `c2Support` | `int c2Support(const c2v *verts, int count, c2v d)` | yes | yes |
| 25 | `c2Witness` | `void c2Witness(c2Simplex *s, c2v *a, c2v *b)` | yes | yes |
| 26 | `c2Div` | `c2v c2Div(c2v a, float b)` | yes | yes |
| 27 | `c2Norm` | `c2v c2Norm(c2v a)` | yes | yes |
| 28 | `c2L` | `c2v c2L(c2Simplex *s)` | yes | yes |
| 29 | `c2MulrvT` | `c2v c2MulrvT(c2r a, c2v b)` | yes | yes |
| 30 | `c2GJK` | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` | yes | yes |
| 31 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB A, c2AABB B)` | yes | yes |
| 32 | `c2AABBtoCapsule` | `int c2AABBtoCapsule(c2AABB A, c2Capsule B)` | yes | yes |
| 33 | `c2CapsuletoCapsule` | `int c2CapsuletoCapsule(c2Capsule A, c2Capsule B)` | yes | yes |
| 34 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle A, c2Circle B)` | yes | yes |
| 35 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle A, c2AABB B)` | yes | yes |
| 36 | `c2CircletoCapsule` | `int c2CircletoCapsule(c2Circle A, c2Capsule B)` | yes | yes |
| 37 | `c2Collided` | `int c2Collided(const void *A, C2_TYPE typeA, const void *B, C2_TYPE typeB)` | yes | yes |
| 38 | `aabb` | `int aabb(float min_x, float min_y, float max_x, float max_y)` (the only symbol in `include/lib.h`) | yes | yes |

## Undefined (imported) symbols

The C `.so` imports only `sqrtf@GLIBC` (plus weak CRT hooks). The Rust `.so`
imports `sqrtf` inlined as the `sqrtss` instruction and additionally pulls in
the Rust standard-library runtime (`malloc`, `memcpy`, `_Unwind_*`, `abort`,
`dl_iterate_phdr`, ...). **All Rust undefined symbols are libc / libgcc
runtime symbols; there are 0 undefined non-libc symbols.**

## Structs / types (not symbols, but part of the ABI surface)

Layout is statically asserted in `src/lib.rs` (`const _: () = { ... }`) and
verified at runtime against the C build in `tests/layout.rs`:

| type | size | align | notes |
|------|------|-------|-------|
| `c2v` | 8 | 4 | |
| `c2r` | 8 | 4 | |
| `c2x` | 16 | 4 | |
| `c2Circle` | 12 | 4 | |
| `c2AABB` | 16 | 4 | |
| `c2Capsule` | 20 | 4 | |
| `c2GJKCache` | 36 | 4 | `iA[3]` @8, `iB[3]` @20, `div` @32 |
| `c2Proxy` | 72 | 4 | internal, `verts[8]` @8 |
| `c2sv` | 36 | 4 | internal |
| `c2Simplex` | 152 | 4 | internal; Rust models C's `a,b,c,d` as `[c2sv;4]` because the C code itself aliases them as an array (`c2sv *verts = &s.a;`) |
