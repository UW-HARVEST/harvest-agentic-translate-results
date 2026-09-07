# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-58LIFt.so`
* Rust `.so`: `translation/target/release/libdiv_euclid_lib.so`

## C source inventory (completeness check)

The whole C library is exactly two files:

| file | contents |
|------|----------|
| `c_src/include/lib.h` | 1 line: `int div_euclid(int v1, int v2);` |
| `c_src/src/lib.c`     | 32 lines: the single definition of `div_euclid` |

`c_src/CMakeLists.txt` builds only `src/lib.c` into one `SHARED` library, and no
executable/driver target. There is therefore **no untranslated C module** — the
Rust crate covers 100% of the C source (`translation/src/lib.rs`, one function).

## Defined (exported) dynamic symbols

`nm -D --defined-only`:

| # | symbol | in C `.so` | in Rust `.so` | status |
|---|--------|-----------|---------------|--------|
| 1 | `div_euclid` | `T` (0x10f9) | `T` (0x11690) | **MATCH** |

Symbol diff (`comm -23` of the two sorted defined-symbol name lists): **empty**.
Reverse diff (Rust-only extra exports): **empty** — the Rust `cdylib` exports no
extra public symbols (`div_euclid` is the only `#[no_mangle] pub extern "C"` item).

## Undefined symbols

Both objects import only libc / runtime symbols; no non-libc symbol is
undefined in either.

C `.so` undefined: `_ITM_deregisterTMCloneTable` (w), `_ITM_registerTMCloneTable`
(w), `__cxa_finalize@GLIBC_2.2.5` (w), `__gmon_start__` (w) — all weak.

Rust `.so` undefined: the same weak entries plus the standard Rust-std libc /
unwinder imports pulled in by `libstd` (`_Unwind_*@GCC_*`, `__errno_location`,
`__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`,
`fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`, `malloc`, `memcpy`,
`memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`,
`pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`,
`readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`,
`write`, `writev`, `__cxa_thread_atexit_impl`).

**0 missing / undefined non-libc symbols in the Rust `.so`.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only build
configuration is the default one (`--no-default-features` is equivalent). The
Phase D "every feature combination" requirement is satisfied by the single
configuration; this is verified by the automated sweep in
`tests/feature_sweep.sh`.

## Completion

- [x] Every C-exported symbol is exported by the Rust `.so` with the exact same name.
- [x] `nm -D` symbol diff is empty in both directions.
- [x] No stubs / `unimplemented!()` / `todo!()` anywhere in `translation/src`.
