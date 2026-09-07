# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory

`c_src` contains exactly one translation unit, `src/lib.c` (83 lines), and one
public header, `include/lib.h` (1 line):

```c
char *encode_base64(int size, const char *src);
```

`src/lib.c` defines two functions:

| C function      | linkage           | exported? |
|-----------------|-------------------|-----------|
| `encode`        | `static char`     | no (file-static) |
| `encode_base64` | external          | yes |

No other `.c` files, no macro-generated symbol families, no `#ifdef`-gated
modules. So the complete expected public ABI is a single symbol.

## Defined (exported) symbols

| # | symbol          | C `.so` | Rust `.so` | status |
|---|-----------------|---------|------------|--------|
| 1 | `encode_base64` | `T`     | `T`        | OK — present in both, exact name |

`encode` is correctly absent from both (it is `static` in C, and a private
`fn` in Rust).

**Symbol diff (C-defined minus Rust-defined): EMPTY.**

No symbol needed a new `#[no_mangle]` wrapper, and no C module was left
untranslated — the single translation unit is fully translated.

## Undefined symbols in the Rust `.so`

Every undefined symbol in `translation/target/release/libdriver.so` resolves to
libc / the platform unwinder; there are **0 missing or undefined non-libc
symbols**:

* libc: `calloc`, `strlen`, `free`, `malloc`, `realloc`, `posix_memalign`,
  `memcpy`, `memmove`, `memset`, `bcmp`, `abort`, `getenv`, `getcwd`,
  `readlink`, `realpath`, `open64`, `close`, `read`, `write`, `writev`,
  `lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`, `munmap`, `syscall`,
  `gettid`, `__errno_location`, `dl_iterate_phdr`,
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`,
  `__tls_get_addr`, `__cxa_finalize`, `__cxa_thread_atexit_impl`
* platform unwinder (`libgcc`): `_Unwind_*`
* standard weak link-time hooks: `_ITM_*TMCloneTable`, `__gmon_start__`

`calloc` and `strlen` are the only two the C `.so` also imports; the Rust
`.so`'s extra imports come from the Rust runtime (panic machinery, allocator
shim, std's lazy backtrace support) and are not part of the library's own ABI.

The two ABI-relevant imports match the C exactly, which matters for the
memory-ownership contract: the returned buffer comes from libc `calloc` in both
implementations, so a caller may release it with libc `free` either way.

## Verification

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
- [x] Every symbol the C `.so` exports is exported by the Rust `.so` with the
      exact same name.
