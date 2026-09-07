# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```sh
nm -D --defined-only c_src/build/libSieve.so
nm -D --defined-only translation/target/release/libSieve.so
nm -D -u   <each .so>
```

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `sieve` | `T` (0x1109) | `T` (0x116e0) | `void sieve(int)`; the only public symbol declared by `include/sieve.h` |

The C source (`src/sieve.c`) contains exactly one function definition and no
file-scope data, no `static` helpers exposed via macros, and no macro-generated
symbol families. So the full C export surface is a single symbol.

### Symbol diff

```
C defined-only symbols not exported by Rust:   (none)
Rust defined-only symbols not exported by C:   (none)
```

**Diff is empty.** No `#[no_mangle]` wrapper needed to be added and no C module
was left untranslated: `src/sieve.c` is the only C translation unit in
`CMakeLists.txt` (`add_library(Sieve SHARED src/sieve.c)`), and it is fully
translated in `translation/src/lib.rs`.

## Undefined (imported) symbols

C `.so` imports:

```
U printf@GLIBC_2.2.5
w _ITM_deregisterTMCloneTable, _ITM_registerTMCloneTable,
  __cxa_finalize@GLIBC_2.2.5, __gmon_start__      (weak, toolchain-generated)
```

Rust `.so` imports `printf@GLIBC_2.2.5` as well (the translation deliberately
calls libc `printf` rather than `println!`, so both libraries share the same
`FILE *stdout` buffer and emit identical bytes), plus the Rust standard
library's own libc/`libgcc` dependencies:

`__errno_location, abort, bcmp, calloc, close, dl_iterate_phdr, free, fstat64,
getcwd, getenv, lseek64, malloc, memcpy, memmove, memset, mmap64, munmap,
open64, posix_memalign, pthread_key_create, pthread_key_delete,
pthread_setspecific, read, readlink, realloc, realpath, stat64, strlen,
syscall, write, writev` (all libc) and `_Unwind_*` (libgcc unwinder), plus the
weak `__cxa_thread_atexit_impl`, `gettid`, `statx`.

**Non-libc / non-runtime undefined symbols in the Rust `.so`: 0.** Every
imported symbol resolves from `libc`/`libgcc_s`, both of which are already
loaded in any process that loads the library (verified: `ldd` resolves all of
them, and the differential tests successfully `dlopen` the Rust `.so`).

## Gate

- [x] `nm -D` shows 0 symbols missing from the Rust `.so`.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.
