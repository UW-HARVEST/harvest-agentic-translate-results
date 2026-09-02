# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` builds exactly one translation unit:

```cmake
add_library(driver SHARED
    src/driver.c)
```

`c_src` contains only `include/driver.h` and `src/driver.c`. There is no
untranslated module — the whole library is one 36-line file, and both of its
non-`static` functions are translated in `translation/src/lib.rs`. No stubs,
no `unimplemented!()`.

## Exported symbol table

| # | symbol | C `.so` | Rust `.so` | declared in `driver.h`? | notes |
|---|--------|---------|------------|-------------------------|-------|
| 1 | `driver` | `T` | `T` | yes | `void driver(char data)` |
| 2 | `printHexCharLine` | `T` | `T` | **no** | `void printHexCharLine(char charHex)` — not in the header but not `static`, so it is part of the exported ABI and is a genuine low-level entry point that must be tested directly |

No macro-generated symbols exist in this library (no function-generating macros
in the C source).

## Symbol diff

```sh
nm -D --defined-only c_src/build/libdriver.so                | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # in C, missing from Rust
```

Result: **empty**. 0 symbols missing from the Rust `.so`.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only translation/target/release/libdriver.so` lists only
libc / libgcc-unwind / ld.so imports:

* `printf@GLIBC_2.2.5` — the single import the translation actually calls; it is
  deliberately the *same* `printf` the C library calls, which is what makes the
  formatted output and the stdout buffering behaviour byte-identical.
* `malloc`, `free`, `calloc`, `realloc`, `posix_memalign`, `memcpy`, `memmove`,
  `memset`, `bcmp`, `strlen`, `abort`, `getenv`, `getcwd`, `readlink`,
  `realpath`, `open64`, `read`, `write`, `writev`, `close`, `lseek64`,
  `stat64`, `fstat64`, `statx`, `mmap64`, `munmap`, `syscall`, `gettid`,
  `__errno_location`, `__cxa_finalize`, `__cxa_thread_atexit_impl`,
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`,
  `dl_iterate_phdr`, `__tls_get_addr` — pulled in by the Rust `std` runtime.
* `_Unwind_*@GCC_*` — libgcc unwinder, pulled in by `std`.
* `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`,
  `__gmon_start__` — weak, optional toolchain hooks present in every ELF DSO
  (also present in the C `.so`).

**0 missing/undefined non-libc symbols.** Gate satisfied.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one (`--no-default-features` is equivalent). The
"every feature combination" requirement collapses to a single combination; it is
still enumerated mechanically (not hard-coded) and exercised by
`translation/run_all_features.sh`, which runs `cargo check`, builds the cdylib in
**both** the debug and release profiles, re-checks symbol parity per artifact,
and runs the whole differential suite against each.

## ABI note discovered during verification

Symbol *names* matched from the start, but the exported `printHexCharLine`
wrapper was not ABI-behaviour-identical. See the "Divergence found and fixed"
section of `CONFIGS.md`.

