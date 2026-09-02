# SYMBOLS.md — Phase A / Phase D symbol parity

Derived mechanically from `nm -D --defined-only` on both shared objects.

Commands:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (`c_src/src/driver.c`, `c_src/include/driver.h`)

| C symbol    | C linkage           | in C `.so` `nm -D` | translated in Rust | exported by Rust `.so` |
|-------------|---------------------|--------------------|--------------------|------------------------|
| `driver`    | external (`void driver(float)`) | yes    | yes (`src/lib.rs`) | yes                    |
| `print_hex` | `static` (internal) | no (not dynamic)   | yes (private `fn print_hex`) | no — correct, C does not export it either |

There is exactly one translation unit and exactly one public header, which
declares exactly one function. No macro-generated symbols exist in the C source
(the only preprocessor directives are the `DRIVER_H_` include guard and
`#include <stdio.h>`). Therefore no whole C module is missing from the Rust
crate: `c_src/src/driver.c` is the complete C implementation and it is fully
translated.

## Dynamic symbol tables

C `.so` defined dynamic symbols:

```
0000000000001173 T driver
```

Rust `.so` defined dynamic symbols:

```
0000000000011720 T driver
```

## Diff

| direction                         | symbols |
|-----------------------------------|---------|
| in C `.so` but missing from Rust `.so` | *(none)* |
| in Rust `.so` but not in C `.so`  | *(none)* |

**Symbol diff is EMPTY.**

## Undefined (imported) symbols

The Rust `.so` imports only libc symbols, which is required and expected because
the translation deliberately calls the C library's `printf` so that output goes
through the identical `stdout` FILE stream with identical buffering semantics.

| group | symbols | libc / toolchain runtime? |
|-------|---------|---------------------------|
| the one the translation actually calls | `printf@GLIBC_2.2.5` | yes — the *same* glibc `printf` the C `.so` calls |
| glibc | `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`, `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `putchar`, `read`, `readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`, `write`, `writev`, `__errno_location`, `__tls_get_addr`, `__cxa_finalize`, `__cxa_thread_atexit_impl` | yes |
| libgcc unwinder | `_Unwind_Backtrace`, `_Unwind_Resume`, `_Unwind_GetIP`, `_Unwind_GetIPInfo`, `_Unwind_SetGR`, `_Unwind_SetIP`, `_Unwind_GetDataRelBase`, `_Unwind_GetTextRelBase`, `_Unwind_GetRegionStart`, `_Unwind_GetLanguageSpecificData` | yes |
| weak, unresolved is fine | `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__gmon_start__` | yes (also weak-undefined in the C `.so`) |

Everything the Rust `.so` imports beyond `printf` comes from the Rust standard
library's allocator / TLS / panic-backtrace machinery, i.e. toolchain runtime,
not from untranslated C. The C `.so` additionally imports `putchar` because gcc
strength-reduced `printf("\n")` to `putchar('\n')`; both emit exactly the byte
`0x0a` to `stdout`, so the observable output is unaffected.

**0 missing / 0 undefined non-libc symbols.** Verified with
`nm -D --undefined-only translation/target/release/libdriver.so`.

## Enforcement

Parity is not a one-off observation; it is re-checked mechanically on every run:

* `tests/phase_d_symbol_parity.rs::d_symbol_parity_c_subset_of_rust` — every C
  export exists in the Rust `.so`
* `tests/phase_d_symbol_parity.rs::d_symbol_parity_no_extra_rust_exports` — the
  Rust `.so` leaks nothing extra, in particular not the `static` `print_hex`
* `tests/phase_d_symbol_parity.rs::d_no_undefined_non_libc_symbols` — no
  undefined non-libc import that would indicate untranslated C
* `./check_all_features.sh` — repeats the `nm -D` comparison for every feature
  combination before running the suite

## How to reproduce

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release && cargo test
./check_all_features.sh          # all feature combinations
```

The tests `dlopen` `translation/target/release/libdriver.so`, so
`cargo build --release` must run before `cargo test` for the tests to exercise
the current source. `check_all_features.sh` does this in the right order for
every combination.
