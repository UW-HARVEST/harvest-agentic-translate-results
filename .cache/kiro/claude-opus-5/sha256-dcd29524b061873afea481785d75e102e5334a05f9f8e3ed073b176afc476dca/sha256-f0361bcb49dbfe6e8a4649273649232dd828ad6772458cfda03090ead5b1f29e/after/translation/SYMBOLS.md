# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-a2hQdn.so | awk '$2=="T"{print $3}' | sort
nm -D --defined-only translation/target/release/libgjk_lib.so | awk '$2=="T"{print $3}' | sort
```

C `.so` exports: **31**  ·  Rust `.so` exports: **31**  ·  missing: **0**  ·  extra: **0**

| # | symbol | C `.so` | Rust `.so` | C decl | notes |
|---|--------|---------|------------|--------|-------|
| 1 | `c2V` | T | T | `c2v c2V(float,float)` | leaf ctor |
| 2 | `c2Mulvs` | T | T | `c2v c2Mulvs(c2v,float)` | |
| 3 | `c2Maxv` | T | T | `c2v c2Maxv(c2v,c2v)` | ternary max, NaN-sensitive |
| 4 | `c2Minv` | T | T | `c2v c2Minv(c2v,c2v)` | ternary min, NaN-sensitive |
| 5 | `c2Clampv` | T | T | `c2v c2Clampv(c2v,c2v,c2v)` | dead in `gjk` path, still exported |
| 6 | `c2Sub` | T | T | `c2v c2Sub(c2v,c2v)` | |
| 7 | `c2Dot` | T | T | `float c2Dot(c2v,c2v)` | |
| 8 | `c2RotIdentity` | T | T | `c2r c2RotIdentity(void)` | |
| 9 | `c2xIdentity` | T | T | `c2x c2xIdentity(void)` | returns 16-byte struct |
| 10 | `c2BBVerts` | T | T | `void c2BBVerts(c2v*,c2AABB*)` | writes 4 verts |
| 11 | `c2MakeProxy` | T | T | `void c2MakeProxy(const void*,C2_TYPE,c2Proxy*)` | `switch` w/o `default` |
| 12 | `c2Len` | T | T | `float c2Len(c2v)` | `sqrtf` |
| 13 | `c2Det2` | T | T | `float c2Det2(c2v,c2v)` | |
| 14 | `c2GJKSimplexMetric` | T | T | `float c2GJKSimplexMetric(c2Simplex*)` | `default:` falls into `case 1:` |
| 15 | `c2Mulrv` | T | T | `c2v c2Mulrv(c2r,c2v)` | |
| 16 | `c2Add` | T | T | `c2v c2Add(c2v,c2v)` | |
| 17 | `c2Mulxv` | T | T | `c2v c2Mulxv(c2x,c2v)` | |
| 18 | `c22` | T | T | `void c22(c2Simplex*)` | 3 branches |
| 19 | `c23` | T | T | `void c23(c2Simplex*)` | 7 branches |
| 20 | `c2Neg` | T | T | `c2v c2Neg(c2v)` | signed-zero sensitive |
| 21 | `c2Skew` | T | T | `c2v c2Skew(c2v)` | |
| 22 | `c2CCW90` | T | T | `c2v c2CCW90(c2v)` | |
| 23 | `c2D` | T | T | `c2v c2D(c2Simplex*)` | count 1/2/3+default |
| 24 | `c2Support` | T | T | `int c2Support(const c2v*,int,c2v)` | reads `verts[0]` unconditionally |
| 25 | `c2Witness` | T | T | `void c2Witness(c2Simplex*,c2v*,c2v*)` | count 1/2/3+default |
| 26 | `c2Div` | T | T | `c2v c2Div(c2v,float)` | div-by-zero → inf/NaN |
| 27 | `c2Norm` | T | T | `c2v c2Norm(c2v)` | |
| 28 | `c2L` | T | T | `c2v c2L(c2Simplex*)` | count 1/2+default |
| 29 | `c2GJK` | T | T | `float c2GJK(...11 args...)` | main driver |
| 30 | `c2MulrvT` | T | T | `c2v c2MulrvT(c2r,c2v)` | |
| 31 | `gjk` | T | T | `void gjk(char,c2v*,c2v*,9×float)` | public header entry point |

## Undefined (imported) symbol check

Rust `.so` undefined symbols are all libc / libgcc-unwind
(`memcpy`, `malloc`, `abort`, `_Unwind_*`, `__cxa_finalize`, …). There are
**0 missing/undefined non-libc symbols**. The C `.so` additionally imports
`sqrtf@GLIBC`; the Rust build lowers `f32::sqrt` to the `sqrtss` instruction
inline, which is the identical IEEE-754 operation, so no import is needed.

## Types that must match layout (verified by static assertions in tests)

| type | size | align | note |
|------|------|-------|------|
| `c2v` | 8 | 4 | |
| `c2r` | 8 | 4 | |
| `c2x` | 16 | 4 | |
| `c2Circle` | 12 | 4 | |
| `c2AABB` | 16 | 4 | |
| `c2Capsule` | 20 | 4 | |
| `c2GJKCache` | 36 | 4 | `float,int,int[3],int[3],float` |
| `c2Proxy` | 72 | 4 | `float,int,c2v[8]` |
| `c2sv` | 36 | 4 | `c2v,c2v,c2v,float,int,int` |
| `c2Simplex` | 152 | 4 | `c2sv a,b,c,d` modelled as `[c2sv;4]`; `div`@144, `count`@148 |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one. `--no-default-features` is therefore
equivalent to the default build; both are exercised in Phase D.
