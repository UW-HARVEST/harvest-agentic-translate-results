# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#  -> c_src/build/libharvest-work-HArM1X.so
cd translation && cargo build --release
#  -> translation/target/release/libflip_horizontal_lib.so
```

## C source inventory (completeness check)

The whole C library is a single translation unit:

| C file | translated? | Rust location |
|--------|-------------|---------------|
| `c_src/include/lib.h` (`cp_pixel_t`, `cp_image_t`, decl of `flip_horizontal`) | yes | `translation/src/lib.rs` (`cp_pixel_t`, `cp_image_t`) |
| `c_src/src/lib.c` (`flip_horizontal`) | yes | `translation/src/lib.rs::flip_horizontal` |

No C module is missing; there is nothing to stub.

## Exported (defined, dynamic) symbols

`nm -D --defined-only`:

| # | symbol | type | in C `.so` | in Rust `.so` | status |
|---|--------|------|-----------|---------------|--------|
| 1 | `flip_horizontal` | `T` (global text) | yes | yes | MATCH |

There are no macro-generated symbols, no aliases, no versioned symbols, no
exported data objects in the C `.so`.

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-HArM1X.so   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libflip_horizontal_lib.so | awk '{print $NF}' | sort)
```

Result: **empty** — 0 symbols missing from the Rust `.so`.

(The Rust `.so` additionally exports nothing else; only the standard ELF
housekeeping entries appear as weak/undefined, see below.)

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind /
pthread imports (`malloc`, `memcpy`, `_Unwind_*`, `__errno_location`, …) that
come from the Rust standard library. **0 missing/undefined non-libc symbols.**

## Types crossing the FFI boundary

| C type | size / layout (x86-64) | Rust type | layout |
|--------|------------------------|-----------|--------|
| `cp_pixel_t` | 4 bytes, `uint8_t r,g,b,a`, align 1 | `#[repr(C)] cp_pixel_t` | 4 bytes, align 1 |
| `cp_image_t` | 16 bytes: `int w` @0, `int h` @4, `cp_pixel_t *pix` @8, align 8 | `#[repr(C)] cp_image_t` | 16 bytes, align 8 |
| `void flip_horizontal(cp_image_t *)` | — | `unsafe extern "C" fn(*mut cp_image_t)` | — |

Layout parity is asserted at runtime by the test
`layout::c_struct_layout_assumptions`.
