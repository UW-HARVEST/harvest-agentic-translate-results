# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-tOnxOy.so | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libaabb_lib.so | awk '{print $3}' | sort > /tmp/rust_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # missing in Rust
comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt   # extra in Rust
```

C `.so` defined symbols: **38**.  Rust `.so` defined symbols: **38**.
Missing in Rust: **0**.  Extra in Rust: **0**.

`c_src` is a single translation unit (`src/lib.c`) with no `static` functions,
so every function has external linkage and appears below. There are no
macro-generated symbols and no exported data objects.

| # | symbol | C signature | present in Rust `.so` |
|---|--------|-------------|-----------------------|
| 1 | `aabb` | `int aabb(float,float,float,float)` | yes |
| 2 | `c22` | `void c22(c2Simplex*)` | yes |
| 3 | `c23` | `void c23(c2Simplex*)` | yes |
| 4 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB,c2AABB)` | yes |
| 5 | `c2AABBtoCapsule` | `int c2AABBtoCapsule(c2AABB,c2Capsule)` | yes |
| 6 | `c2Add` | `c2v c2Add(c2v,c2v)` | yes |
| 7 | `c2BBVerts` | `void c2BBVerts(c2v*,c2AABB*)` | yes |
| 8 | `c2CCW90` | `c2v c2CCW90(c2v)` | yes |
| 9 | `c2CapsuletoCapsule` | `int c2CapsuletoCapsule(c2Capsule,c2Capsule)` | yes |
| 10 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle,c2AABB)` | yes |
| 11 | `c2CircletoCapsule` | `int c2CircletoCapsule(c2Circle,c2Capsule)` | yes |
| 12 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle,c2Circle)` | yes |
| 13 | `c2Clampv` | `c2v c2Clampv(c2v,c2v,c2v)` | yes |
| 14 | `c2Collided` | `int c2Collided(const void*,C2_TYPE,const void*,C2_TYPE)` | yes |
| 15 | `c2D` | `c2v c2D(c2Simplex*)` | yes |
| 16 | `c2Det2` | `float c2Det2(c2v,c2v)` | yes |
| 17 | `c2Div` | `c2v c2Div(c2v,float)` | yes |
| 18 | `c2Dot` | `float c2Dot(c2v,c2v)` | yes |
| 19 | `c2GJK` | `float c2GJK(const void*,C2_TYPE,const c2x*,const void*,C2_TYPE,const c2x*,c2v*,c2v*,int,int*,c2GJKCache*)` | yes |
| 20 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex*)` | yes |
| 21 | `c2L` | `c2v c2L(c2Simplex*)` | yes |
| 22 | `c2Len` | `float c2Len(c2v)` | yes |
| 23 | `c2MakeProxy` | `void c2MakeProxy(const void*,C2_TYPE,c2Proxy*)` | yes |
| 24 | `c2Maxv` | `c2v c2Maxv(c2v,c2v)` | yes |
| 25 | `c2Minv` | `c2v c2Minv(c2v,c2v)` | yes |
| 26 | `c2Mulrv` | `c2v c2Mulrv(c2r,c2v)` | yes |
| 27 | `c2MulrvT` | `c2v c2MulrvT(c2r,c2v)` | yes |
| 28 | `c2Mulvs` | `c2v c2Mulvs(c2v,float)` | yes |
| 29 | `c2Mulxv` | `c2v c2Mulxv(c2x,c2v)` | yes |
| 30 | `c2Neg` | `c2v c2Neg(c2v)` | yes |
| 31 | `c2Norm` | `c2v c2Norm(c2v)` | yes |
| 32 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | yes |
| 33 | `c2Skew` | `c2v c2Skew(c2v)` | yes |
| 34 | `c2Sub` | `c2v c2Sub(c2v,c2v)` | yes |
| 35 | `c2Support` | `int c2Support(const c2v*,int,c2v)` | yes |
| 36 | `c2V` | `c2v c2V(float,float)` | yes |
| 37 | `c2Witness` | `void c2Witness(c2Simplex*,c2v*,c2v*)` | yes |
| 38 | `c2xIdentity` | `c2x c2xIdentity(void)` | yes |

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports (`memcpy`, `malloc`, `abort`, `_Unwind_*`, `__cxa_finalize`, ...).
**0 non-libc undefined symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `cargo test --no-default-features` is
therefore identical to the default build; both are exercised by
`scripts/run_all.sh`.
