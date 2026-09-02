# SYMBOLS.md — symbol parity between C `.so` and Rust `.so`

Generated mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source surface (mechanical inventory of `c_src/`)

`c_src/` contains exactly two source files:

| file | lines | contents |
|------|-------|----------|
| `c_src/include/driver.h` | 29 | one declaration: `void driver(float x);` |
| `c_src/src/driver.c` | 40 | `static void print_hex(unsigned char *p, int len)` + `void driver(float x)` |

There is no other translation unit, no macro-generated symbol, no global data,
no constructor/destructor and no `#ifdef`-gated code. `CMakeLists.txt` builds a
single shared library from `src/driver.c` with `-fno-strict-aliasing` and no
`-D` defines.

## Exported (defined, global) symbols

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `driver` | `T driver` | `T driver` | `#[unsafe(no_mangle)] pub extern "C" fn driver(x: f32)` |

Symbol parity alone is NOT sufficient here: the Rust `.so` originally exported
`driver` with the signature `extern "C" fn(c_int)` while the C declares
`void driver(float)`. The name matched, so this table looked clean, but the two
read the argument from different registers (`%edi` vs `%xmm0`). See the "Defect
found and fixed" section of `ERRORS.md`. The signature — not just the name — is
verified by `errors_b8_float_abi_register_class` in `tests/differential.rs`.

`print_hex` is `static` in C (internal linkage) and therefore is intentionally
absent from the C `.so` export table; the Rust translation keeps it private
(module-private `fn print_hex`), so it is likewise absent. This is correct
parity, not a missing export.

### Symbol diff

```
comm -23 <(c_so_exports) <(rust_so_exports)   ->  (empty)
```

**0 symbols missing from the Rust `.so`.**

No C module was left untranslated: `driver.c` is the only source file and both
of its functions (`driver`, `print_hex`) have real translations in
`translation/src/lib.rs`. There are no stubs and no `unimplemented!()`.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only translation/target/release/libdriver.so` lists only
libc / libgcc-unwind imports:

* glibc: `printf`, `putchar`, `memcpy`, `memmove`, `memset`, `bcmp`, `malloc`,
  `calloc`, `realloc`, `free`, `posix_memalign`, `abort`, `__errno_location`,
  `getenv`, `getcwd`, `open64`, `read`, `write`, `writev`, `close`, `lseek64`,
  `stat64`, `fstat64`, `statx`, `readlink`, `realpath`, `mmap64`, `munmap`,
  `dl_iterate_phdr`, `syscall`, `gettid`, `pthread_key_create`,
  `pthread_key_delete`, `pthread_setspecific`, `__tls_get_addr`,
  `__cxa_finalize`, `__cxa_thread_atexit_impl`
* libgcc unwinder: `_Unwind_*`
* toolchain weak stubs: `__gmon_start__`, `_ITM_*(de)registerTMCloneTable`

**0 missing / undefined non-libc symbols.**

Note: LLVM lowers `printf("\n")` to `putchar('\n')` in the Rust build (the
`printf` -> `putchar` libcall simplification). Both write to the same libc
`stdout` stream, so the observable byte stream is unchanged; the discarded
return value is the only difference and it is discarded in the C original too.
The C build performs the same simplification (`putchar` also appears in the C
`.so`'s undefined list).
