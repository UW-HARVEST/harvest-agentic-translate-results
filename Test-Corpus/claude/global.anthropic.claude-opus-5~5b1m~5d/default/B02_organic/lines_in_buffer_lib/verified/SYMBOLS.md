# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## Source of truth

C `.so`: `c_src/build/libdriver.so`
Rust `.so`: `translation/target/{debug,release}/libdriver.so`

Built with:

```sh
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## `nm -D --defined-only` on the C `.so`

```
0000000000001119 T UTIL_createLinePointers
```

That is the complete set — `c_src/` contains exactly one translation unit
(`src/lib.c`, 34 lines) and one public header (`include/lib.h`, 3 lines) which
declares exactly one function. There are no macro-generated symbols, no
`#ifdef`-gated extra entry points, no data symbols, and no additional C files
listed in `CMakeLists.txt` (`add_library(driver SHARED src/lib.c)`).

## Symbol table

| # | symbol | kind | in C `.so` | in Rust `.so` | notes |
|---|--------|------|-----------|---------------|-------|
| 1 | `UTIL_createLinePointers` | `T` (global text) | yes | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` in `src/lib.rs` |

## Missing-symbol analysis

**0 symbols missing.** No module of the C source was skipped; `src/lib.c` is
fully translated in `translation/src/lib.rs`. No stubs, no `unimplemented!()`,
no `todo!()` anywhere in the crate.

## Undefined (imported) symbols

The Rust `.so` must not require any non-libc symbol that the C `.so` does not.

C imports (non-libc): none. C libc imports: `malloc`, `free`.
Rust libc imports: `malloc`, `free` (declared `extern "C"` in `src/lib.rs`),
plus the usual Rust runtime/libc set (`memcpy`, `__cxa_thread_atexit_impl`,
unwinder symbols, …) which are all satisfied by the system libc /
`libgcc_s`. There are no unresolved non-libc symbols.

Verified mechanically by `tests/symbols.rs::c_symbols_are_all_exported_by_rust`
and `tests/symbols.rs::rust_so_has_no_unresolved_non_libc_symbols`.

## ABI signature check

| | C | Rust |
|---|---|---|
| return | `const char**` | `*const *const c_char` |
| arg 0 | `char* buffer` | `*mut c_char` |
| arg 1 | `size_t numLines` | `usize` |
| arg 2 | `size_t bufferSize` | `usize` |
| linkage | `extern` C, default visibility | `#[unsafe(no_mangle)] extern "C"` |

Allocation ABI: the returned block is produced by the **libc `malloc`** in both
implementations (the Rust side calls `malloc`/`free` via `extern "C"`, *not*
Rust's global allocator), so a caller may `free()` the result of either library
interchangeably. The differential tests rely on this and `free()` every
non-NULL result.
