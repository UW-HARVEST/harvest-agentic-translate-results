# SYMBOLS.md — public symbol parity

Source of truth: `nm -D --defined-only` on

* C   : `c_src/build/libharvest-work-p9WqFJ.so`
* Rust: `translation/target/release/libcapsule_lib.so`

Only `src/lib.c` exists in the C tree, so there is exactly one translation unit
to account for; no C module was skipped.

## Every `T` (global text) symbol of the C `.so`

| # | symbol | C signature | exported by Rust `.so` |
|---|--------|-------------|------------------------|
| 1 | `c2V` | `c2v c2V(float,float)` | yes |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v,float)` | yes |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v,c2v)` | yes |
| 4 | `c2Minv` | `c2v c2Minv(c2v,c2v)` | yes |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v,c2v,c2v)` | yes |
| 6 | `c2Sub` | `c2v c2Sub(c2v,c2v)` | yes |
| 7 | `c2Dot` | `float c2Dot(c2v,c2v)` | yes |
| 8 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | yes |
| 9 | `c2xIdentity` | `c2x c2xIdentity(void)` | yes |
| 10 | `c2BBVerts` | `void c2BBVerts(c2v*,c2AABB*)` | yes |
| 11 | `c2MakeProxy` | `void c2MakeProxy(const void*,C2_TYPE,c2Proxy*)` | yes |
| 12 | `c2Len` | `float c2Len(c2v)` | yes |
| 13 | `c2Det2` | `float c2Det2(c2v,c2v)` | yes |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex*)` | yes |
| 15 | `c2Mulrv` | `c2v c2Mulrv(c2r,c2v)` | yes |
| 16 | `c2Add` | `c2v c2Add(c2v,c2v)` | yes |
| 17 | `c2Mulxv` | `c2v c2Mulxv(c2x,c2v)` | yes |
| 18 | `c22` | `void c22(c2Simplex*)` | yes |
| 19 | `c23` | `void c23(c2Simplex*)` | yes |
| 20 | `c2Neg` | `c2v c2Neg(c2v)` | yes |
| 21 | `c2Skew` | `c2v c2Skew(c2v)` | yes |
| 22 | `c2CCW90` | `c2v c2CCW90(c2v)` | yes |
| 23 | `c2D` | `c2v c2D(c2Simplex*)` | yes |
| 24 | `c2Support` | `int c2Support(const c2v*,int,c2v)` | yes |
| 25 | `c2Witness` | `void c2Witness(c2Simplex*,c2v*,c2v*)` | yes |
| 26 | `c2Div` | `c2v c2Div(c2v,float)` | yes |
| 27 | `c2Norm` | `c2v c2Norm(c2v)` | yes |
| 28 | `c2L` | `c2v c2L(c2Simplex*)` | yes |
| 29 | `c2MulrvT` | `c2v c2MulrvT(c2r,c2v)` | yes |
| 30 | `c2GJK` | `float c2GJK(const void*,C2_TYPE,const c2x*,const void*,C2_TYPE,const c2x*,c2v*,c2v*,int,int*,c2GJKCache*)` | yes |
| 31 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB,c2AABB)` | yes |
| 32 | `c2AABBtoCapsule` | `int c2AABBtoCapsule(c2AABB,c2Capsule)` | yes |
| 33 | `c2CapsuletoCapsule` | `int c2CapsuletoCapsule(c2Capsule,c2Capsule)` | yes |
| 34 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle,c2Circle)` | yes |
| 35 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle,c2AABB)` | yes |
| 36 | `c2CircletoCapsule` | `int c2CircletoCapsule(c2Circle,c2Capsule)` | yes |
| 37 | `c2Collided` | `int c2Collided(const void*,C2_TYPE,const void*,C2_TYPE)` | yes |
| 38 | `capsule` | `int capsule(float,float,float,float,float)` | yes |

## Diff

```
$ diff <(nm -D --defined-only c_src/build/*.so      | awk '$2=="T"{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libcapsule_lib.so \
                                                    | awk '$2=="T"{print $3}' | sort)
(empty)
```

**38 / 38 C symbols present in the Rust `.so`, 0 missing.**
No stubs / `unimplemented!()` were used; every symbol has a real translation of
the corresponding C body.

Undefined (imported) symbols in the Rust `.so` are libc/compiler-runtime only
(`memcpy`, `__stack_chk_fail`, `_Unwind_Resume`, `rust_eh_personality`, …) —
verified by `nm -D -u`; no unresolved project symbols.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
configuration is the default one. `cargo check --no-default-features` and
`cargo check` are the same build. (See `configs.sh` / the test log.)
