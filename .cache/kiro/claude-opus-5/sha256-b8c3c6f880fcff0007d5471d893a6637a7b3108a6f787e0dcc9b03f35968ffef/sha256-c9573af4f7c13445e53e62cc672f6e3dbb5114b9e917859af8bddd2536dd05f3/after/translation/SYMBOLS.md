# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-Nn7f7C.so | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort
nm -D --defined-only translation/target/release/libgjk_cache_lib.so | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort
```

C `.so` defined symbols: **31**. Rust `.so` defined symbols: **31**.
`comm -23` (C-only) → empty. `comm -13` (Rust-only) → empty.

All 31 non-static C functions in `c_src/src/lib.c` are exported by the Rust
`cdylib` under the exact same name. There is exactly one translation unit in
the C project (`src/lib.c`), so there is no un-translated module.

| # | symbol | C signature (from `c_src/src/lib.c`) | in C `.so` | in Rust `.so` |
|---|--------|--------------------------------------|:----------:|:-------------:|
| 1 | `c2V` | `c2v c2V(float x, float y)` | T | T |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v a, float b)` | T | T |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v a, c2v b)` | T | T |
| 4 | `c2Minv` | `c2v c2Minv(c2v a, c2v b)` | T | T |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v a, c2v lo, c2v hi)` | T | T |
| 6 | `c2Sub` | `c2v c2Sub(c2v a, c2v b)` | T | T |
| 7 | `c2Dot` | `float c2Dot(c2v a, c2v b)` | T | T |
| 8 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | T | T |
| 9 | `c2xIdentity` | `c2x c2xIdentity(void)` | T | T |
| 10 | `c2BBVerts` | `void c2BBVerts(c2v *out, c2AABB *bb)` | T | T |
| 11 | `c2MakeProxy` | `void c2MakeProxy(const void *shape, C2_TYPE type, c2Proxy *p)` | T | T |
| 12 | `c2Len` | `float c2Len(c2v a)` | T | T |
| 13 | `c2Det2` | `float c2Det2(c2v a, c2v b)` | T | T |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex *s)` | T | T |
| 15 | `c2Mulrv` | `c2v c2Mulrv(c2r a, c2v b)` | T | T |
| 16 | `c2Add` | `c2v c2Add(c2v a, c2v b)` | T | T |
| 17 | `c2Mulxv` | `c2v c2Mulxv(c2x a, c2v b)` | T | T |
| 18 | `c22` | `void c22(c2Simplex *s)` | T | T |
| 19 | `c23` | `void c23(c2Simplex *s)` | T | T |
| 20 | `c2Neg` | `c2v c2Neg(c2v a)` | T | T |
| 21 | `c2Skew` | `c2v c2Skew(c2v a)` | T | T |
| 22 | `c2CCW90` | `c2v c2CCW90(c2v a)` | T | T |
| 23 | `c2D` | `c2v c2D(c2Simplex *s)` | T | T |
| 24 | `c2Support` | `int c2Support(const c2v *verts, int count, c2v d)` | T | T |
| 25 | `c2Witness` | `void c2Witness(c2Simplex *s, c2v *a, c2v *b)` | T | T |
| 26 | `c2Div` | `c2v c2Div(c2v a, float b)` | T | T |
| 27 | `c2Norm` | `c2v c2Norm(c2v a)` | T | T |
| 28 | `c2L` | `c2v c2L(c2Simplex *s)` | T | T |
| 29 | `c2MulrvT` | `c2v c2MulrvT(c2r a, c2v b)` | T | T |
| 30 | `c2GJK` | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` | T | T |
| 31 | `gjk_cache` | `void gjk_cache(char, c2v*, c2v*, float a1..a4, float b1..b5)` | T | T |

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc_s
(`_Unwind_*`, `__cxa_*`, `malloc`, `memcpy`, `mmap64`, `pthread_key_*`, …)
imports pulled in by the Rust runtime. **0 missing/undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` section, so the only
build configuration is the default one (`--no-default-features` is equivalent).
Verified by `grep -n '\[features\]' Cargo.toml` → no match.

## Layout notes required for parity

* `c2Simplex` in C is `c2sv a, b, c, d; float div; int count;` (152 bytes). The
  Rust translation uses `verts: [c2sv; 4]` because the C code aliases the four
  members through `c2sv *verts = &s.a;`. Offsets are identical.
* `c2sv` = 36 bytes (`sA` 0, `sB` 8, `p` 16, `u` 24, `iA` 28, `iB` 32).
* `c2Proxy` = 72 bytes (`radius` 0, `count` 4, `verts` 8..72).
* `c2GJKCache` = 36 bytes (`metric` 0, `count` 4, `iA` 8..20, `iB` 20..32, `div` 32).
* `C2_TYPE` is a plain C enum passed as a 32-bit int; the Rust side takes `u32`
  so any out-of-range value crosses the boundary unchanged.
