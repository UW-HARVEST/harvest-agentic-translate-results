# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-trFJfb.so
nm -D --defined-only translation/target/release/libagglom_lib.so
```

`c_src/CMakeLists.txt` builds exactly one translation unit (`src/lib.c`) into a
shared library, so the C `.so` public surface is the set of non-`static`
functions in `src/lib.c`. There are no macro-generated symbols and no exported
data objects (all four tables — `tflac_crc16_tables`, `m__mantissa`,
`m__offset`, `m__exponent` — are `static`, hence file-local in C and
`pub(crate)` in Rust).

## Symbol table

| # | C symbol | C signature (`c_src/src/lib.c`) | in C `.so` | in Rust `.so` | Rust item |
|---|----------|----------------------------------|-----------|--------------|-----------|
| 1 | `c2V` | `c2v c2V(float, float)` | T | T | `pub extern "C" fn c2V` |
| 2 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | T | T | `pub extern "C" fn c2Maxv` |
| 3 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | T | T | `pub extern "C" fn c2Minv` |
| 4 | `c2Clampv` | `c2v c2Clampv(c2v, c2v, c2v)` | T | T | `pub extern "C" fn c2Clampv` |
| 5 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | T | T | `pub extern "C" fn c2Sub` |
| 6 | `c2Dot` | `float c2Dot(c2v, c2v)` | T | T | `pub extern "C" fn c2Dot` |
| 7 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle, c2Circle)` | T | T | `pub extern "C" fn c2CircletoCircle` |
| 8 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle, c2AABB)` | T | T | `pub extern "C" fn c2CircletoAABB` |
| 9 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB, c2AABB)` | T | T | `pub extern "C" fn c2AABBtoAABB` |
| 10 | `f2` | `int f2(const void*, C2_TYPE, const void*, C2_TYPE)` | T | T | `pub unsafe extern "C" fn f2` |
| 11 | `f3` | `int f3(int, int)` | T | T | `pub extern "C" fn f3` |
| 12 | `f4` | `double f4(cn_rnd_t*)` | T | T | `pub unsafe extern "C" fn f4` |
| 13 | `f5` | `uint32_t f5(uint32_t)` | T | T | `pub extern "C" fn f5` |
| 14 | `f7` | `tflac_u32 f7(tflac_u32, tflac_u32, tflac_u32)` | T | T | `pub extern "C" fn f7` |
| 15 | `f9` | `lm_vec2 f9(lm_vec2, lm_vec2, lm_vec2, lm_vec2)` | T | T | `pub extern "C" fn f9` |
| 16 | `f10` | `float f10(uint16_t)` | T | T | `pub extern "C" fn f10` |
| 17 | `f11` | `void f11(float*, const float*)` | T | T | `pub unsafe extern "C" fn f11` |
| 18 | `f12` | `void f12(float*, const float*)` | T | T | `pub unsafe extern "C" fn f12` |
| 19 | `f13` | `void f13(float*, const float*)` | T | T | `pub unsafe extern "C" fn f13` |
| 20 | `agglom` | `double agglom(33 scalars)` — see `include/lib.h` | T | T | `pub extern "C" fn agglom` |

## `static` (file-local) C entities — correctly NOT exported by either `.so`

| C entity | kind | Rust counterpart |
|----------|------|------------------|
| `cn_rnd_next` | `static uint64_t(cn_rnd_t*)` | private `fn cn_rnd_next` |
| `lm_v2` | `static lm_vec2(float,float)` | private `fn lm_v2` |
| `lm_sub2` | `static lm_vec2(lm_vec2,lm_vec2)` | private `fn lm_sub2` |
| `lm_dot2` | `static float(lm_vec2,lm_vec2)` | private `fn lm_dot2` |
| `tflac_crc16_tables` | `static const tflac_u16[8][256]` | `pub(crate) static` in `src/tables.rs` (dead, as in C) |
| `m__mantissa` | `static uint32_t[2048]` | `pub(crate) static` in `src/tables.rs` |
| `m__offset` | `static uint16_t[64]` | `pub(crate) static` in `src/tables.rs` |
| `m__exponent` | `static uint32_t[64]` | `pub(crate) static` in `src/tables.rs` |

All four tables were verified element-by-element against the C source
(2048 / 2048 / 64 / 64 values, all identical).

## Result

```
$ diff <(nm -D --defined-only <c.so>  | awk '{print $3}' | sort) \
       <(nm -D --defined-only <rust.so> | awk '{print $3}' | sort)
   (no output)
```

**Missing symbols: 0. Extra symbols: 0.** No module of `src/lib.c` was skipped
by the translation; no stubs are present.

Rust `.so` undefined symbols are only libc (`memcpy`, `malloc`, …) and the
Rust std/unwind runtime (`_Unwind_*`) — 0 missing non-libc symbols.
The C `.so` additionally imports `floorf` and `fmodf` from libm; the Rust
translation implements those inline (`f32::floor`, `%`), which is why they do
not appear as imports.

## Verification (Phase D)

Re-checked against **both** Rust build profiles after the translation fixes:

```
                                      C .so   Rust release   Rust debug
defined dynamic symbols                 20         20            20
missing from Rust (comm -23)             —          0             0
extra   in   Rust (comm -13)             —          0             0
undefined non-libc / non-runtime         —          0             0
```

No symbol required translating a missing module: `src/lib.c` is a single
translation unit and all 20 of its non-`static` functions were already present
in `src/lib.rs`. No stubs, no `unimplemented!()`, no `todo!()` anywhere:

```
$ grep -rnE 'unimplemented!|todo!|panic!\("stub' src/
   (no matches)
```
