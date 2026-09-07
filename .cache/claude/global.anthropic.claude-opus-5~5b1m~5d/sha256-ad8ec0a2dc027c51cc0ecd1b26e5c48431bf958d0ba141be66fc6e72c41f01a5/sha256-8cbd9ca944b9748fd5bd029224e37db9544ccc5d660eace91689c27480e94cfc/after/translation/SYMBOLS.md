# SYMBOLS.md — Phase A symbol surface

C shared library: `c_src/build/libharvest-work-a7i4Gw.so`
Rust shared library: `translation/target/release/libarity_lib.so`

Both taken from `nm -D --defined-only <so> | sort`.

## Symbol table

| # | C symbol (`nm -D`) | C type | Rust `.so` exports it? | Rust item |
|---|--------------------|--------|------------------------|-----------|
| 1 | `shift_array`         | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn shift_array` |
| 2 | `process_string`      | `T` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_string` |
| 3 | `apply_bitmask`       | `T` | YES | `#[unsafe(no_mangle)] pub extern "C" fn apply_bitmask` |
| 4 | `init_matrix`         | `T` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn init_matrix` |
| 5 | `compare_allocations` | `T` | YES | `#[unsafe(no_mangle)] pub extern "C" fn compare_allocations` |
| 6 | `arity4`              | `T` | YES | `#[unsafe(no_mangle)] pub extern "C" fn arity4` |
| 7 | `arity2`              | `T` | YES | `#[unsafe(no_mangle)] pub extern "C" fn arity2` |
| 8 | `arity3`              | `T` | YES | `#[unsafe(no_mangle)] pub extern "C" fn arity3` |
| 9 | `arity`               | `T` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn arity` |

**Missing from Rust: NONE. Extra in Rust: NONE. Symbol diff is EMPTY.**

There are no macro-generated symbols, no exported data symbols, and no
exported `const`/global variables in the C source (`DataBlock` is a
file-local `typedef`; every `mask*`/`temp` is a function-local automatic).

## Notes on signatures / ABI

* `include/lib.h` declares `int arity(int len, int *params);` but
  `src/lib.c` **defines** `int arity(unsigned char len, int *params)`.
  `lib.c` does not `#include "lib.h"`, so the compiler never sees the
  conflict. Verified against the emitted code:

  ```
  1634: mov    %edi,%eax
  163a: mov    %al,-0x4(%rbp)      <- only the low 8 bits are kept
  163d: cmpb   $0x1,-0x4(%rbp)
  1641: ja     164a                <- UNSIGNED byte comparison
  ```

  So the effective behaviour for a caller using the public header is
  "truncate `len` to `u8`, then compare unsigned". The Rust translation
  takes `len: c_int` and performs `(len as u32 & 0xff) as u8`, which
  reproduces this exactly (e.g. `len = 256 -> 0 -> -1`,
  `len = -1 -> 255 -> arity4`).

* `void init_matrix(int matrix[3][4])` is an `int (*)[4]` at the ABI
  level, i.e. a plain pointer; Rust models it as `*mut c_int` and indexes
  `i * 4 + j`. Identical ABI.

* The C library calls its own `arity2`/`arity3`/`arity4` **through the
  PLT**, so those calls are interposable. Because the tests `dlopen` both
  libraries with libloading's default `RTLD_LOCAL`, no cross-library
  interposition occurs and each library always calls its own helpers.

## Undefined (imported) symbols

The C `.so` imports exactly `malloc`, `free`, `memmove`, `strlen`
(+ the weak `_ITM_*` / `__cxa_finalize` / `__gmon_start__` stubs).

The Rust `.so` imports that same libc set plus the Rust `std` runtime's
own libc/libgcc dependencies (`_Unwind_*` from libgcc, and
`abort`/`calloc`/`realloc`/`memcpy`/`memset`/`mmap64`/`open64`/`read`/
`write`/`dl_iterate_phdr`/`pthread_key_*`/... from glibc).

**0 missing/undefined non-libc symbols in the Rust `.so`** — every
unresolved entry is a libc or libgcc import that the dynamic loader
satisfies, and none of them is a symbol the C library was supposed to
provide.
