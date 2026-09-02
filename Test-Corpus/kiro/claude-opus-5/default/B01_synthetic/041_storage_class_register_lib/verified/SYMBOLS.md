# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libdriver.so`:

| # | symbol | type | present in Rust `.so` | notes |
|---|--------|------|------------------------|-------|
| 1 | `driver` | `T` (global text) | YES — `T driver` | The only public API. Declared in `c_src/include/driver.h` as `void driver(int x);`. No namespace/renaming macros exist in the header, so the linker name is plainly `driver`. |

**Missing symbols: 0.** No `#[no_mangle]` wrapper had to be added and no C
module was left untranslated: `c_src` contains exactly one translation unit
(`src/driver.c`, 31 lines incl. the licence header) defining exactly one
function, plus one public header (`include/driver.h`) declaring exactly that
function. `find c_src -type f` lists only `CMakeLists.txt`,
`include/driver.h`, `src/driver.c` — there is no second module to translate.

## Weak / compiler-supplied symbols (not part of the API)

Both objects also carry the usual toolchain-generated weak symbols. These are
emitted by the linker/CRT, not by the library source, so they are not API and
are listed only for completeness:

| symbol | C `.so` | Rust `.so` |
|--------|---------|------------|
| `_ITM_deregisterTMCloneTable` | `w` | `w` |
| `_ITM_registerTMCloneTable` | `w` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | `w` |
| `__gmon_start__` | `w` | `w` |
| `__cxa_thread_atexit_impl@GLIBC_2.18` | — | `w` |
| `gettid@GLIBC_2.30` | — | `w` |
| `statx@GLIBC_2.28` | — | `w` |

## Undefined (imported) symbols

| object | undefined symbols |
|--------|-------------------|
| C | `printf@GLIBC_2.2.5` |
| Rust | `printf@GLIBC_2.2.5` plus the Rust `std`/`libgcc` runtime imports: `_Unwind_*@GCC_*`, `__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`, `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`, `readlink`, `realloc`, `realpath`, `stat64`, `strlen`, `syscall`, `write`, `writev` |

Every Rust undefined symbol resolves against libc (`libc.so.6`) or
`libgcc_s.so.1` — confirmed with `ldd translation/target/release/libdriver.so`,
which reports only `linux-vdso.so.1`, `libgcc_s.so.1`, `libc.so.6` and the
dynamic loader, with no `not found` entries. Both libraries import the same
`printf` from glibc, so the formatting and stdio-buffering behaviour is
literally the same code in both.

## Completion gate

- [x] `nm -D` shows **0 missing** exported symbols in the Rust `.so`
      (`driver` present in both, same name, same `T` binding).
- [x] `nm -D` shows **0 undefined non-libc / non-runtime** symbols in the
      Rust `.so` (verified via `ldd`: nothing unresolved).
