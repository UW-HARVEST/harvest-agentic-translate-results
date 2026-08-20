# SYMBOLS.md — Symbol surface parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

## How the two `.so` files are produced

```sh
# C
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> translated_rust/c_src/build/libtranslated_rust.so

# Rust
cd translated_rust && cargo build --no-default-features
# -> translated_rust/target/debug/libbitwriter_add_lib.so
```

## Translation-unit inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source file | lines | translated to | status |
|---|---|---|---|
| `c_src/src/lib.c`     | 24 | `src/lib.rs` (`bitwriter_add`)      | translated |
| `c_src/include/lib.h` | 19 | `src/lib.rs` (`tflac_bitwriter`, typedefs) | translated |

There are **no** untranslated C source files, so there is no "whole module was
skipped" completeness failure to repair.

## Defined (exported) symbols

`nm -D --defined-only` on the C `.so`:

```
0000000000001119 T bitwriter_add
```

`nm -D --defined-only` on the Rust `.so` (filtering the weak ELF/CRT
housekeeping symbols the linker adds to every shared object):

```
0000000000011f60 T bitwriter_add
```

### Parity table

| # | C symbol | type | exported by Rust `.so`? | notes |
|---|----------|------|-------------------------|-------|
| 1 | `bitwriter_add` | `T` (global text) | **YES** — `T bitwriter_add` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn bitwriter_add` in `src/lib.rs` |

**Missing symbols: 0.** The symbol diff is empty.

`struct tflac_bitwriter` and the `tflac_u8` / `tflac_u32` / `tflac_u64` /
`tflac_uint` typedefs are compile-time-only (types produce no ELF symbols);
they are mirrored by `#[repr(C)] pub struct tflac_bitwriter` and the `u8` /
`u32` / `u64` primitives in `src/lib.rs`. Their ABI (size 32, align 8, field
offsets `val`=0, `bits`=8, `pos`=12, `len`=16, `tot`=20, `buffer`=24) is
verified at run time by the differential tests rather than by `nm`.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only

* glibc imports (`malloc`, `memcpy`, `free`, `open64`, `read`, `write`,
  `syscall`, `pthread_key_create`, …),
* libgcc unwinder imports (`_Unwind_*@GCC_*`) pulled in by the default
  `panic = "unwind"` dev profile, and
* the usual weak CRT hooks (`__gmon_start__`, `_ITM_*TMCloneTable`,
  `__cxa_finalize`).

**0 missing / undefined non-libc symbols.** Nothing from the C library is left
dangling.

## Verification command

`./check_symbols.sh` regenerates and diffs both symbol lists and exits
non-zero if any C symbol is absent from the Rust `.so`.
