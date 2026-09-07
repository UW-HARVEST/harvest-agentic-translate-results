# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-YBwZSQ.so` (built via CMake, `src/lib.c`)
* Rust `.so`: `translation/target/release/libhsv_to_rgb_lib.so` (`crate-type = ["cdylib"]`)

## C source inventory

The whole C library is a single translation unit, `c_src/src/lib.c` (59 lines),
declaring exactly one public entry point in `c_src/include/lib.h`:

```c
void hsv_to_rgb(float *dest, const float *src);
```

There are no other C source files, no macro-generated symbol families, no
global/static data with external linkage, and no `#ifdef`-gated alternate
implementations. So the expected exported surface is one symbol.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | type | status |
|---|--------|---------|-----------|------|--------|
| 1 | `hsv_to_rgb` | `T` (0x1109) | `T` (0x11710) | `void(float*, const float*)` | ✅ present in both, exact name |

## Non-libc undefined symbols

C `.so` imports (`nm -D` `U` entries): `floorf` (libm) — libc/libm only.
Rust `.so` imports: libc only (`memcpy`/`__cxa_*`-class runtime entries as
emitted by rustc); `floorf` is inlined as an SSE4.1 `roundss`/soft-float
`f32::floor`, which is bit-identical to libm `floorf` (both are exact,
correctly-rounded operations on all inputs including NaN/±Inf/±0).

## Result

```
$ comm -3 <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
          <(nm -D --defined-only rust.so | awk '{print $3}' | sort)
(empty)
```

**0 symbols missing from the Rust `.so`. 0 undefined non-libc symbols.**
No stubs, no `unimplemented!()`, no untranslated C modules — the entire C
library (one function) is translated in `translation/src/lib.rs`.
