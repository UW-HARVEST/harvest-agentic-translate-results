# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C source inventory (completeness check)

The whole C library is two files; both are accounted for in the Rust crate:

| C file | contents | translated in |
|--------|----------|---------------|
| `c_src/include/driver.h` | single declaration `void driver(int x);` | `translation/src/lib.rs` |
| `c_src/src/driver.c` | single definition of `driver` | `translation/src/lib.rs` (`#[no_mangle] pub extern "C" fn driver`) |

No C module is missing from the translation, so no additional translation work
was required for symbol parity.

## Exported (defined, dynamic) symbols

`nm -D --defined-only c_src/build/libdriver.so`

| symbol | type | signature (from `driver.h`) |
|--------|------|-----------------------------|
| `driver` | `T` (global text) | `void driver(int x)` |

`nm -D --defined-only translation/target/release/libdriver.so`

| symbol | type | present in C? |
|--------|------|---------------|
| `driver` | `T` (global text) | yes |

## Parity result

| # | symbol | in C `.so` | in Rust `.so` | status |
|---|--------|-----------|---------------|--------|
| 1 | `driver` | yes | yes | MATCH |

* Symbols exported by C but missing from Rust: **0**
* Symbols exported by Rust but not by C: **0** (Rust exports no extra globals;
  `crate-type = ["cdylib"]` hides all Rust-internal symbols)

## Undefined symbols in the Rust `.so`

`nm -D -u translation/target/release/libdriver.so` lists only libc / libgcc
runtime imports, i.e. **0 missing non-libc symbols**:

* libc: `printf`, `memcpy`, `memmove`, `memset`, `malloc`, `calloc`, `realloc`,
  `free`, `posix_memalign`, `bcmp`, `strlen`, `abort`, `getenv`, `getcwd`,
  `readlink`, `realpath`, `open64`, `close`, `read`, `write`, `writev`,
  `lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`, `munmap`, `syscall`,
  `dl_iterate_phdr`, `__errno_location`, `__cxa_finalize`,
  `__cxa_thread_atexit_impl`, `__tls_get_addr`, `gettid`,
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`
* libgcc unwinder (`panic = "abort"` still links the personality shims):
  `_Unwind_*`
* weak toolchain hooks: `_ITM_registerTMCloneTable`,
  `_ITM_deregisterTMCloneTable`, `__gmon_start__`

The only functional import shared with the C library is `printf@GLIBC_2.2.5`,
which is intentional: the Rust translation calls the platform `printf` so that
formatting, stream selection and buffering are byte-identical to the C code.

## Completion gate

- [x] `nm -D` shows **0** missing symbols in the Rust `.so` relative to the C `.so`.
- [x] `nm -D` shows **0** missing/undefined non-libc symbols in the Rust `.so`.
