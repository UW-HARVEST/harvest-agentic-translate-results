# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only  c_src/build/libdriver.so
nm -D --defined-only  translation/target/release/libdriver.so
nm -D --undefined-only <both>
```

## Public (defined, dynamic) symbols exported by the C `.so`

| # | symbol | type | present in Rust `.so`? | notes |
|---|--------|------|------------------------|-------|
| 1 | `driver` | `T` (global text) | YES | `#[unsafe(no_mangle)] pub extern "C" fn driver(c_int)` in `src/lib.rs` |

Nothing else. `print_hex` is `static` in `c_src/src/driver.c`, so it is not a
dynamic symbol in the C `.so`; the Rust translation likewise keeps `print_hex`
private, so it contributes no dynamic symbol. `house_t` is a type, not a symbol.

There are no macro-generated exports, no exported globals/data symbols, and no
additional translation units — `CMakeLists.txt` compiles exactly one source
file (`src/driver.c`) into `libdriver.so`, and that file is fully translated in
`translation/src/lib.rs`. No module was skipped, so no missing C source needed
to be translated.

## Symbol diff

```
comm -23 <(nm -D --defined-only c_src/build/libdriver.so    | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
```

Result: **empty** — 0 symbols exported by the C `.so` are missing from the Rust
`.so`.

## Undefined (imported) symbols

| `.so` | undefined symbols | all libc / runtime? |
|-------|-------------------|---------------------|
| C | `printf`, `putchar`, plus the weak CRT/ITM stubs (`__cxa_finalize`, `__gmon_start__`, `_ITM_*`) | yes |
| Rust | `printf`, `putchar`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `malloc`/`calloc`/`realloc`/`free`/`posix_memalign`, `abort`, `__errno_location`, `getenv`, `getcwd`, `open64`/`read`/`write`/`writev`/`close`/`lseek64`/`stat64`/`fstat64`/`statx`/`readlink`/`realpath`, `mmap64`/`munmap`, `syscall`, `dl_iterate_phdr`, `pthread_key_*`/`pthread_setspecific`/`__tls_get_addr`/`__cxa_thread_atexit_impl`/`gettid`, `_Unwind_*` | yes |

**0 missing / undefined non-libc symbols in the Rust `.so`.** The extra Rust
imports are the standard-library/panic-runtime support that `libstd` always
pulls in; none of them is a symbol the C library was expected to provide.

Note: the C `.so` imports `putchar` because GCC rewrites `printf("\n")` into
`putchar('\n')`. The Rust build calls `printf("\n")` directly. Both emit a
single `0x0a` byte to `stdout`, so the observable byte stream is identical; this
is a codegen detail, not an ABI or behavioural difference.

## Verification checklist

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
- [x] Every C-exported symbol (`driver`) is exported by the Rust `.so` with the
      exact same name.
