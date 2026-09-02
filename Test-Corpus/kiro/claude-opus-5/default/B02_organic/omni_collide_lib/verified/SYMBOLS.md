# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-dJ3KRO.so   | awk '$2=="T"||$2=="W"{print $3}' | sort > c_syms.txt
nm -D --defined-only translation/target/release/libomni_collide_lib.so | awk '$2=="T"||$2=="W"{print $3}' | sort > rust_syms.txt
comm -23 c_syms.txt rust_syms.txt   # missing in Rust
comm -13 c_syms.txt rust_syms.txt   # extra in Rust
```

* C `.so` exported (T/W) symbols: **39**
* Rust `.so` exported (T/W) symbols: **39**
* Missing in Rust: **0**
* Extra in Rust: **0**
* Undefined non-libc symbols in Rust `.so`: **0** (only libc/libgcc/std imports)

The C library is a single translation unit (`c_src/src/lib.c`); no C module was
skipped by the translation, so no additional C source had to be translated.
`lib.h` declares only `omni_collide`, but every other function in `lib.c` is
non-`static` and therefore also part of the dynamic surface; all of them are
re-exported from Rust with `#[unsafe(no_mangle)] extern "C"`.

| # | symbol | C signature (from `c_src/src/lib.c`) | Rust export | present |
|---|--------|--------------------------------------|-------------|---------|
| 1 | `c2V` | `c2v c2V(float, float)` | `extern "C" fn c2V` | yes |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v, float)` | `extern "C" fn c2Mulvs` | yes |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | `extern "C" fn c2Maxv` | yes |
| 4 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | `extern "C" fn c2Minv` | yes |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v, c2v, c2v)` | `extern "C" fn c2Clampv` | yes |
| 6 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | `extern "C" fn c2Sub` | yes |
| 7 | `c2Dot` | `float c2Dot(c2v, c2v)` | `extern "C" fn c2Dot` | yes |
| 8 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | `extern "C" fn c2RotIdentity` | yes |
| 9 | `c2xIdentity` | `c2x c2xIdentity(void)` | `extern "C" fn c2xIdentity` | yes |
| 10 | `c2BBVerts` | `void c2BBVerts(c2v*, c2AABB*)` | `extern "C" fn c2BBVerts` | yes |
| 11 | `c2MakeProxy` | `void c2MakeProxy(const void*, C2_TYPE, c2Proxy*)` | `extern "C" fn c2MakeProxy` | yes |
| 12 | `c2Len` | `float c2Len(c2v)` | `extern "C" fn c2Len` | yes |
| 13 | `c2Det2` | `float c2Det2(c2v, c2v)` | `extern "C" fn c2Det2` | yes |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex*)` | `extern "C" fn c2GJKSimplexMetric` | yes |
| 15 | `c2Mulrv` | `c2v c2Mulrv(c2r, c2v)` | `extern "C" fn c2Mulrv` | yes |
| 16 | `c2Add` | `c2v c2Add(c2v, c2v)` | `extern "C" fn c2Add` | yes |
| 17 | `c2Mulxv` | `c2v c2Mulxv(c2x, c2v)` | `extern "C" fn c2Mulxv` | yes |
| 18 | `c22` | `void c22(c2Simplex*)` | `extern "C" fn c22` | yes |
| 19 | `c23` | `void c23(c2Simplex*)` | `extern "C" fn c23` | yes |
| 20 | `c2Neg` | `c2v c2Neg(c2v)` | `extern "C" fn c2Neg` | yes |
| 21 | `c2Skew` | `c2v c2Skew(c2v)` | `extern "C" fn c2Skew` | yes |
| 22 | `c2CCW90` | `c2v c2CCW90(c2v)` | `extern "C" fn c2CCW90` | yes |
| 23 | `c2D` | `c2v c2D(c2Simplex*)` | `extern "C" fn c2D` | yes |
| 24 | `c2Support` | `int c2Support(const c2v*, int, c2v)` | `extern "C" fn c2Support` | yes |
| 25 | `c2Witness` | `void c2Witness(c2Simplex*, c2v*, c2v*)` | `extern "C" fn c2Witness` | yes |
| 26 | `c2Div` | `c2v c2Div(c2v, float)` | `extern "C" fn c2Div` | yes |
| 27 | `c2Norm` | `c2v c2Norm(c2v)` | `extern "C" fn c2Norm` | yes |
| 28 | `c2L` | `c2v c2L(c2Simplex*)` | `extern "C" fn c2L` | yes |
| 29 | `c2MulrvT` | `c2v c2MulrvT(c2r, c2v)` | `extern "C" fn c2MulrvT` | yes |
| 30 | `c2GJK` | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` | `extern "C" fn c2GJK` | yes |
| 31 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB, c2AABB)` | `extern "C" fn c2AABBtoAABB` | yes |
| 32 | `c2AABBtoCapsule` | `int c2AABBtoCapsule(c2AABB, c2Capsule)` | `extern "C" fn c2AABBtoCapsule` | yes |
| 33 | `c2CapsuletoCapsule` | `int c2CapsuletoCapsule(c2Capsule, c2Capsule)` | `extern "C" fn c2CapsuletoCapsule` | yes |
| 34 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle, c2Circle)` | `extern "C" fn c2CircletoCircle` | yes |
| 35 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle, c2AABB)` | `extern "C" fn c2CircletoAABB` | yes |
| 36 | `c2CircletoCapsule` | `int c2CircletoCapsule(c2Circle, c2Capsule)` | `extern "C" fn c2CircletoCapsule` | yes |
| 37 | `c2Collided` | `int c2Collided(const void*, C2_TYPE, const void*, C2_TYPE)` | `extern "C" fn c2Collided` | yes |
| 38 | `ptr_from_parts` | `void* ptr_from_parts(C2_TYPE, float, float, float, float, float)` | `extern "C" fn ptr_from_parts` | yes |
| 39 | `omni_collide` | `int omni_collide(C2_TYPE, float×5, C2_TYPE, float×5)` | `extern "C" fn omni_collide` | yes |

## Enum values (checked against `c_src/include/lib.h`)

| name | value |
|------|-------|
| `C2_TYPE_CAPSULE` | 0 |
| `C2_TYPE_CIRCLE` | 1 |
| `C2_TYPE_AABB` | 2 |

## Struct layouts (verified by differential FFI tests)

| struct | C layout | size |
|--------|----------|------|
| `c2v` | `float x, y` | 8 |
| `c2r` | `float c, s` | 8 |
| `c2x` | `c2v p; c2r r` | 16 |
| `c2Circle` | `c2v p; float r` | 12 |
| `c2AABB` | `c2v min, max` | 16 |
| `c2Capsule` | `c2v a, b; float r` | 20 |
| `c2GJKCache` | `float metric; int count; int iA[3]; int iB[3]; float div` | 36 |
| `c2Proxy` | `float radius; int count; c2v verts[8]` | 72 |
| `c2sv` | `c2v sA, sB, p; float u; int iA, iB` | 36 |
| `c2Simplex` | `c2sv a,b,c,d; float div; int count` | 152 |

## Cargo feature combinations

`translation/Cargo.toml` declares **no** `[features]` section, so the only
build configuration is the default one (`--no-default-features` is equivalent).
Symbol parity and all differential tests were run under that single
configuration; see `run_all.sh`.

## Verified result (Phase D)

```
$ nm -D --defined-only c_src/build/libharvest-work-dJ3KRO.so | awk '$2=="T"||$2=="W"{print $3}' | sort | wc -l
39
$ nm -D --defined-only translation/target/release/libomni_collide_lib.so | awk '$2=="T"||$2=="W"{print $3}' | sort | wc -l
39
$ comm -23 c_syms.txt rust_syms.txt   # missing in Rust
(empty)
$ comm -13 c_syms.txt rust_syms.txt   # extra in Rust
(empty)
```

The same diff is empty for the **debug-profile** cdylib (also 39 exports), and
`nm -D --undefined-only` on the Rust `.so` lists only glibc/libgcc/`std` runtime
imports — no unresolved project symbols.

## Notes on how the exports were made to match

* No C module was missing: `c_src` is a single translation unit and every
  non-`static` function in it already had a Rust counterpart. Nothing was
  stubbed and no `unimplemented!()` exists in the crate.
* All 39 Rust functions carry `#[unsafe(no_mangle)] extern "C"` **and**
  `#[inline(never)]`. The `inline(never)` matters for behavioural parity, not
  just for the symbol table: the C is built at `-O0`, so `c2Dot`, `c2Sub`,
  `c2Add`, `c2Mulvs`, `c2Det2`, `c2V` … are real calls with one fixed
  instruction sequence everywhere they are used. Letting LLVM inline and
  re-schedule them per call site changed which NaN payload propagated inside
  `c22`/`c23`/`c2Witness`/`c2GJK`.
* Scalar float arithmetic goes through the private `fp` module, which pins the
  SSE destination register with inline `asm!` so it matches the instruction gcc
  emits at each site (see the note at the end of `ERRORS.md`).
