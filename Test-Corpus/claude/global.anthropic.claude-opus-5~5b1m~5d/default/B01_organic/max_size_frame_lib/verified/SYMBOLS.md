# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C  `.so`: `c_src/build/libharvest-work-E1McXt.so`
- Rust `.so`: `translation/target/release/libmax_size_frame_lib.so`

## C exported symbols (`nm -D --defined-only`)

| # | symbol | type | exported by Rust `.so`? |
|---|--------|------|-------------------------|
| 1 | `max_size_frame` | `T` (global text) | YES — `#[unsafe(no_mangle)] pub extern "C" fn max_size_frame` |

## Rust exported symbols

| # | symbol | type | present in C? |
|---|--------|------|---------------|
| 1 | `max_size_frame` | `T` (global text) | YES |

## Header surface (`c_src/include/lib.h`)

```c
#include <stdint.h>
typedef uint32_t tflac_u32;
tflac_u32 max_size_frame(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth);
```

There are no function-renaming / namespacing macros, no `#ifdef` feature
switches, and no additional translation units in `c_src/CMakeLists.txt`
(`add_library(... SHARED src/lib.c)` only). So the whole library surface is a
single function, and no C source was left untranslated.

`tflac_u32` is a typedef, not a symbol; it contributes nothing to `nm` output.

## Symbol diff

```
$ diff <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only RUST.so | awk '{print $3}' | sort)
(empty)
```

**Missing / undefined non-libc symbols in Rust: 0.** ✅

Undefined symbols in the Rust `.so` are only the libc/runtime imports that a
`cdylib` always pulls in (e.g. `__libc_start_main`-family, `memcpy`,
`_Unwind_*` / `rust_eh_personality` are absent because `panic = "abort"`); none
correspond to C library functions this project was supposed to provide.
