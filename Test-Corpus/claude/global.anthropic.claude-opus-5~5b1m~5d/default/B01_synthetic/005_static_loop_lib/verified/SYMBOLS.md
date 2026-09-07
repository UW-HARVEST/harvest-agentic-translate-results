# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C  `.so`: `c_src/build/libStaticLoop.so`   (cmake, default build type)
- Rust `.so`: `translation/target/release/libStaticLoop.so` (`cargo build --release`)

## C source inventory

The whole C library is exactly two translation-unit-visible functions, both
declared in `include/staticloop.h`:

| C file | function | declared in header |
|--------|----------|--------------------|
| `src/staticloop.c` | `int static_sum(int update)` | yes |
| `src/staticloop.c` | `void driver(int stride)`    | yes |

There are no macro-generated symbols, no exported globals (`sum` is a
function-scope `static`, therefore **not** an exported symbol — verified: it
does not appear in `nm -D` of the C `.so`), and no additional C source files.
So there is no "whole module was never translated" gap in this project.

## Symbol table (C `.so` → Rust `.so`)

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|-----------|--------|
| 1 | `static_sum` | `T` (0x1119) | `T` | **present** |
| 2 | `driver`     | `T` (0x1139) | `T` | **present** |

Symbol diff (`comm -23` of the two sorted defined-symbol name lists): **empty**.

Reproduce with:

```sh
diff <(nm -D --defined-only c_src/build/libStaticLoop.so             | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libStaticLoop.so | awk '{print $3}' | sort)
```

This is asserted automatically by the test `symbols::rust_so_exports_every_c_symbol`
in `tests/differential.rs`, which shells out to `nm` on both objects.

## Undefined symbols in the Rust `.so`

`nm -D -u` on the Rust `.so` lists only libc / libgcc-unwind / ld.so imports:

`_ITM_*`, `_Unwind_*`, `__cxa_finalize`, `__cxa_thread_atexit_impl`,
`__errno_location`, `__gmon_start__`, `__tls_get_addr`, `abort`, `bcmp`,
`calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`,
`gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`,
`munmap`, `open64`, `posix_memalign`, `printf`, `pthread_key_create`,
`pthread_key_delete`, `pthread_setspecific`, `read`, `readlink`, `realloc`,
`realpath`, `stat64`, `statx`, `strlen`, `syscall`, `write`, `writev`.

**0 missing / undefined non-libc symbols.** ✅

Note: `printf@GLIBC_2.2.5` is imported on purpose — the Rust `driver` calls the
very same glibc `printf` the C `driver` calls, so the emitted bytes and the
stdio buffering discipline are identical.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one (`--no-default-features` is equivalent).
Symbol parity and all tests are therefore verified under the single existing
configuration; see `check_all_features.sh`.
