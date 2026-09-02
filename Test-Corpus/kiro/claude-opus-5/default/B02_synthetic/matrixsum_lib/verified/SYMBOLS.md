# SYMBOLS.md — dynamic symbol parity

Derived mechanically from:

```sh
nm -D --defined-only  c_src/build/libharvest-work-4VJzNR.so
nm -D --undefined-only c_src/build/libharvest-work-4VJzNR.so
nm -D --defined-only  translation/target/release/libmatrixsum_lib.so
nm -D --undefined-only translation/target/release/libmatrixsum_lib.so
```

## C `.so` exported (defined) dynamic symbols

| # | symbol | type | C declaration | present in Rust `.so` | notes |
|---|--------|------|---------------|-----------------------|-------|
| 1 | `init_array`                | T (text) | `DynamicArray* init_array(size_t)` | YES | `#[no_mangle] extern "C"` |
| 2 | `expand_array`              | T (text) | `int expand_array(DynamicArray*)` | YES | `#[no_mangle] extern "C"` |
| 3 | `add_element`               | T (text) | `int add_element(DynamicArray*, int)` | YES | `#[no_mangle] extern "C"` |
| 4 | `free_array`                | T (text) | `void free_array(DynamicArray*)` | YES | `#[no_mangle] extern "C"` |
| 5 | `process_flags`             | T (text) | `int process_flags(int)` | YES | `#[no_mangle] extern "C"` |
| 6 | `calculate_matrix_checksum` | T (text) | `int calculate_matrix_checksum()` | YES | `#[no_mangle] extern "C"` |
| 7 | `matrixsum`                 | T (text) | `int matrixsum(int,int,int,int)` | YES | the only symbol in `include/lib.h` |
| 8 | `matrix`                    | D (data, size `0x30` = 48 B) | `int matrix[3][4]` | YES (D, size `0x30`) | exported as `pub static mut matrix: [[c_int;4];3]` |

**Missing from the Rust `.so`: NONE.** No `#[no_mangle]` wrapper had to be
added and no C module was untranslated — `c_src/src/lib.c` is the only C
translation unit in `CMakeLists.txt` and every one of its external definitions
has a Rust counterpart with an identical name.

Object type and size match for the one data symbol (`matrix`, `D`, 48 bytes in
both), so a caller resolving `matrix` through `dlsym` sees the same
row-major 3x4 `int` layout in both libraries.

## Undefined (imported) symbols

C imports only `malloc`, `realloc`, `free` (plus the standard weak
`_ITM_*` / `__cxa_finalize` / `__gmon_start__` glibc/ELF boilerplate).

The Rust `.so` imports the same three allocator entry points — it deliberately
declares `extern "C" { malloc/realloc/free }` rather than using Rust's global
allocator, because `init_array` hands raw allocations across the ABI and
`free_array` gives them back, so both libraries must share one heap
implementation.

The remaining Rust undefined symbols are all libc / libgcc-unwind runtime
support pulled in by `std` (`_Unwind_*`, `memcpy`, `memset`, `memmove`, `bcmp`,
`calloc`, `posix_memalign`, `abort`, `__errno_location`, `__tls_get_addr`,
`pthread_key_*`, `dl_iterate_phdr`, `open64`/`read`/`write`/`writev`/`close`/
`lseek64`/`fstat64`/`stat64`/`statx`/`mmap64`/`munmap`/`readlink`/`realpath`/
`getcwd`/`getenv`/`strlen`/`syscall`/`gettid`, and the weak
`__cxa_thread_atexit_impl`).

**Non-libc undefined symbols in the Rust `.so`: 0.**

## Verification checklist

- [x] Every C-exported symbol is exported by the Rust `.so` under the exact same name.
- [x] `matrix` is exported as a data (`D`) object of the same size in both.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.
- [x] No stubs / `unimplemented!()` were introduced to fake parity.
- [x] `Cargo.toml` declares **no** `[features]` section, so the default build is
      the only feature configuration (Phase D feature-combo sweep is a single
      combination; verified explicitly by `combos.sh`).
