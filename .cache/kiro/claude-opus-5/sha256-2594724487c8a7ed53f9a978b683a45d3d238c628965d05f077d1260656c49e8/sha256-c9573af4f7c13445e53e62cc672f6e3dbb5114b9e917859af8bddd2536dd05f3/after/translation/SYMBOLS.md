# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only build/libharvest-work-DqQCoH.so

# Rust
cd translation && cargo build --release
nm -D --defined-only target/release/libhdr_bitrate_lib.so
```

## C `.so` dynamic symbol table (defined)

| # | symbol | type | source |
|---|--------|------|--------|
| 1 | `hdr_bitrate` | `T` (global text) | `c_src/src/lib.c` |

`c_src/include/lib.h` declares exactly one function and there are no
namespace-renaming macros, no `#define`-generated symbol aliases, and no other
translation units in `CMakeLists.txt` (`add_library(... SHARED src/lib.c)`), so
the surface is a single symbol.

`halfrate.0` appears in the *static* symbol table (`nm` without `-D`) as a
function-local `static const` array. It is **not** a dynamic symbol and is not
part of the ABI, so it is not required in the Rust `.so`.

## Rust `.so` dynamic symbol table (defined, non-libc)

| # | symbol | type | source |
|---|--------|------|--------|
| 1 | `hdr_bitrate` | `T` (global text) | `translation/src/lib.rs` (`#[unsafe(no_mangle)] pub unsafe extern "C" fn`) |

## Diff

| symbol | in C `.so` | in Rust `.so` | status |
|--------|-----------|--------------|--------|
| `hdr_bitrate` | yes | yes | MATCH |

**Missing from Rust: none.** No implementation had to be added and no C module
was left untranslated — `src/lib.c` is the only C source file and its single
function is fully translated.

## Undefined (imported) symbols in the Rust `.so`

All `U` entries are the standard C runtime / unwinder imports emitted for any
`cdylib` (`__cxa_thread_atexit_impl`, `_ITM_*`, `__tls_get_addr`,
`__gmon_start__`, and libc/`libgcc_s` entries). There are **0 undefined
non-libc symbols** — the library has no unresolved Rust-side references.

## Verification gate

- [x] `nm -D` shows 0 symbols present in the C `.so` but missing from the Rust `.so`.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.
- [x] No symbol is a stub / `unimplemented!()`; `hdr_bitrate` is a real translation.
