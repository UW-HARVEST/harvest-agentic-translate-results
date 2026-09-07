# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` exported (defined) symbols — `c_src/build/libdriver.so`

| # | symbol | type | C declaration | exported by Rust `.so`? |
|---|--------|------|---------------|-------------------------|
| 1 | `driver` | `T` (text, global) | `void driver(const char *in);` — declared in `include/driver.h` | YES |
| 2 | `run`    | `T` (text, global) | `void run(int extra_bedrooms);` — **not** in the public header, but non-`static` in `src/driver.c`, so it is a real exported entry point | YES |

`nm -D --defined-only` raw output:

```
C:    00000000000012c8 T driver
      00000000000011cf T run
Rust: 0000000000011800 T driver
      00000000000119b0 T run
```

## Symbols deliberately NOT exported (both sides)

These are `static` in `c_src/src/driver.c` and therefore have internal linkage.
They must NOT appear in `nm -D` of either library; the Rust translation keeps
them as private `unsafe fn`s.

| C symbol | kind | Rust counterpart (private) |
|----------|------|----------------------------|
| `the_house`               | `static house_t` file-scope mutable state | `static mut THE_HOUSE: house_t` |
| `add_floor`               | `static void(house_t*)`      | `unsafe fn add_floor` |
| `add_bedrooms`            | `static void(house_t*,int)`  | `unsafe fn add_bedrooms` |
| `add_floor_to_the_house`  | `static void(void)`          | `unsafe fn add_floor_to_the_house` |
| `print_the_house`         | `static void(void)`          | `unsafe fn print_the_house` |
| `parse_val`               | `static bool(const char*,int*)` | `unsafe fn parse_val` |

## Undefined (imported) symbols in the Rust `.so`

`nm -D --undefined-only translation/target/release/libdriver.so` lists only
libc / libgcc-unwind / weak-ELF symbols:

`__errno_location`, `printf`, `puts`, `strtol`, `malloc`, `calloc`, `realloc`,
`free`, `posix_memalign`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`,
`abort`, `getenv`, `getcwd`, `readlink`, `realpath`, `open64`, `close`, `read`,
`write`, `writev`, `lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`,
`munmap`, `syscall`, `dl_iterate_phdr`, `pthread_key_create`,
`pthread_key_delete`, `pthread_setspecific`, `gettid`, `__tls_get_addr`,
`__cxa_finalize`, `__cxa_thread_atexit_impl`, `_Unwind_*`,
`_ITM_(de)registerTMCloneTable`, `__gmon_start__`.

The three that come from the translated code itself are `printf`, `strtol` and
`__errno_location` — exactly the three libc facilities `driver.c` uses
(`<stdio.h>`, `<stdlib.h>`, `<errno.h>`). The remainder are Rust runtime /
panic-machinery imports.

## Verdict

**0 missing symbols.** The C `.so` exports 2 symbols; the Rust `.so` exports both
with the exact same names. **0 undefined non-libc symbols.** No whole C module
was skipped: `c_src` contains exactly one translation unit (`src/driver.c`,
86 lines) plus one header, and every function in it is accounted for above.
