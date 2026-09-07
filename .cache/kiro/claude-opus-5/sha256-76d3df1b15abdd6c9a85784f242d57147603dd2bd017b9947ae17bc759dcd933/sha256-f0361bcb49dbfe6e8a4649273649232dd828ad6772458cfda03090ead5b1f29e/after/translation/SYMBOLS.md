# SYMBOLS.md — public symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory

The whole C library is two files:

| C file | functions defined |
|--------|-------------------|
| `c_src/src/driver.c` | `fma_array`, `call_fma`, `driver` |
| `c_src/include/driver.h` | declares `driver` only (no macros, no name mangling / renaming) |

There are no other translation units, no `#define`-generated symbol names, no
`static` helpers, and no global/exported data objects. So the complete expected
export set is exactly three function symbols.

## Exported (dynamic, defined) symbol table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `fma_array` | `T` | `T` | `#[no_mangle] extern "C"` in `src/driver.rs` |
| 2 | `call_fma`  | `T` | `T` | `#[no_mangle] extern "C"` in `src/driver.rs` |
| 3 | `driver`    | `T` | `T` | `#[no_mangle] extern "C"` in `src/driver.rs` |

**Symbols exported by C but missing from Rust: 0.**
No implementation was absent, so no C source needed to be translated in this
phase and no stubs were introduced.

Rust additionally exports the usual `cdylib` weak/internal artefacts
(`_ITM_*`, `__cxa_finalize`, `__cxa_thread_atexit_impl`, `__gmon_start__`,
`gettid`, `statx` as weak undefined). Extra Rust-side symbols are allowed; the
gate is that no C symbol is missing.

## Undefined symbol audit (non-libc must be empty)

| `.so` | undefined symbols | verdict |
|-------|-------------------|---------|
| C | `__isoc99_sscanf`, `printf` (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`) | all libc/toolchain |
| Rust | `printf`, `sscanf`, `memcpy`, `memset`, `memmove`, `bcmp`, `strlen`, `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `abort`, `__errno_location`, `getenv`, `getcwd`, `readlink`, `realpath`, `open64`, `read`, `write`, `writev`, `close`, `lseek64`, `fstat64`, `stat64`, `mmap64`, `munmap`, `syscall`, `dl_iterate_phdr`, `pthread_key_*`, `pthread_setspecific`, `__tls_get_addr`, `_Unwind_*` | all libc / libgcc unwinder (Rust std + panic machinery) |

**Non-libc undefined symbols in the Rust `.so`: 0.**

Note: the C object references glibc's C99 alias `__isoc99_sscanf` while the Rust
object references the plain `sscanf` entry point. The two differ only in the
handling of the GNU `%a` allocation modifier; for the `"%d%zn"` format actually
used they are the same code path, and the differential tests confirm identical
parsing results.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is also valid and
identical). Verified with:

```
grep -n '\[features\]' translation/Cargo.toml   # -> no match
```

Phase D's "repeat B–C for every feature combination" therefore reduces to the
two equivalent configurations `(default)` and `(no-default-features)`, both of
which are exercised by the test script `run_all.sh`.
