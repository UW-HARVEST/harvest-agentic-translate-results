# SYMBOLS.md — Phase A: symbol surface map

Derived mechanically from `nm -D` on the built shared libraries.

## Build commands

```sh
# C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust translation
cargo build --no-default-features
# -> target/debug/librev16_lib.so
```

## Translation-unit inventory

The C library consists of exactly one source file and one public header. Both
are fully translated — no module/file was skipped.

| C source file | translated to | status |
|---|---|---|
| `c_src/src/lib.c` (9 lines) | `src/lib.rs` | fully translated |
| `c_src/include/lib.h` (3 lines) | `src/lib.rs` (signature) | fully translated |

`c_src/CMakeLists.txt` lists `src/lib.c` as the *only* member of the
`add_library(... SHARED ...)` target, so this inventory is complete by
construction.

## Exported (defined) dynamic symbols

`nm -D --defined-only`, non-`_`-prefixed (i.e. excluding toolchain/libc
housekeeping symbols such as `_init`, `_fini`, `__bss_start`, `_edata`):

| # | symbol | C `.so` | Rust `.so` | kind | signature |
|---|--------|---------|------------|------|-----------|
| 1 | `rev16` | `T` (present) | `T` (present) | text/function | `uint32_t rev16(uint32_t a)` |

### Symbol diff

```
C symbol count:    1
Rust symbol count: 1

MISSING FROM RUST: <empty>
EXTRA IN RUST:     <empty>
```

**Result: 0 missing symbols.** Every symbol the C `.so` exports is exported by
the Rust `.so` under the exact same name. No `#[no_mangle]` wrapper had to be
added and no C source had to be back-filled, because the single C translation
unit was already fully translated.

`rev16` is exported from Rust via:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn rev16(a: u32) -> u32
```

There are no macro-generated symbols in the C source (the source contains no
preprocessor conditionals or function-defining macros — verified by grepping
for `#if`/`#define`), so there is no hidden symbol surface.

## Undefined (imported) symbols

Undefined symbols are *not* required to match: they are an artefact of the
implementation language's runtime, not of the library's ABI.

* C `.so` imports 4 symbols, all toolchain housekeeping:
  `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
  `__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`.
* Rust `.so` imports 49 symbols. Every one is either libc
  (`malloc`, `memcpy`, `write`, `open64`, `pthread_*`, …), the unwinder
  (`_Unwind_*@GCC_*`), or the same toolchain housekeeping symbols. These come
  from the Rust standard library that is statically linked into every `cdylib`.

**Non-libc undefined symbols in the Rust `.so`: 0.** Nothing is left dangling;
the Rust `.so` resolves against a stock glibc + libgcc.

## Completion gate (Phase D)

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 missing/undefined **non-libc** symbols in the Rust `.so`.
