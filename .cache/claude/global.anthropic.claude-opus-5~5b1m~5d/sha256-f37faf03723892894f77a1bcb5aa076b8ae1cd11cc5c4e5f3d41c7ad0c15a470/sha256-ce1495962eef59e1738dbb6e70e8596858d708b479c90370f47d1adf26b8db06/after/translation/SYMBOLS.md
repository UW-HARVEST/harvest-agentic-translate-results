# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-te7RZi.so` (name comes from the parent
  directory via `cmake_path(GET parent FILENAME project_name)`)
* Rust `.so`: `translation/target/{debug,release}/liboverunder_lib.so`

Reproduce with:

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/lib*.so                       | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only translation/target/release/liboverunder_lib.so | awk '{print $3}' | sort -u)
```

## Defined (exported) symbols

| # | C symbol | C source | exported by Rust `.so` | Rust item |
|---|----------|----------|------------------------|-----------|
| 1 | `safe_double_to_int`       | `src/lib.c:39`  | YES | `#[unsafe(no_mangle)] pub extern "C" fn safe_double_to_int` |
| 2 | `process_with_fallthrough` | `src/lib.c:51`  | YES | `#[unsafe(no_mangle)] pub extern "C" fn process_with_fallthrough` |
| 3 | `copy_data_block`          | `src/lib.c:77`  | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn copy_data_block` |
| 4 | `handle_pointer_operations`| `src/lib.c:81`  | YES | `#[unsafe(no_mangle)] pub extern "C" fn handle_pointer_operations` |
| 5 | `overunder`                | `src/lib.c:92`  | YES | `#[unsafe(no_mangle)] pub extern "C" fn overunder` |

`include/lib.h` declares only `overunder`, but the other four have external
linkage in `lib.c` (no `static`) and are therefore part of the `.so` ABI. All
five are exercised directly through `libloading` in the differential tests.

There are no macro-generated exported symbols: `MAKE_VAR_NAME` and `PRINT_VAR`
only paste *local* variable names / string literals inside function bodies.

**Symbol diff (C-only symbols missing from Rust): EMPTY.** No symbol required
translation work; no stubs were introduced.

## Undefined (imported) symbols

Both libraries import only libc/runtime symbols, so there are 0 missing
non-libc symbols on the Rust side.

| C imports | note |
|---|---|
| `printf@GLIBC_2.2.5`   | all output goes through libc `printf`; Rust imports the same symbol |
| `putchar@GLIBC_2.2.5`  | compiler-generated: both gcc and LLVM lower `printf("\n")` to `putchar('\n')` |
| `memcpy@GLIBC_2.14`    | `copy_data_block` / array copy; Rust `copy_nonoverlapping` lowers to the same |
| `strncpy@GLIBC_2.2.5`  | `strncpy(label, "Source", 19)`; reimplemented in Rust as `strncpy_fixed` (no import needed) |
| `sqrt@GLIBC_2.2.5`     | `temp4`; Rust `f64::sqrt` lowers to the `sqrtsd` instruction (no import needed) |
| `__cxa_finalize`, `__gmon_start__`, `_ITM_*registerTMCloneTable` | standard ELF/toolchain boilerplate, present in both |

The Rust `.so` additionally imports the usual Rust runtime set (`_Unwind_*`,
`malloc`/`free`/`realloc`/`calloc`/`posix_memalign`, `memmove`, `memset`,
`bcmp`, `strlen`, `abort`, `__errno_location`, `pthread_key_*`, `dl_iterate_phdr`,
`open64`/`read`/`write`/`close`/`lseek64`/`stat64`/`fstat64`/`statx`/`readlink`/
`realpath`/`getcwd`/`getenv`/`mmap64`/`munmap`/`writev`/`syscall`/`gettid`/
`__tls_get_addr`/`__cxa_thread_atexit_impl`). These come from `std`'s panic
machinery and allocator shim, are all provided by glibc, and none of them is an
unresolved symbol.

## Verification

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
      the exact same name.
- [x] `nm -D` shows 0 missing / undefined non-libc symbols in the Rust `.so`.
- [x] No stubbed / `unimplemented!()` symbol exists.
