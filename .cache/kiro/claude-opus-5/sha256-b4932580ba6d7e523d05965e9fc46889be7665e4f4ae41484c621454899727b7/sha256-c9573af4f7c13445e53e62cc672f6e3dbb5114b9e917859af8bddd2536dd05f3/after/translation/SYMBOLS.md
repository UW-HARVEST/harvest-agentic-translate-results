# SYMBOLS.md — public symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-sPD1Kl.so
nm -D --defined-only translation/target/release/libpremultiply_lib.so
```

## C source surface

The whole library is two files:

| file | lines | contents |
|---|---|---|
| `c_src/include/lib.h` | 16 | `cp_pixel_t`, `cp_image_t`, `void premultiply(cp_image_t *)` |
| `c_src/src/lib.c` | 20 | definition of `premultiply` |

There are no macros that generate additional symbols, no `#ifdef`-gated
alternate entry points, no additional translation units in `CMakeLists.txt`
(`add_library(... SHARED src/lib.c)` only). So the expected exported surface is
exactly one function.

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|-----------|--------|
| 1 | `premultiply` | `T` (text, global) | `T` (text, global) | PRESENT in both |

Symbol diff (C-defined minus Rust-defined), ignoring absolute/weak
loader/runtime entries:

```
(empty)
```

No symbol is missing, therefore no `#[no_mangle]` wrapper had to be added and
no untranslated C module was found. `c_src/src/lib.c` is translated in full
(`translation/src/lib.rs::premultiply`).

## Undefined (imported) symbols

Both objects import only loader/runtime entries. The C `.so` imports
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`. The Rust `.so` additionally imports the `libgcc` unwinder
(`_Unwind_*`) and libc/pthread entries pulled in by the Rust standard library
(`malloc`, `free`, `memcpy`, `abort`, `dl_iterate_phdr`, `pthread_key_create`,
...). Every one of these is libc / language-runtime, not a library symbol the
translation failed to provide.

**0 missing/undefined non-libc symbols in the Rust `.so`.**

## Types crossing the ABI

Checked against the C ABI on x86-64 SysV:

| C type | size / align | Rust mirror | size / align |
|---|---|---|---|
| `cp_pixel_t` | 4 / 1 | `#[repr(C)] cp_pixel_t` | 4 / 1 |
| `cp_image_t` | 16 / 8 (4 + 4 + 8) | `#[repr(C)] cp_image_t` | 16 / 8 |

`sizeof(cp_pixel_t) == 4` is what the C source uses as its stride/step
constant, matching `PIXEL_SIZE = 4` in the Rust.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. `--no-default-features` is therefore
also a valid (and identical) configuration; both are exercised in Phase D.
