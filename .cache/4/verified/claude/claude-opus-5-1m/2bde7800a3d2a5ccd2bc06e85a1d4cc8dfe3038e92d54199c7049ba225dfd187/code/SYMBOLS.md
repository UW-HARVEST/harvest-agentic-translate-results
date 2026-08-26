# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Generated mechanically from:

```sh
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cargo build
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/debug/libcomplexmode_lib.so
```

C shared object:    `c_src/build/libtranslated_rust.so`
Rust shared object: `target/debug/libcomplexmode_lib.so`

## Defined (exported) symbols

Only `src/lib.c` exists in `c_src/CMakeLists.txt`, so the whole C library is one
translation unit.  Every function in it has external linkage (none are `static`),
so all 7 are exported.

| # | C symbol (`nm -D`, type `T`) | in Rust `.so` | Rust definition |
|---|------------------------------|---------------|-----------------|
| 1 | `check_permissions`   | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn check_permissions` |
| 2 | `compare_operations`  | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn compare_operations` |
| 3 | `complexmode`         | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn complexmode` |
| 4 | `copy_and_sum`        | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn copy_and_sum` |
| 5 | `create_result_string`| YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn create_result_string` |
| 6 | `multiply_with_log`   | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn multiply_with_log` |
| 7 | `safe_add`            | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn safe_add` |

**Symbol diff (C defined \ Rust defined): EMPTY.** No symbol is missing, so no
missing C module had to be translated and no export wrapper had to be added.
`c_src/include/lib.h` only declares `complexmode`, but the other six functions
are non-`static` in `lib.c` and therefore part of the ABI surface; all six are
exported by the Rust `.so` too.

The C `.so` additionally exposes the usual weak/linker-generated symbols
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`) — these are toolchain artifacts, not library API, and the Rust
`.so` has the same weak entries.

Verified automatically by three tests in `tests/symbols.rs`:

* `c_and_rust_export_the_same_symbols` — re-runs `nm -D --defined-only` on both
  objects and asserts the C-minus-Rust set difference is **empty**, plus the
  seven names explicitly and `len == 7`;
* `rust_so_has_no_unresolved_imports` — `dlopen(RTLD_NOW)` on the Rust `.so`, so
  every relocation must resolve at load time;
* `every_exported_symbol_is_callable_through_dlsym` — resolves all seven symbols
  in *both* objects with the right FFI signature and calls one of them.

The same assertions run against the C `.so` built with `CMAKE_BUILD_TYPE` unset
(`-O0`), `Debug`, `Release`, `RelWithDebInfo` and `MinSizeRel` (selected with
`CDIFF_C_SO=`), and against the Rust `.so` in both the debug and the release
profile — the export set is 7/7 in every case.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/debug/libcomplexmode_lib.so` lists only
libc / libgcc-unwind imports:

* libc: `malloc`, `free`, `calloc`, `realloc`, `posix_memalign`, `memcpy`,
  `memmove`, `memset`, `bcmp`, `strcmp`, `strlen`, `printf`, `snprintf`,
  `write`, `writev`, `read`, `close`, `open64`, `lseek64`, `fstat64`, `stat64`,
  `statx`, `mmap64`, `munmap`, `getcwd`, `getenv`, `readlink`, `realpath`,
  `syscall`, `abort`, `__errno_location`, `gettid`, `dl_iterate_phdr`,
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`,
  `__tls_get_addr`, `__cxa_thread_atexit_impl`, `__cxa_finalize`
* libgcc unwinder (Rust std panic machinery): `_Unwind_*`

**0 missing / undefined non-libc symbols.**

The five libc functions the C code itself imports (`malloc`, `free`, `memcpy`,
`strcmp`, `printf`/`puts`, `snprintf`) are imported by the Rust `.so` as well —
the translation deliberately calls the *same* C runtime so that
`malloc`/`free` ownership can be handed across the FFI boundary, `strcmp`
returns the identical implementation-defined magnitude, and `printf`/`snprintf`
format bytes identically.  (`puts` appears in the C `.so` only because GCC
rewrites `printf("literal\n")` into `puts("literal")`; both write the same bytes
to the same `stdout` buffer — and at `opt-level > 0` LLVM performs the very same
rewrite, so the release Rust `.so` imports `puts` as well.)
