# SYMBOLS.md — Phase A symbol surface

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-hil7Mk.so | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libcapsule_lib.so | awk '{print $3}' | sort
```

C `.so`: 38 defined dynamic symbols. Rust `.so`: 38 defined dynamic symbols.
`comm -23` (missing in Rust) = EMPTY. `comm -13` (extra in Rust) = EMPTY.

| # | symbol | C signature (from `c_src/src/lib.c`) | in Rust `.so` |
|---|--------|--------------------------------------|---------------|
| 1 | `c2V` | `c2v c2V(float x, float y)` | yes |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v a, float b)` | yes |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v a, c2v b)` | yes |
| 4 | `c2Minv` | `c2v c2Minv(c2v a, c2v b)` | yes |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v a, c2v lo, c2v hi)` | yes |
| 6 | `c2Sub` | `c2v c2Sub(c2v a, c2v b)` | yes |
| 7 | `c2Dot` | `float c2Dot(c2v a, c2v b)` | yes |
| 8 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | yes |
| 9 | `c2xIdentity` | `c2x c2xIdentity(void)` | yes |
| 10 | `c2BBVerts` | `void c2BBVerts(c2v *out, c2AABB *bb)` | yes |
| 11 | `c2MakeProxy` | `void c2MakeProxy(const void *shape, C2_TYPE type, c2Proxy *p)` | yes |
| 12 | `c2Len` | `float c2Len(c2v a)` | yes |
| 13 | `c2Det2` | `float c2Det2(c2v a, c2v b)` | yes |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex *s)` | yes |
| 15 | `c2Mulrv` | `c2v c2Mulrv(c2r a, c2v b)` | yes |
| 16 | `c2Add` | `c2v c2Add(c2v a, c2v b)` | yes |
| 17 | `c2Mulxv` | `c2v c2Mulxv(c2x a, c2v b)` | yes |
| 18 | `c22` | `void c22(c2Simplex *s)` | yes |
| 19 | `c23` | `void c23(c2Simplex *s)` | yes |
| 20 | `c2Neg` | `c2v c2Neg(c2v a)` | yes |
| 21 | `c2Skew` | `c2v c2Skew(c2v a)` | yes |
| 22 | `c2CCW90` | `c2v c2CCW90(c2v a)` | yes |
| 23 | `c2D` | `c2v c2D(c2Simplex *s)` | yes |
| 24 | `c2Support` | `int c2Support(const c2v *verts, int count, c2v d)` | yes |
| 25 | `c2Witness` | `void c2Witness(c2Simplex *s, c2v *a, c2v *b)` | yes |
| 26 | `c2Div` | `c2v c2Div(c2v a, float b)` | yes |
| 27 | `c2Norm` | `c2v c2Norm(c2v a)` | yes |
| 28 | `c2L` | `c2v c2L(c2Simplex *s)` | yes |
| 29 | `c2MulrvT` | `c2v c2MulrvT(c2r a, c2v b)` | yes |
| 30 | `c2GJK` | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` | yes |
| 31 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB A, c2AABB B)` | yes |
| 32 | `c2AABBtoCapsule` | `int c2AABBtoCapsule(c2AABB A, c2Capsule B)` | yes |
| 33 | `c2CapsuletoCapsule` | `int c2CapsuletoCapsule(c2Capsule A, c2Capsule B)` | yes |
| 34 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle A, c2Circle B)` | yes |
| 35 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle A, c2AABB B)` | yes |
| 36 | `c2CircletoCapsule` | `int c2CircletoCapsule(c2Circle A, c2Capsule B)` | yes |
| 37 | `c2Collided` | `int c2Collided(const void *A, C2_TYPE typeA, const void *B, C2_TYPE typeB)` | yes |
| 38 | `capsule` | `int capsule(float,float,float,float,float)` (the only symbol in `include/lib.h`) | yes |

## Undefined symbols in Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc/glibc
(`memcpy`, `malloc`, `abort`, `__errno_location`, …) and libgcc unwinder
(`_Unwind_*`) imports pulled in by the Rust runtime. **0 missing/undefined
non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section, so the only
configuration is the default (no features). Verified by:

```sh
grep -n '^\[features\]' translation/Cargo.toml   # no match
```

`cargo test --no-default-features` is therefore equivalent to the default and
is still run explicitly as the second (only other) combination.

## Final verification (re-run by `translation/phase_d.sh`)

```
=== build C ===
C  .so: c_src/build/libharvest-work-hil7Mk.so
combos:   <default> <no-default-features>

############ COMBINATION: <default> ############
  symbols: C=38 Rust=38
  missing from Rust: none
  undefined non-libc symbols: none
  test result: ok. 7 passed   (tests/nan_fuzz.rs)
  test result: ok. 58 passed  (tests/phase_b.rs)
  test result: ok. 26 passed  (tests/phase_c.rs)

############ COMBINATION: <no-default-features> ############
  symbols: C=38 Rust=38
  missing from Rust: none
  undefined non-libc symbols: none
  (same 91 tests pass)

PHASE D: ALL COMBINATIONS PASS
```

No symbol needed a new `#[no_mangle]` wrapper and no C module was missing from
the translation — the symbol diff was already empty on the first `nm -D`
comparison, in both directions. The whole library is one translation unit
(`c_src/src/lib.c`, 647 lines) and all 38 of its non-static functions are
present in `translation/src/lib.rs`.
