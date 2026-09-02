# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so | sort
nm -D --defined-only translation/target/release/libdriver.so | sort
nm -D -u  translation/target/release/libdriver.so | sort
```

## C source inventory (`c_src/src/driver.c`)

| C function | linkage | translated? | Rust item |
|---|---|---|---|
| `fma_array` | external (`T`) | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn fma_array` |
| `inner`     | `static` (internal, no external linkage — correctly absent from both `.so`s) | yes | private `fn inner` |
| `driver`    | external (`T`) | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |

There is exactly one C translation unit (`src/driver.c`, per `CMakeLists.txt`),
so no module/file was skipped. No macro-generated symbols exist in this source.

## Exported (defined) symbol parity

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `fma_array` | `T` | `T` | MATCH |
| 2 | `driver`    | `T` | `T` | MATCH |

C exports not in Rust: **none**.
Rust extra non-libc exports: **none**.

## Undefined symbols in the Rust `.so`

All undefined (`U` / `w`) symbols in the Rust `.so` resolve to the platform C
library / unwinder and are therefore permitted:

* libc: `memcpy`, `printf`, `malloc`, `calloc`, `realloc`, `free`,
  `posix_memalign`, `memmove`, `memset`, `bcmp`, `strlen`, `abort`,
  `__errno_location`, `getenv`, `getcwd`, `readlink`, `realpath`,
  `open64`, `close`, `read`, `write`, `writev`, `lseek64`, `fstat64`,
  `stat64`, `statx`, `mmap64`, `munmap`, `syscall`, `dl_iterate_phdr`,
  `gettid`, `pthread_key_create`, `pthread_key_delete`,
  `pthread_setspecific`, `__tls_get_addr`, `__cxa_finalize`,
  `__cxa_thread_atexit_impl`
* GCC unwinder (Rust panic machinery): `_Unwind_*`
* Standard weak ELF hooks: `_ITM_registerTMCloneTable`,
  `_ITM_deregisterTMCloneTable`, `__gmon_start__`

`memcpy` and `printf` are the same two libc imports the C `.so` has; the Rust
translation calls the platform `printf`/`memcpy` directly so that stdout
formatting and buffering are byte-identical.

**Result: 0 missing symbols, 0 undefined non-libc symbols. Gate PASSES.**

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section, so the crate has
exactly one configuration (default = no features). `--no-default-features` and
the default build are the same code path; both are checked/tested by
`check_feature_combos.sh`.
