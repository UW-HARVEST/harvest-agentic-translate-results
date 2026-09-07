# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```
nm -D --defined-only  c_src/build/libdriver.so
nm -D --defined-only  translation/target/release/libdriver.so
nm -D --undefined-only translation/target/release/libdriver.so
```

## C source inventory (ground truth)

Every function in `c_src/` (from `grep -nE '^[a-zA-Z_].*\(' src/*.c include/*.h`):

| C function | file:line | linkage | exported? |
|---|---|---|---|
| `print_hex(unsigned char *p, int len)` | `src/driver.c:28` | `static` (internal) | NO — `static`, must NOT appear in `nm -D` |
| `driver(float x)` | `src/driver.c:35`, decl `include/driver.h:27` | external | YES |

There is exactly ONE translation unit (`src/driver.c`, per `CMakeLists.txt`
`add_library(driver SHARED src/driver.c)`), so no C module is missing from the
translation. `translation/src/lib.rs` covers both functions.

## Defined dynamic symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `driver` | `T` (0x1173) | `T` (0x11720) | MATCH |

`print_hex` is `static` in C and correspondingly a private `unsafe fn` in Rust —
absent from both `.so`s. Correct parity.

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so            | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(empty)
```

**Missing-from-Rust count: 0.** No `#[no_mangle]` wrapper needs to be added and
no C module needs translating.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports. Grouped:

- libc used by the translation itself: `printf`, `putchar`
  (`putchar` is LLVM's `SimplifyLibCalls` rewrite of `printf("\n")` — the C
  `.so` imports `putchar` for the identical reason, see below)
- Rust std runtime / allocator: `malloc`, `calloc`, `realloc`, `free`,
  `posix_memalign`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `abort`,
  `__errno_location`
- Rust std panic/backtrace machinery: `_Unwind_*` (libgcc), `dl_iterate_phdr`
- Rust std TLS / thread glue: `__tls_get_addr`, `__cxa_thread_atexit_impl`,
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `gettid`
- Rust std file/IO glue: `open64`, `read`, `write`, `writev`, `close`,
  `lseek64`, `stat64`, `fstat64`, `statx`, `readlink`, `realpath`, `getcwd`,
  `getenv`, `mmap64`, `munmap`, `syscall`
- weak ELF boilerplate present in BOTH: `_ITM_deregisterTMCloneTable`,
  `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`

**Non-libc undefined symbols: 0.** `ldd` resolves the Rust `.so` fully against
`libgcc_s.so.1` + `libc.so.6` only.

For reference, the C `.so`'s undefined set is
`printf`, `putchar` + the same weak boilerplate — i.e. the C compiler performed
the same `printf("\n")` → `putchar('\n')` libcall simplification that LLVM did
for the Rust build. Both emit the byte `0x0a`, so behaviour is identical.

## Gate

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` (diff is empty).
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
- [x] `static` C function `print_hex` is exported by neither `.so`.
