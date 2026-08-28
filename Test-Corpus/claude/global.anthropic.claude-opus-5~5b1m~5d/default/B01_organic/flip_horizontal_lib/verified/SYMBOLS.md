# SYMBOLS.md — Symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-fHPNiD.so   (name = parent dir name, per CMakeLists.txt)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libflip_horizontal_lib.so
```

## C `.so` — defined dynamic symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-fHPNiD.so
00000000000010f9 T flip_horizontal
```

Exactly **one** public symbol. The C library is a single translation unit
(`src/lib.c`) with no macro-generated exports, no static tables, no
constructors/destructors, and no additional entry points.

## Rust `.so` — defined dynamic symbols

```
$ nm -D --defined-only translation/target/release/libflip_horizontal_lib.so
0000000000011c70 T flip_horizontal
```

## Symbol table

| # | C symbol | type | present in Rust `.so`? | Rust item |
|---|----------|------|------------------------|-----------|
| 1 | `flip_horizontal` | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn flip_horizontal` |

## Diff

```
$ comm -23 <(nm -D --defined-only <C.so>  | awk '{print $3}' | sort -u) \
           <(nm -D --defined-only <RS.so> | awk '{print $3}' | sort -u)
(empty)
```

**0 missing symbols.** No module of the C source was skipped: `c_src/src/lib.c`
is 19 lines and its single function is translated. No stubs, no
`unimplemented!()`.

## Undefined symbols

The C `.so` imports only weak toolchain/libc symbols, none of which are part of
the library's API surface:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

**0 missing/undefined non-libc symbols in the Rust `.so`.**

## Types (from `include/lib.h`, not symbols but part of the ABI)

| C type | size / align (x86-64) | layout | Rust counterpart |
|--------|----------------------|--------|------------------|
| `cp_pixel_t` | 4 / 1 | `u8 r; u8 g; u8 b; u8 a;` | `#[repr(C)] cp_pixel_t` |
| `cp_image_t` | 16 / 8 | `int w` @0, `int h` @4, `cp_pixel_t *pix` @8 (4 bytes padding after `h`) | `#[repr(C)] cp_image_t` |

Both are verified at runtime by the differential tests, which construct the
struct once and hand the same raw bytes to both libraries.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the crate has
exactly one configuration (default = empty feature set). There are no `#ifdef`s
in the C source either, so there is no conditional-compilation surface to
cross-check. Phase D's "repeat for every feature combination" therefore reduces
to the single default configuration; the driver script still enumerates and runs
it explicitly.
