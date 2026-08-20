# SYMBOLS.md — Phase A, step 1: public symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C shared library
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust shared library (cdylib, lib name = hdr_compare_lib)
cd translated_rust && cargo build
# -> target/debug/libhdr_compare_lib.so
```

## C source inventory (completeness check)

The entire C library is two files, and both are accounted for in the Rust
translation — no C source file was skipped:

| C file | contents | Rust counterpart |
|--------|----------|------------------|
| `c_src/include/lib.h` | declares `int hdr_compare(const uint8_t*, const uint8_t*)` | `src/lib.rs` (`#[no_mangle] pub unsafe extern "C" fn hdr_compare`) |
| `c_src/src/lib.c` | `static int hdr_valid(const uint8_t*)`, `int hdr_compare(...)` | `src/lib.rs` (`unsafe fn hdr_valid`, `hdr_compare`) |

`c_src/CMakeLists.txt` compiles exactly one translation unit (`src/lib.c`).

## `nm -D` — DEFINED symbols

### C: `c_src/build/libtranslated_rust.so`

```
0000000000001190 T hdr_compare
```

### Rust: `target/debug/libhdr_compare_lib.so`

```
0000000000012050 T hdr_compare
```

## Parity table

| # | C symbol (`nm -D`, defined) | type | present in Rust `.so`? | notes |
|---|-----------------------------|------|------------------------|-------|
| 1 | `hdr_compare` | `T` (GLOBAL FUNC) | **YES** — `T hdr_compare` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` in `src/lib.rs` |

### Symbols intentionally NOT exported by either library

| C symbol | binding | why not exported |
|----------|---------|------------------|
| `hdr_valid` | `LOCAL FUNC` (`static` in `lib.c`) | `static` linkage — absent from `.dynsym` of the C `.so`; kept private (`unsafe fn`) in Rust. Verified with `readelf --dyn-syms`: only reachable as a LOCAL symtab entry, never a dynamic export. |

## Symbol diff (must be empty)

```sh
comm -23 <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only target/debug/libhdr_compare_lib.so   | awk '{print $NF}' | sort -u)
```

Result: **empty** — 0 C symbols missing from the Rust `.so`.

The reverse direction (Rust-only extra exports) is also empty apart from the
platform-standard weak/glibc entries listed below.

## Undefined symbols in the Rust `.so` (all libc / unwinder)

`nm -D --undefined-only target/debug/libhdr_compare_lib.so` lists only
toolchain-provided symbols, i.e. **0 missing non-libc symbols**:

* weak ELF/TM boilerplate: `_ITM_deregisterTMCloneTable`,
  `_ITM_registerTMCloneTable`, `__gmon_start__`, `__cxa_finalize`,
  `__cxa_thread_atexit_impl`, `gettid`, `statx`
* glibc: `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`,
  `fstat64`, `getcwd`, `getenv`, `lseek64`, `malloc`, `memcpy`, `memmove`,
  `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`,
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`,
  `readlink`, `realloc`, `realpath`, `stat64`, `strlen`, `syscall`,
  `__errno_location`, `__tls_get_addr`, `write`, `writev`
* libgcc unwinder (`_Unwind_*`) — pulled in by the Rust std panic runtime, not
  by translated code

(The C `.so` needs none of these because its single function has no runtime
dependencies; the extra glibc references come from Rust `std`, which the
`cdylib` links in. They are satisfied by the system libraries at load time —
verified by `libloading` successfully opening the Rust `.so` in the tests.)

## Enforced as a test

`tests/symbols.rs` re-derives both export lists with `nm -D --defined-only`
at test time, asserts the C set is `{hdr_compare}`, asserts nothing in it is
missing from the Rust `.so`, `dlsym`s every C symbol out of the Rust `.so`, and
asserts that the `static` helper `hdr_valid` is exported by neither.

Verified for both Cargo profiles (the `release` profile uses
`panic = "abort"`, i.e. different codegen):

| Rust `.so` | exports | diff vs C |
|-----------|---------|-----------|
| `target/debug/libhdr_compare_lib.so` | `T hdr_compare` | empty |
| `target/release/libhdr_compare_lib.so` | `T hdr_compare` | empty |

## Status

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
      the exact same name (`hdr_compare`), in both profiles.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.
- [x] No stubs / `unimplemented!()` were introduced: `hdr_compare` is a full
      translation of the C body, and both C source files are fully translated —
      there is no skipped module.
- [x] The symbol diff (C-defined minus Rust-defined) is **empty**.
