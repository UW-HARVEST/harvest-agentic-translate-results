# SYMBOLS.md — Public symbol surface (Phase A)

Mechanically derived from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-Hy8ql9.so`
  (CMake derives the project name from the parent directory name, so the
  file name follows the working-directory name.)
* Rust `.so`: `translation/target/release/libdoubleneg_lib.so`

Reproduce with:

```sh
nm -D --defined-only c_src/build/libharvest-work-*.so     | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libdoubleneg_lib.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # must be EMPTY
```

## Defined (exported) symbols

| # | symbol (C `.so`) | exported by Rust `.so`? | Rust implementation |
|---|------------------|-------------------------|---------------------|
| 1 | `calculate_with_doubles` | yes | `src/doubles.rs` |
| 2 | `convert_double_to_int`  | yes | `src/conv.rs` |
| 3 | `create_numeric_buffer`  | yes | `src/buffer.rs` |
| 4 | `doubleneg`              | yes | `src/doubleneg.rs` |
| 5 | `find_value_in_buffer`   | yes | `src/buffer.rs` |
| 6 | `process_negation`       | yes | `src/negation.rs` |

`c_src/src/lib.c` is the only translation unit; `include/lib.h` declares only
`doubleneg`, but no function is `static`, so all six have external linkage and
all six are part of the ABI. There are no namespacing/renaming macros, so
linker names equal source names. No macro-generated symbols exist.

**Missing from Rust `.so`: 0.** No stubs were introduced; every symbol is a
real translation of the corresponding C function.

## Undefined (imported) symbols

C imports: `memchr`, `pow`, `printf`, `puts` (all glibc/libm) plus the weak
`_ITM_*`, `__cxa_finalize`, `__gmon_start__` markers every ELF DSO carries.

The Rust `.so` imports the same `memchr`, `pow`, `printf`, `puts` (the
translation deliberately calls the *same* libc/libm entry points so that
`%e` formatting and `pow` results are bit-identical) plus the standard Rust
runtime set: `_Unwind_*`, `__errno_location`, `__tls_get_addr`, `abort`,
`bcmp`, `calloc`/`free`/`malloc`/`realloc`/`posix_memalign`,
`close`/`open64`/`read`/`write`/`writev`/`lseek64`/`fstat64`/`stat64`/`statx`,
`dl_iterate_phdr`, `getcwd`, `getenv`, `gettid`, `memcpy`/`memmove`/`memset`,
`mmap64`/`munmap`, `pthread_key_*`/`pthread_setspecific`, `readlink`,
`realpath`, `strlen`, `syscall`.

**Undefined non-libc symbols in the Rust `.so`: 0** — every entry above is
provided by glibc or libgcc_s, both of which are already dependencies of any
process that loads the library.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the
only build configuration is the default one (`--no-default-features` is
equivalent to the default here). Verified mechanically:

```sh
grep -n '^\[features\]' translation/Cargo.toml   # no match
```
