# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-kHnqEC.so   (name = parent dir name, see CMakeLists.txt)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libhdr_bitrate_lib.so
```

## Complete C source inventory

The library is a single translation unit. Nothing was skipped by the translation:

| C file | lines | contents |
|--------|------:|----------|
| `c_src/include/lib.h` | 3 | `#include <stdint.h>` + the single prototype |
| `c_src/src/lib.c`     | 14 | `hdr_bitrate` + its `static const` table |

`c_src/build/CMakeFiles/**/CMakeCCompilerId.c` is CMake compiler-probe scratch,
not part of the library (it is not listed in `add_library`).

## Defined (exported) dynamic symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `hdr_bitrate` | `T` (0x10f9) | `T` | **present in both** |

Symbol diff (`comm -23` of the two sorted defined-symbol name lists): **empty**.

Total: C exports 1 symbol, Rust exports 1 symbol, 0 missing.

No implementation had to be added and no C module had to be translated: the
single C translation unit yields exactly one public symbol, and the Rust
`#[no_mangle] pub unsafe extern "C" fn hdr_bitrate` wrapper exports it under the
identical name. There are no macro-generated symbols in this library.

## Undefined symbols

The Rust `.so` must not require any non-libc symbol that the C `.so` does not.

C `.so` undefined (all weak, toolchain-injected):

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

Rust `.so` undefined: the same weak toolchain symbols plus libc/`libgcc`
imports pulled in by the Rust runtime (`memcpy`, `__libc_start_main`-family,
unwinder helpers, …). All are libc/toolchain-provided, so there are **0 missing
or undefined non-libc symbols**. This is asserted programmatically by
`tests/symbol_parity.rs`.

## ABI

| item | C | Rust |
|------|---|------|
| name | `hdr_bitrate` | `hdr_bitrate` |
| signature | `unsigned hdr_bitrate(const uint8_t *h)` | `unsafe extern "C" fn(*const c_uchar) -> c_uint` |
| calling convention | SysV C | `extern "C"` |
| return width | `unsigned` (32-bit) | `c_uint` (32-bit) |
| bytes of `*h` read | `h[1]`, `h[2]` only | `h[1]`, `h[2]` only |
