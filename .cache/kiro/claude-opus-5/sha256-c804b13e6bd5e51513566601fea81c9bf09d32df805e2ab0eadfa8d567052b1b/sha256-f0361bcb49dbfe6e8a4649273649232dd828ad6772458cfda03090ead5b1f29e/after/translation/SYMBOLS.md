# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C `.so`:    `c_src/build/libharvest-work-cPQcWe.so`
* Rust `.so`: `translation/target/release/libcall_predict_lib.so`

## Defined (exported) symbols

`nm -D --defined-only` output:

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `call_predict` | `T` | `T` | OK — exported by both |

**C exported symbol count (non-weak, defined): 1**
**Missing from Rust `.so`: 0**

## Weak / compiler-generated entries present in the C `.so`

These are emitted by the toolchain (crtstuff / glibc), not by `src/lib.c`, and are
not part of the library's API surface. Both objects carry them:

| symbol | C | Rust |
|--------|---|------|
| `_ITM_deregisterTMCloneTable` | `w` | `w` |
| `_ITM_registerTMCloneTable` | `w` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | `w` |
| `__gmon_start__` | `w` | `w` |

## Symbols in the C source that are NOT exported

All of these are `static` in `c_src/src/lib.c`, so the C compiler gives them
internal linkage and they do not appear in `.dynsym`. The Rust translation
mirrors this by leaving them private (no `#[no_mangle]`):

`BTAC1C2_PredictSample`, `BTAC1C2_PredictSample_Pfn0` … `BTAC1C2_PredictSample_Pfn11`,
`BTAC1C2_GetPredictFunc`.

`c_src/include/lib.h` declares `int get_predict_func(int pfcn);`, but **no such
function is defined anywhere in the C sources**. It is therefore absent from the
C `.so` (`nm -D` confirms) and is correctly absent from the Rust `.so` too.
Adding it to Rust would be a fabricated symbol, which is worse than parity.

## Undefined (imported) symbols in the Rust `.so`

`nm -D -u` on the Rust `.so` lists only libc / libgcc-unwind imports:

`_Unwind_*` (libgcc), `__cxa_thread_atexit_impl`, `__errno_location`,
`__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`,
`fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`, `malloc`, `memcpy`,
`memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`,
`pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`,
`readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`,
`write`, `writev`.

**Non-libc undefined symbols: 0.**

## Feature combinations

`translation/Cargo.toml` declares an optional, non-default feature
`test_internals`, used only to expose the `static` C helpers for differential
testing (see `CONFIGS.md`). The default build's exported surface is byte-for-byte
the single `call_predict` symbol above.

| feature combo | exported symbols |
|---|---|
| (default / `--no-default-features`) | `call_predict` — exact parity with C |
| `--features test_internals` | `call_predict` + `rsw_predict_sample`, `rsw_pfn0` … `rsw_pfn11` (test scaffolding only) |

## Gate

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` relative to the C `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
