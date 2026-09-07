# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-qM4rJt.so` (from `c_src/src/lib.c`)
* Rust `.so`: `translation/target/release/libgjk_lib.so`

```
nm -D --defined-only <so> | awk '{print $3}' | sort
```

## Result

* C exports: **31**
* Rust exports: **31**
* Missing in Rust: **0**
* Extra in Rust: **0**

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind /
`__gmon_start__` / `_ITM_*` imports — **0** missing non-libc symbols.

## Table

| # | symbol | C signature | exported by Rust | Rust item |
|---|--------|-------------|------------------|-----------|
| 1 | `c2V` | `c2v c2V(float, float)` | yes | `pub extern "C" fn c2V` |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v, float)` | yes | `pub extern "C" fn c2Mulvs` |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | yes | `pub extern "C" fn c2Maxv` |
| 4 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | yes | `pub extern "C" fn c2Minv` |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v, c2v, c2v)` | yes | `pub extern "C" fn c2Clampv` |
| 6 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | yes | `pub extern "C" fn c2Sub` |
| 7 | `c2Add` | `c2v c2Add(c2v, c2v)` | yes | `pub extern "C" fn c2Add` |
| 8 | `c2Dot` | `float c2Dot(c2v, c2v)` | yes | `pub extern "C" fn c2Dot` |
| 9 | `c2Det2` | `float c2Det2(c2v, c2v)` | yes | `pub extern "C" fn c2Det2` |
| 10 | `c2Len` | `float c2Len(c2v)` | yes | `pub extern "C" fn c2Len` |
| 11 | `c2Div` | `c2v c2Div(c2v, float)` | yes | `pub extern "C" fn c2Div` |
| 12 | `c2Norm` | `c2v c2Norm(c2v)` | yes | `pub extern "C" fn c2Norm` |
| 13 | `c2Neg` | `c2v c2Neg(c2v)` | yes | `pub extern "C" fn c2Neg` |
| 14 | `c2Skew` | `c2v c2Skew(c2v)` | yes | `pub extern "C" fn c2Skew` |
| 15 | `c2CCW90` | `c2v c2CCW90(c2v)` | yes | `pub extern "C" fn c2CCW90` |
| 16 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | yes | `pub extern "C" fn c2RotIdentity` |
| 17 | `c2xIdentity` | `c2x c2xIdentity(void)` | yes | `pub extern "C" fn c2xIdentity` |
| 18 | `c2Mulrv` | `c2v c2Mulrv(c2r, c2v)` | yes | `pub extern "C" fn c2Mulrv` |
| 19 | `c2MulrvT` | `c2v c2MulrvT(c2r, c2v)` | yes | `pub extern "C" fn c2MulrvT` |
| 20 | `c2Mulxv` | `c2v c2Mulxv(c2x, c2v)` | yes | `pub extern "C" fn c2Mulxv` |
| 21 | `c2BBVerts` | `void c2BBVerts(c2v*, c2AABB*)` | yes | `pub unsafe extern "C" fn c2BBVerts` |
| 22 | `c2MakeProxy` | `void c2MakeProxy(const void*, C2_TYPE, c2Proxy*)` | yes | `pub unsafe extern "C" fn c2MakeProxy` |
| 23 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex*)` | yes | `pub unsafe extern "C" fn c2GJKSimplexMetric` |
| 24 | `c22` | `void c22(c2Simplex*)` | yes | `pub unsafe extern "C" fn c22` |
| 25 | `c23` | `void c23(c2Simplex*)` | yes | `pub unsafe extern "C" fn c23` |
| 26 | `c2D` | `c2v c2D(c2Simplex*)` | yes | `pub unsafe extern "C" fn c2D` |
| 27 | `c2L` | `c2v c2L(c2Simplex*)` | yes | `pub unsafe extern "C" fn c2L` |
| 28 | `c2Support` | `int c2Support(const c2v*, int, c2v)` | yes | `pub unsafe extern "C" fn c2Support` |
| 29 | `c2Witness` | `void c2Witness(c2Simplex*, c2v*, c2v*)` | yes | `pub unsafe extern "C" fn c2Witness` |
| 30 | `c2GJK` | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` | yes | `pub unsafe extern "C" fn c2GJK` |
| 31 | `gjk` | `void gjk(char, c2v*, c2v*, float x9)` | yes | `pub unsafe extern "C" fn gjk` |

## Notes on ABI-relevant types (verified by `const _` layout asserts in lib.rs)

| type | size | note |
|------|------|------|
| `c2v` | 8 | returned in one XMM (packed 2×f32) |
| `c2r` | 8 | |
| `c2x` | 16 | returned in XMM0+XMM1 |
| `c2Circle` | 12 | |
| `c2AABB` | 16 | |
| `c2Capsule` | 20 | |
| `c2GJKCache` | 36 | `metric, count, iA[3], iB[3], div` |
| `c2Proxy` | 72 | `radius, count, verts[8]` — **`verts` beyond `count` is left uninitialised by `c2MakeProxy`** |
| `c2sv` | 36 | |
| `c2Simplex` | 152 | `a,b,c,d` modelled as `verts[4]` in Rust |

`C2_TYPE` is an unfixed C enum ⇒ passed as `int`; any `int` value is a legal
argument across the FFI boundary (see `ERRORS.md` row 1).

## Build / project shape

`c_src/CMakeLists.txt` builds **only** a `SHARED` library (`add_library(... SHARED src/lib.c)`)
linked against `m`. **There is no binary/driver executable**, so the
"compare C and Rust stdout" clause of the completion gate is vacuous.

`translation/Cargo.toml` declares `crate-type = ["cdylib"]`, no `[features]`
section ⇒ **exactly one feature combination exists** (the empty default). The
"repeat under every feature combination" clause therefore collapses to the
single default build, confirmed by the feature-combination sweep script.
