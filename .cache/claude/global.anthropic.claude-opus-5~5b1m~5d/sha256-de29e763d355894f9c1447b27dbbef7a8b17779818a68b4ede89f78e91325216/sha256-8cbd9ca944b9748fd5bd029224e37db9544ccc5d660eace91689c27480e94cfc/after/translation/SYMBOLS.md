# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-sQlHPw.so   | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libcapsule_lib.so | awk '{print $3}' | sort
```

C `.so` exports **38** symbols. Rust `.so` exports **38** symbols.
`comm -23` (missing in Rust) → **empty**. `comm -13` (extra in Rust) → **empty**.

| # | symbol | C signature (from `c_src/src/lib.c`) | in C .so | in Rust .so |
|---|--------|--------------------------------------|----------|-------------|
| 1 | `c2V` | `c2v c2V(float, float)` | T | T |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v, float)` | T | T |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | T | T |
| 4 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | T | T |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v, c2v, c2v)` | T | T |
| 6 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | T | T |
| 7 | `c2Dot` | `float c2Dot(c2v, c2v)` | T | T |
| 8 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | T | T |
| 9 | `c2xIdentity` | `c2x c2xIdentity(void)` | T | T |
| 10 | `c2BBVerts` | `void c2BBVerts(c2v*, c2AABB*)` | T | T |
| 11 | `c2MakeProxy` | `void c2MakeProxy(const void*, C2_TYPE, c2Proxy*)` | T | T |
| 12 | `c2Len` | `float c2Len(c2v)` | T | T |
| 13 | `c2Det2` | `float c2Det2(c2v, c2v)` | T | T |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex*)` | T | T |
| 15 | `c2Mulrv` | `c2v c2Mulrv(c2r, c2v)` | T | T |
| 16 | `c2Add` | `c2v c2Add(c2v, c2v)` | T | T |
| 17 | `c2Mulxv` | `c2v c2Mulxv(c2x, c2v)` | T | T |
| 18 | `c22` | `void c22(c2Simplex*)` | T | T |
| 19 | `c23` | `void c23(c2Simplex*)` | T | T |
| 20 | `c2Neg` | `c2v c2Neg(c2v)` | T | T |
| 21 | `c2Skew` | `c2v c2Skew(c2v)` | T | T |
| 22 | `c2CCW90` | `c2v c2CCW90(c2v)` | T | T |
| 23 | `c2D` | `c2v c2D(c2Simplex*)` | T | T |
| 24 | `c2Support` | `int c2Support(const c2v*, int, c2v)` | T | T |
| 25 | `c2Witness` | `void c2Witness(c2Simplex*, c2v*, c2v*)` | T | T |
| 26 | `c2Div` | `c2v c2Div(c2v, float)` | T | T |
| 27 | `c2Norm` | `c2v c2Norm(c2v)` | T | T |
| 28 | `c2L` | `c2v c2L(c2Simplex*)` | T | T |
| 29 | `c2MulrvT` | `c2v c2MulrvT(c2r, c2v)` | T | T |
| 30 | `c2GJK` | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` | T | T |
| 31 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB, c2AABB)` | T | T |
| 32 | `c2AABBtoCapsule` | `int c2AABBtoCapsule(c2AABB, c2Capsule)` | T | T |
| 33 | `c2CapsuletoCapsule` | `int c2CapsuletoCapsule(c2Capsule, c2Capsule)` | T | T |
| 34 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle, c2Circle)` | T | T |
| 35 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle, c2AABB)` | T | T |
| 36 | `c2CircletoCapsule` | `int c2CircletoCapsule(c2Circle, c2Capsule)` | T | T |
| 37 | `c2Collided` | `int c2Collided(const void*, C2_TYPE, const void*, C2_TYPE)` | T | T |
| 38 | `capsule` | `int capsule(float, float, float, float, float)` — the only symbol declared in `include/lib.h` | T | T |

## Undefined (imported) symbols

C `.so` undefined non-libc symbols: none. C imports only `sqrtf` (libm) plus the
usual `_ITM_*` / `__gmon_start__` / `__cxa_finalize` glibc weak stubs.
Rust `.so` imports only libc/libgcc runtime symbols (`memcpy`, `sqrtf` is
inlined to `sqrtss`), plus the same glibc weak stubs. **0 missing/undefined
non-libc symbols.**

## Notes

* No module of the C source is untranslated: `c_src` consists of exactly one
  translation unit (`src/lib.c`, 647 lines) and one header (`include/lib.h`,
  1 line). Every non-`static` C function above has a real Rust implementation —
  there are no stubs and no `unimplemented!()`.
* The C library exports its "internal" helpers too (nothing is `static`), so the
  Rust crate must export the full low-level surface, not just `capsule`. It does.
* The Rust struct layouts are `#[repr(C)]` and byte-identical:
  `c2v`=8, `c2r`=8, `c2x`=16, `c2Circle`=12, `c2AABB`=16, `c2Capsule`=20,
  `c2GJKCache`=36, `c2Proxy`=72, `c2sv`=36, `c2Simplex`=152.
  `c2Simplex`'s four `c2sv a,b,c,d` members are modelled as `verts: [c2sv; 4]`
  so the C idiom `c2sv *verts = &s.a; verts[i]` is reproducible verbatim.
* `c2GJK`'s `c2Proxy pA; c2Proxy pB;` are *uninitialised* C locals, so they are
  modelled as thread-local persistent scratch rather than fresh zeroed values —
  see `ERRORS.md` rows 18/19 and the "Scope" section there. This adds two
  private helpers (`c2GJK_impl`, `c2GJK_nested`) which are deliberately NOT
  exported, keeping the exported symbol set at exactly the C's 38.
