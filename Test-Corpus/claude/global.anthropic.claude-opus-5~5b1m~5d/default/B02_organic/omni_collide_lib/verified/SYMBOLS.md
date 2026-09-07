# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

- C   `.so`: `c_src/build/libharvest-work-CEfk71.so`  — 39 defined dynamic symbols
- Rust `.so`: `translation/target/release/libomni_collide_lib.so` — 39 defined dynamic symbols

Diff command used:

```sh
nm -D --defined-only <lib> | awk '{print $3}' | sort
comm -23 c_syms.txt r_syms.txt   # missing in Rust -> EMPTY
comm -13 c_syms.txt r_syms.txt   # extra in Rust   -> EMPTY
```

## Result: **0 missing, 0 extra.** Symbol parity is exact.

| # | symbol | C type | in Rust `.so` | C signature (from `c_src/src/lib.c`) |
|---|--------|--------|---------------|--------------------------------------|
| 1 | `c2V` | T | yes | `c2v c2V(float, float)` |
| 2 | `c2Mulvs` | T | yes | `c2v c2Mulvs(c2v, float)` |
| 3 | `c2Maxv` | T | yes | `c2v c2Maxv(c2v, c2v)` |
| 4 | `c2Minv` | T | yes | `c2v c2Minv(c2v, c2v)` |
| 5 | `c2Clampv` | T | yes | `c2v c2Clampv(c2v, c2v, c2v)` |
| 6 | `c2Sub` | T | yes | `c2v c2Sub(c2v, c2v)` |
| 7 | `c2Dot` | T | yes | `float c2Dot(c2v, c2v)` |
| 8 | `c2RotIdentity` | T | yes | `c2r c2RotIdentity(void)` |
| 9 | `c2xIdentity` | T | yes | `c2x c2xIdentity(void)` |
| 10 | `c2BBVerts` | T | yes | `void c2BBVerts(c2v*, c2AABB*)` |
| 11 | `c2MakeProxy` | T | yes | `void c2MakeProxy(const void*, C2_TYPE, c2Proxy*)` |
| 12 | `c2Len` | T | yes | `float c2Len(c2v)` |
| 13 | `c2Det2` | T | yes | `float c2Det2(c2v, c2v)` |
| 14 | `c2GJKSimplexMetric` | T | yes | `float c2GJKSimplexMetric(c2Simplex*)` |
| 15 | `c2Mulrv` | T | yes | `c2v c2Mulrv(c2r, c2v)` |
| 16 | `c2Add` | T | yes | `c2v c2Add(c2v, c2v)` |
| 17 | `c2Mulxv` | T | yes | `c2v c2Mulxv(c2x, c2v)` |
| 18 | `c22` | T | yes | `void c22(c2Simplex*)` |
| 19 | `c23` | T | yes | `void c23(c2Simplex*)` |
| 20 | `c2Neg` | T | yes | `c2v c2Neg(c2v)` |
| 21 | `c2Skew` | T | yes | `c2v c2Skew(c2v)` |
| 22 | `c2CCW90` | T | yes | `c2v c2CCW90(c2v)` |
| 23 | `c2D` | T | yes | `c2v c2D(c2Simplex*)` |
| 24 | `c2Support` | T | yes | `int c2Support(const c2v*, int, c2v)` |
| 25 | `c2Witness` | T | yes | `void c2Witness(c2Simplex*, c2v*, c2v*)` |
| 26 | `c2Div` | T | yes | `c2v c2Div(c2v, float)` |
| 27 | `c2Norm` | T | yes | `c2v c2Norm(c2v)` |
| 28 | `c2L` | T | yes | `c2v c2L(c2Simplex*)` |
| 29 | `c2MulrvT` | T | yes | `c2v c2MulrvT(c2r, c2v)` |
| 30 | `c2GJK` | T | yes | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` |
| 31 | `c2AABBtoAABB` | T | yes | `int c2AABBtoAABB(c2AABB, c2AABB)` |
| 32 | `c2AABBtoCapsule` | T | yes | `int c2AABBtoCapsule(c2AABB, c2Capsule)` |
| 33 | `c2CapsuletoCapsule` | T | yes | `int c2CapsuletoCapsule(c2Capsule, c2Capsule)` |
| 34 | `c2CircletoCircle` | T | yes | `int c2CircletoCircle(c2Circle, c2Circle)` |
| 35 | `c2CircletoAABB` | T | yes | `int c2CircletoAABB(c2Circle, c2AABB)` |
| 36 | `c2CircletoCapsule` | T | yes | `int c2CircletoCapsule(c2Circle, c2Capsule)` |
| 37 | `c2Collided` | T | yes | `int c2Collided(const void*, C2_TYPE, const void*, C2_TYPE)` |
| 38 | `ptr_from_parts` | T | yes | `void* ptr_from_parts(C2_TYPE, float, float, float, float, float)` |
| 39 | `omni_collide` | T | yes | `int omni_collide(C2_TYPE, float x5, C2_TYPE, float x5)` |

## Undefined (imported) symbols

The Rust `.so` imports only libc/libm symbols. Verify with:

```sh
nm -D -u translation/target/release/libomni_collide_lib.so
```

Non-libc undefined symbols: **0**.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
configuration is the default one (`--no-default-features` is equivalent).
Phase D's "every feature combination" therefore collapses to a single combo,
which is verified.
