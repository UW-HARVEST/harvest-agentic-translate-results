# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

## Build commands

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-UrYD9e.so

cd translation && cargo build --release
# -> translation/target/release/libhalf2float_lib.so
```

## C `.so` exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libharvest-work-UrYD9e.so`

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `half2float` | `T` (global text) | YES — `#[unsafe(no_mangle)] pub extern "C" fn half2float` |

That is the complete list. The C translation unit contains exactly one
external definition; `m__mantissa`, `m__offset` and `m__exponent` are `static`
(internal linkage) in C, so they are deliberately NOT exported and must NOT be
exported by Rust either. The Rust translation keeps them as private
(non-`#[no_mangle]`) `static` items, which matches.

## Rust `.so` exported dynamic symbols

`nm -D --defined-only translation/target/release/libhalf2float_lib.so`,
filtering out Rust-runtime/std internals (`_ZN…`, `rust_…`, `__rust…`):

| # | symbol | type |
|---|--------|------|
| 1 | `half2float` | `T` (global text) |

## Symbol diff

| direction | result |
|-----------|--------|
| C-exported symbols missing from Rust `.so` | **0 (empty)** |
| Rust non-libc/non-std symbols undefined at load time | **0** |

No symbol required translation work: the single C entry point already has a
matching `extern "C"` export wrapper. No C module was skipped — `src/lib.c` is
the only C source file listed in `c_src/CMakeLists.txt`, and all three of its
static tables plus its one function are present in `translation/src/lib.rs`
(table contents verified element-by-element, 2048 + 64 + 64 values, all equal).

## Header surface

`c_src/include/lib.h` in full:

```c
#include <stdint.h>

float half2float(uint16_t h);
```

One declaration, one definition, one export. Symbol parity is complete.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configurations are the (empty) default feature set and
`--no-default-features`, which are identical. Both are exercised; see
`CONFIGS.md`.
