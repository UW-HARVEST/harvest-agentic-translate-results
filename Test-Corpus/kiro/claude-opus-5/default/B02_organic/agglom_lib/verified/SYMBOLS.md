# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-NoJ8KN.so
nm -D --defined-only translation/target/release/libagglom_lib.so
```

The C library is a single translation unit (`c_src/src/lib.c`, 1104 lines) with
one public header (`c_src/include/lib.h`, which declares only `agglom`).
Everything in `lib.c` that is not `static` is therefore an exported symbol, so
the real public surface is 20 symbols, not the 1 the header advertises.

There are **no** namespace/renaming macros and no macro-generated symbol names
in this library, so linker names equal source-level names.

## Symbol table

| # | C symbol | kind | signature (from C) | present in Rust `.so` |
|---|----------|------|--------------------|------------------------|
| 1 | `c2V` | T | `c2v c2V(float, float)` | yes |
| 2 | `c2Maxv` | T | `c2v c2Maxv(c2v, c2v)` | yes |
| 3 | `c2Minv` | T | `c2v c2Minv(c2v, c2v)` | yes |
| 4 | `c2Clampv` | T | `c2v c2Clampv(c2v, c2v, c2v)` | yes |
| 5 | `c2Sub` | T | `c2v c2Sub(c2v, c2v)` | yes |
| 6 | `c2Dot` | T | `float c2Dot(c2v, c2v)` | yes |
| 7 | `c2CircletoCircle` | T | `int c2CircletoCircle(c2Circle, c2Circle)` | yes |
| 8 | `c2CircletoAABB` | T | `int c2CircletoAABB(c2Circle, c2AABB)` | yes |
| 9 | `c2AABBtoAABB` | T | `int c2AABBtoAABB(c2AABB, c2AABB)` | yes |
| 10 | `f2` | T | `int f2(const void*, C2_TYPE, const void*, C2_TYPE)` | yes |
| 11 | `f3` | T | `int f3(int, int)` | yes |
| 12 | `f4` | T | `double f4(cn_rnd_t*)` | yes |
| 13 | `f5` | T | `uint32_t f5(uint32_t)` | yes |
| 14 | `f7` | T | `tflac_u32 f7(tflac_u32, tflac_u32, tflac_u32)` | yes |
| 15 | `f9` | T | `lm_vec2 f9(lm_vec2, lm_vec2, lm_vec2, lm_vec2)` | yes |
| 16 | `f10` | T | `float f10(uint16_t)` | yes |
| 17 | `f11` | T | `void f11(float*, const float*)` | yes |
| 18 | `f12` | T | `void f12(float*, const float*)` | yes |
| 19 | `f13` | T | `void f13(float*, const float*)` | yes |
| 20 | `agglom` | T | `double agglom(33 scalars)` — the only header-declared entry point | yes |

## `static` (non-exported) C helpers — must NOT be exported, and are not

`cn_rnd_next`, `lm_v2`, `lm_sub2`, `lm_dot2`, `tflac_crc16_tables`,
`m__mantissa`, `m__offset`, `m__exponent`.

`tflac_crc16_tables` is file-static and never referenced by any function; it is
translated in `src/tables.rs` for fidelity but is dead in both builds.

## Result

```
comm -23 <(nm -D --defined-only C.so  | awk '{print $3}' | sort) \
         <(nm -D --defined-only rust.so | awk '{print $3}' | sort)
```

is **empty**: 0 C symbols missing from the Rust `.so`.

`nm -D -u` on the Rust `.so` lists only libc / libgcc-unwind imports
(`malloc`, `memcpy`, `_Unwind_*`, `__errno_location`, …) — 0 undefined
non-libc symbols.

## Static-libm note (verified in Phase B)

`nm` shows the Rust `.so` resolves `floorf` / `fmodf` from
`compiler_builtins::math::libm_math` (local `t` symbols) rather than importing
glibc's, while the C `.so` calls glibc (`-lm`). These are independent
implementations, so `tests/hardening_libm.rs` sweeps the `h` argument of `f11`
(`fmodf`) and `f12` (`floorf`) across **every** f32 exponent — all 256 exponent
values × both signs × 4096 mantissas = 2,097,152 inputs per function, verified
by instrumenting the loop counter — and compares bit-for-bit. No divergence.

`fabsf` is *not* imported by either library: GCC inlines it as `andps` against
`0x7fffffff`, which clears the sign bit without quieting a signalling NaN, so
the Rust uses the equivalent explicit bit operation rather than a libm call.
