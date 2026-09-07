# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D` on the CMake-built C shared library
(`c_src/build/libharvest-work-sRDPq5.so`) and the Cargo-built Rust cdylib
(`translation/target/release/librev16_lib.so`).

## Commands used

```
nm -D --defined-only c_src/build/libharvest-work-sRDPq5.so
nm -D --defined-only translation/target/release/librev16_lib.so
```

## C `.so` defined symbols

| symbol | type | source |
|--------|------|--------|
| `rev16` | `T` (global text) | `c_src/src/lib.c` |

That is the complete list. `c_src/CMakeLists.txt` compiles exactly one
translation unit (`src/lib.c`), and that file defines exactly one function.
There are no macro-generated symbols, no aliases, no versioned symbols, no
global/static data, and no `#ifdef`-gated extra definitions anywhere in
`c_src/`. The public header `c_src/include/lib.h` declares only `rev16` and
contains no renaming/namespacing macros, so the linker name equals the
source-level name.

## Rust `.so` defined symbols

| symbol | type | source |
|--------|------|--------|
| `rev16` | `T` (global text) | `translation/src/lib.rs` (`#[unsafe(no_mangle)] pub extern "C" fn rev16`) |

## Parity diff

| symbol | in C `.so` | in Rust `.so` | action needed |
|--------|-----------|---------------|---------------|
| `rev16` | yes | yes | none |

**Missing from Rust: 0 symbols.** No implementation is absent, so no C source
had to be translated and no export wrapper had to be added.

## Undefined (imported) symbols

The C `.so` imports only weak libc/toolchain symbols
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`). These are C-runtime artifacts,
not library API. `rev16` itself is a leaf function: it calls nothing.

**0 missing/undefined non-libc symbols in the Rust `.so`.**
