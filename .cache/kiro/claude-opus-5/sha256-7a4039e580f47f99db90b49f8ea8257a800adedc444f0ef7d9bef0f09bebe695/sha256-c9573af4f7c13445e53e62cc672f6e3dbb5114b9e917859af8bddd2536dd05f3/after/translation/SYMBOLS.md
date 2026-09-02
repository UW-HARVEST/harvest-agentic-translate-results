# SYMBOLS.md — public ABI surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#  -> c_src/build/libharvest-work-iwe3AI.so
cd translation && cargo build --release
#  -> translation/target/release/libmax_size_frame_lib.so
```

## Defined (exported) symbols

`nm -D --defined-only` output:

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `max_size_frame` | `T` (0x10f9) | `T` (0x116a0) | MATCH |

Symbol count: C = 1, Rust = 1. **Symbol diff is empty.**

The C header `c_src/include/lib.h` declares exactly one function and no
namespace-renaming/macro-generated symbols:

```c
typedef uint32_t tflac_u32;
tflac_u32 max_size_frame(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth);
```

`c_src/src/lib.c` defines exactly that one function (verified: `grep -c '^[a-z].*('`
finds a single non-static definition, and the file is 10 lines long). There is no
untranslated C module, so no Phase A "translate the missing source" work applies.

The Rust side exports it via `#[unsafe(no_mangle)] pub extern "C" fn max_size_frame`
(the `unsafe(...)` wrapper form is required by edition 2024, which this crate uses).

## Undefined symbols

`nm -D --undefined-only`:

* C: only weak CRT/ITM stubs — `_ITM_deregisterTMCloneTable`,
  `_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`.
* Rust: the same weak CRT/ITM stubs, plus libc (`malloc`, `memcpy`, `write`,
  `open64`, ...) and `_Unwind_*` from GCC's unwinder. All of these come from the
  Rust standard library / panic runtime that is linked into every `cdylib`.

**0 missing/undefined non-libc symbols in the Rust `.so`.** Every undefined entry
is either a weak CRT stub also present in the C library, a libc import, or a
libgcc unwinder import; none is an unresolved symbol from this crate.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section, so the crate has
exactly one configuration (default, which is empty). `cargo check
--no-default-features` and the default build are therefore the same build; both
are exercised. There are no `#ifdef`/`#if` directives in the C source either
(verified by grep), so there is no conditional-compilation surface on either side.
