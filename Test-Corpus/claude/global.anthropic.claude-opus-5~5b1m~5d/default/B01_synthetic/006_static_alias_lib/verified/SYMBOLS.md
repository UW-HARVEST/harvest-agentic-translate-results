# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libStaticAlias.so
nm -D --defined-only translation/target/release/libStaticAlias.so
```

## C source inventory (completeness check)

The whole library is three files, so the "was a module skipped?" question is
answered exhaustively:

| C file | contents | translated? |
|--------|----------|-------------|
| `c_src/CMakeLists.txt` | build only, `add_library(StaticAlias SHARED src/staticalias.c)` | n/a |
| `c_src/include/staticalias.h` | declares `static_alias`, `driver`; `STATICALIAS_H_` guard macro (no code) | yes |
| `c_src/src/staticalias.c` | defines `static_alias`, `driver`; function-local `static int inner = 1` | yes (`translation/src/lib.rs`) |

There is exactly **one** translation unit and **two** external definitions in
it. No module/file of the C library is missing from the Rust crate.

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `static_alias` | `T` | `T` | `int *static_alias(int *outer)` → `unsafe extern "C" fn(*mut c_int) -> *mut c_int`, `#[unsafe(no_mangle)]` |
| 2 | `driver`       | `T` | `T` | `void driver(int, int)` → `unsafe extern "C" fn(c_int, c_int)`, `#[unsafe(no_mangle)]` |

**Missing from Rust: none.** The symbol diff (C-defined minus Rust-defined) is
empty. No macro-generated / aliased / versioned exports exist in the C `.so`
(no `#define`-generated function families, no `__attribute__((alias))`, no
version script).

The function-local `static int inner` is **not** an exported symbol in C (it is
a local static with internal linkage); the Rust equivalent (`static mut INNER`)
is likewise not exported. Its address is nevertheless observable through the
return value of `static_alias`, and the tests use exactly that channel to
inspect and control it.

## Undefined (imported) symbols

The C `.so` imports only `printf@GLIBC_2.2.5` plus the standard weak
CRT/ITM/`__cxa_finalize`/`__gmon_start__` set.

The Rust `.so` imports `printf@GLIBC_2.2.5` (the translation deliberately calls
libc `printf` so the emitted bytes and the stdio buffering match C exactly) plus
the Rust standard-library/runtime set: `_Unwind_*`, `malloc`/`free`/`realloc`/
`calloc`/`posix_memalign`, `memcpy`/`memmove`/`memset`/`bcmp`/`strlen`,
`open64`/`read`/`write`/`writev`/`close`/`lseek64`/`stat64`/`fstat64`/`statx`,
`mmap64`/`munmap`, `getcwd`/`getenv`/`readlink`/`realpath`,
`dl_iterate_phdr`/`syscall`/`abort`/`__errno_location`,
`pthread_key_*`/`pthread_setspecific`/`__tls_get_addr`/`__cxa_thread_atexit_impl`/`gettid`.

**0 missing/undefined non-libc symbols** in the Rust `.so`: every `U`/`w` entry
above is resolved by glibc (`libc.so.6`) or libgcc's unwinder
(`libgcc_s.so.1`), both of which the `.so` links against. Verified with
`ldd -r`, which reports no unresolved symbols.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only build
configuration is the default one (`--no-default-features` and the default build
are identical, and there is no non-empty feature combination to enumerate). The
automation in `translation/verify.sh` re-derives the feature list from
`Cargo.toml` and runs every subset of it, rather than assuming there are none,
and additionally repeats the whole suite for the `dev` and `release` profiles.

## Reproducing

```sh
cd translation && ./verify.sh
```
