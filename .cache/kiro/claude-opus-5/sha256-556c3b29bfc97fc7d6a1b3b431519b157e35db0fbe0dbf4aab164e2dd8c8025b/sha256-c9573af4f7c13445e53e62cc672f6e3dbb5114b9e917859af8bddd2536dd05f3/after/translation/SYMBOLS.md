# SYMBOLS.md — Phase A symbol map

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Exported (defined, dynamic) symbols

### C `libdriver.so`

```
0000000000001119 T driver
```

That is the complete list — one symbol. It matches the single declaration in
`c_src/include/driver.h`:

```c
void driver(const char *s1, const char *s2);
```

There are no namespace/prefix macros, no macro-generated symbol families, no
`__attribute__((alias))`, and no additional translation units: `CMakeLists.txt`
builds exactly one source file, `src/driver.c`.

### Rust `libdriver.so`

```
00000000000116d0 T driver
```

## Parity table

| # | C symbol | type | exported by Rust `.so`? | Rust definition |
|---|----------|------|-------------------------|-----------------|
| 1 | `driver` | `T` (global text) | YES — exact name match | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` in `src/lib.rs` |

**Missing symbols: 0.** No `#[no_mangle]` wrapper had to be added and no C
module was left untranslated — `src/driver.c` is the only C source file in the
project and its only function is `driver`.

## Undefined (imported) symbols

Neither library is allowed to leave a non-libc symbol undefined.

C `libdriver.so` imports:

| symbol | provider |
|--------|----------|
| `printf@GLIBC_2.2.5` | libc |
| `strcspn@GLIBC_2.2.5` | libc |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__` | weak, libc/ITM toolchain glue |

Rust `libdriver.so` imports `printf@GLIBC_2.2.5` (the translation deliberately
calls the *same* libc `printf` so the formatted bytes and the stdio buffering
behaviour are identical) plus the standard Rust runtime set: `_Unwind_*`
(libgcc), `__errno_location`, `abort`, `bcmp`, `calloc`/`malloc`/`realloc`/
`free`/`posix_memalign`, `memcpy`/`memmove`/`memset`, `strlen`, `read`/`write`/
`writev`/`close`/`open64`/`lseek64`/`fstat64`/`stat64`/`statx`/`readlink`/
`realpath`/`getcwd`, `mmap64`/`munmap`, `getenv`, `syscall`, `dl_iterate_phdr`,
`pthread_key_*`/`pthread_setspecific`/`__tls_get_addr`/`__cxa_thread_atexit_impl`,
`gettid`.

**Non-libc / non-runtime undefined symbols in the Rust `.so`: 0.** Every entry
above is resolved by glibc or libgcc, both of which are already dependencies of
the C library's own runtime environment.

Note that the Rust `.so` importing `strcspn` is *not* required: `strcspn` is an
internal, non-exported helper in the C library (it is a libc call, not part of
the C library's public surface), so reimplementing it inside Rust does not
change the exported ABI. What matters is that the *observable behaviour* of that
reimplementation matches glibc's, which is what `CONFIGS.md` and `ERRORS.md`
drive tests for.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section, so the only build
configurations are the default one and `--no-default-features` (identical, since
there are no default features). Both are covered by the test scripts.
