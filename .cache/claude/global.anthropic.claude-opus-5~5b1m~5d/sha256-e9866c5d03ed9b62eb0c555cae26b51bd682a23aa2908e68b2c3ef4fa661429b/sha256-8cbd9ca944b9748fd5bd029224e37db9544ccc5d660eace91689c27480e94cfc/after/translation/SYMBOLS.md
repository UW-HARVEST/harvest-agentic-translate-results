# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C `.so`:    `c_src/build/libharvest-work-Pdk2bS.so`
* Rust `.so`: `translation/target/release/libnormalize_lib.so`

## C source inventory (completeness check)

The whole C library is two files:

| C file | functions defined |
|--------|-------------------|
| `c_src/include/lib.h` | (declaration only) `void normalize(float *dest, const float *src, int size);` |
| `c_src/src/lib.c`     | `normalize` |

There are **no other translation units**, no macro-generated symbol families, no
`#ifdef`-gated extra modules. So the complete public surface is exactly one
function. Nothing was skipped by the translation step.

## Exported (defined) symbols — `nm -D --defined-only`

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `normalize` | `T` (0x1119) | `T` (0x11690) | ✅ present in both |

Symbol diff (`comm -23` of the two sorted defined-symbol lists): **empty**.

## Undefined symbols — `nm -D --undefined-only`

C `.so` needs: `memset`, `sqrtf` (+ the standard weak
`_ITM_*` / `__cxa_finalize` / `__gmon_start__` glibc/CRT markers).

Rust `.so` needs: `memset`, `memcpy`, `memmove`, `bcmp`, `malloc`, `calloc`,
`realloc`, `free`, `posix_memalign`, `abort`, `getenv`, `getcwd`, `readlink`,
`realpath`, `open64`, `close`, `read`, `write`, `writev`, `lseek64`, `stat64`,
`fstat64`, `statx`, `mmap64`, `munmap`, `strlen`, `syscall`, `dl_iterate_phdr`,
`__errno_location`, `__tls_get_addr`, `pthread_key_*`, `pthread_setspecific`,
`gettid`, `_Unwind_*`, plus the same weak CRT markers.

`sqrtf` is not an undefined import in the Rust `.so` because `f32::sqrt`
lowers to the `sqrtss` instruction inline; this is the same IEEE-754
correctly-rounded operation glibc's `sqrtf` performs, so behaviour is identical.

All remaining Rust imports are **libc / libgcc-unwind runtime** symbols pulled in
by `std` (allocator, panic machinery, backtrace support). There are **0 missing or
undefined non-libc symbols**.

## Gate

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
- [x] Every symbol the C `.so` exports is exported by the Rust `.so` with the
      exact same name.
