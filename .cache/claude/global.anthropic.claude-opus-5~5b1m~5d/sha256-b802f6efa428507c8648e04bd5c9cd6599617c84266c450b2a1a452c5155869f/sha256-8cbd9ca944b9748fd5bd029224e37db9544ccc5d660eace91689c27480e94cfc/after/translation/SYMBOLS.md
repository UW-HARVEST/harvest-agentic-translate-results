# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C:    `c_src/build/libdriver.so`
- Rust: `translation/target/release/libdriver.so`

## C `.so` exported (defined, global) symbols

```
$ nm -D --defined-only c_src/build/libdriver.so
000000000000122b T driver
0000000000001129 T fma_array
```

## Rust `.so` exported (defined, global `T`) symbols

```
$ nm -D --defined-only translation/target/release/libdriver.so | grep -v ' [a-z] '
0000000000011760 T driver
0000000000011910 T fma_array
```

## Parity table

| # | symbol      | type | in C `.so` | in Rust `.so` | notes |
|---|-------------|------|-----------|---------------|-------|
| 1 | `driver`    | `T` (func) | yes | yes | `void driver(const int *data, int len)` — declared in `include/driver.h` |
| 2 | `fma_array` | `T` (func) | yes | yes | `void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)` — NOT in the header, but non-`static` in `src/driver.c`, so it is part of the ABI surface and must be exported |

## Non-exported C symbols (correctly absent from both)

| symbol   | reason |
|----------|--------|
| `inner`  | declared `static void inner(int *out, int len)` in `src/driver.c` → internal linkage, `t` not `T`. The Rust translation likewise keeps `inner` as a private `unsafe fn`. |

## Undefined / imported symbols

C imports `memcpy` and `printf` from libc. Rust imports `printf` from libc
(the `memcpy` is `ptr::copy_nonoverlapping`, which lowers to `memcpy`).
Both are ordinary libc imports resolved by the dynamic loader.

```
$ nm -D --undefined-only c_src/build/libdriver.so
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U memcpy@GLIBC_2.14
                 U printf@GLIBC_2.2.5
```

The Rust `.so` imports the same two real dependencies (`memcpy@GLIBC_2.14`,
`printf@GLIBC_2.2.5`) plus the standard Rust-runtime set — libc allocator and
I/O (`malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `memmove`,
`memset`, `bcmp`, `strlen`, `read`, `write`, `writev`, `open64`, `close`,
`lseek64`, `stat64`/`fstat64`/`statx`, `mmap64`, `munmap`, `getcwd`, `getenv`,
`readlink`, `realpath`, `syscall`, `abort`, `__errno_location`), TLS
(`__tls_get_addr`, `pthread_key_*`, `pthread_setspecific`,
`__cxa_thread_atexit_impl`) and panic/backtrace unwinding (`_Unwind_*` from
`libgcc`, `dl_iterate_phdr`). Every one of these is provided by glibc/libgcc
and resolved by the dynamic loader.

**Non-libc/non-libgcc undefined symbols in the Rust `.so`: 0.**

## Symbol diff result

**Missing from Rust `.so`: NONE (0).**
**Undefined non-libc symbols in Rust `.so`: NONE (0).**

No module of the C source was skipped: `c_src` contains exactly one
translation unit (`src/driver.c`, 46 lines) and one header
(`include/driver.h`, 28 lines), and all three of its functions
(`fma_array`, `inner`, `driver`) are present in `translation/src/lib.rs`.

## Automated enforcement

`tests/phase_d_symbols.rs` re-derives this table at test time rather than
trusting the snapshot above:

| test | what it enforces |
|------|------------------|
| `d01_every_c_exported_symbol_is_exported_by_rust` | runs `nm -D --defined-only` on both `.so`s, keeps the global (uppercase-type) entries, and asserts the set difference `C \ Rust` is **empty**. Also asserts `driver` and `fma_array` are present in both by name, so a shrinking export list cannot pass silently. |
| `d02_rust_has_no_undefined_non_libc_symbols` | runs `nm -D --undefined-only` on the Rust `.so` and asserts every unresolved name is provided by glibc or libgcc. |
| `d03_both_libraries_resolve_and_are_callable` | `dlopen`s both objects and calls both exported symbols through the FFI, proving the names are live entry points and not just symbol-table entries. |

## Completeness of the translation

`c_src` is one translation unit and one header, 74 lines in total. Its three
functions map one-to-one onto `translation/src/lib.rs`:

| C | Rust | export |
|---|------|--------|
| `void fma_array(int*, const int*, const int*, const int*, int)` | `pub unsafe extern "C" fn fma_array` | `#[unsafe(no_mangle)]` |
| `static void inner(int*, int)` | `unsafe fn inner` | none (matches the C's internal linkage) |
| `void driver(const int*, int)` | `pub unsafe extern "C" fn driver` | `#[unsafe(no_mangle)]` |

No C module was skipped, so no additional translation work was required in
Phase A or Phase D; nothing is stubbed and nothing is `unimplemented!()`.
