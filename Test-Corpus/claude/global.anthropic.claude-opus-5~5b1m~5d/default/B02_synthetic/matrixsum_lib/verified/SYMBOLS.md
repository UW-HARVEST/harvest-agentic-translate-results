# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared libraries.

* C library: `c_src/build/libharvest-work-3eT73m.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`;
  the library name is derived from the parent directory name by `CMakeLists.txt`)
* Rust library: `translation/target/{debug,release}/libmatrixsum_lib.so`
  (`crate-type = ["cdylib"]`)

Regenerate with:

```sh
nm -D --defined-only c_src/build/lib*.so   | awk '{print $2, $3}' | sort -k2 > /tmp/c.syms
nm -D --defined-only translation/target/release/libmatrixsum_lib.so \
                                           | awk '{print $2, $3}' | sort -k2 > /tmp/r.syms
diff /tmp/c.syms /tmp/r.syms
```

## Defined (exported) symbols

| # | symbol | C type / size | Rust type / size | C source | present in Rust `.so` |
|---|--------|---------------|------------------|----------|-----------------------|
| 1 | `init_array`                | `T` 0x7f  | `T` 0x5d  | `lib.c:45`  | YES |
| 2 | `expand_array`              | `T` 0x77  | `T` 0x40  | `lib.c:60`  | YES |
| 3 | `add_element`               | `T` 0x78  | `T` 0x68  | `lib.c:75`  | YES |
| 4 | `free_array`                | `T` 0x31  | `T` 0x2c  | `lib.c:88`  | YES |
| 5 | `process_flags`             | `T` 0x80  | `T` 0x1b  | `lib.c:95`  | YES |
| 6 | `calculate_matrix_checksum` | `T` 0x56  | `T` 0x43  | `lib.c:115` | YES |
| 7 | `matrixsum`                 | `T` 0x18c | `T` 0x13c | `lib.c:128` | YES |
| 8 | `matrix`                    | `D` 0x30  | `D` 0x30  | `lib.c:28`  | YES (`#[no_mangle] pub static mut matrix: [[c_int; 4]; 3]`, 48 bytes — identical size) |

**Missing symbols: 0.** The symbol diff (name + `nm` type letter + size for the
data object) is EMPTY. Text-segment sizes differ, which is expected (different
compilers); only names/linkage/type must match.

Notes:

* Only `matrixsum` is declared in the public header `c_src/include/lib.h`, but
  all seven functions plus the `matrix` data object have external linkage in the
  C translation unit and therefore appear in `nm -D`. All of them are part of the
  ABI surface an external caller can reach via `dlsym`, so all of them are
  exported by the Rust crate and all of them are covered by the differential
  tests (see `CONFIGS.md`, `ERRORS.md`).
* `matrix` is exported as mutable data in both libraries. Tests write to it
  through `dlsym("matrix")` in **both** `.so`s and then re-check
  `calculate_matrix_checksum` / `matrixsum`, which proves the Rust translation
  really loads from the global (no constant folding of the initializer).
* `DynamicArray`, the `FLAG_*` macros and `SIZEOF_INT` are compile-time-only
  constructs in C (a `typedef` and `#define`s) — they produce no symbols, so
  there is nothing to export. `DynamicArray`'s **layout** is part of the ABI
  (`init_array` returns a pointer to it); it is `#[repr(C)]` in Rust and the
  layout is asserted field-by-field in the tests.

## Undefined (imported) symbols

C imports: `malloc`, `realloc`, `free` (+ the standard weak
`_ITM_*`/`__cxa_finalize`/`__gmon_start__` glibc boilerplate).

Rust imports the same three allocator symbols — the translation deliberately
calls glibc `malloc`/`realloc`/`free` via `extern "C"` instead of the Rust
allocator, so allocator-edge behaviour (e.g. `malloc(0)`, `realloc(p, 0)`)
is bit-identical to the C. All remaining Rust imports are libc / `libgcc`
unwinder / `std` runtime symbols:

`_Unwind_*`, `__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`,
`close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `lseek64`,
`malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
`posix_memalign`, `pthread_key_*`, `pthread_setspecific`, `read`, `readlink`,
`realloc`, `realpath`, `stat64`, `strlen`, `syscall`, `write`, `writev`,
`gettid` (weak), `statx` (weak), `__cxa_thread_atexit_impl` (weak).

**0 missing / undefined non-libc symbols in the Rust `.so`.**
