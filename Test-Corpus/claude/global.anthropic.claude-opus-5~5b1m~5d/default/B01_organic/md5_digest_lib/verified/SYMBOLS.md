# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-Z43aNY.so`
* Rust `.so`: `translation/target/release/libmd5_digest_lib.so`

## C source inventory (completeness check)

The whole C tree is:

```
c_src/CMakeLists.txt
c_src/include/lib.h      (14 lines)
c_src/src/lib.c          (21 lines)
```

`CMakeLists.txt` compiles exactly one translation unit (`src/lib.c`). There are
no other modules, so there is no "whole file never translated" gap: `lib.c`
defines one function, `md5_digest`, and it is translated in `src/lib.rs`.

## Defined (exported) dynamic symbols

`nm -D --defined-only`, filtered to symbols originating in the library itself
(i.e. excluding compiler/runtime-emitted entries such as `_init`, `_fini`,
`__bss_start`, `_edata`, `_end`, and Rust's allocator shims which are `t`/local
or absent from the dynamic table):

| # | symbol      | type | in C `.so` | in Rust `.so` | notes |
|---|-------------|------|------------|---------------|-------|
| 1 | `md5_digest` | `T` (global text) | yes | yes | `#[no_mangle] pub unsafe extern "C" fn md5_digest` |

**Symbol diff (C − Rust): EMPTY.** Every symbol the C `.so` exports is exported
by the Rust `.so` under the exact same name.

Raw output for the record:

```
$ nm -D --defined-only c_src/build/libharvest-work-Z43aNY.so
00000000000010f9 T md5_digest

$ nm -D --defined-only translation/target/release/libmd5_digest_lib.so
0000000000011690 T md5_digest
```

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` reports only libc / libgcc-unwind /
glibc-pthread imports pulled in by the Rust standard library:

`_ITM_*`, `_Unwind_*`, `__cxa_finalize`, `__cxa_thread_atexit_impl`,
`__errno_location`, `__gmon_start__`, `__tls_get_addr`, `abort`, `bcmp`,
`calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`,
`gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`,
`munmap`, `open64`, `posix_memalign`, `pthread_key_create`,
`pthread_key_delete`, `pthread_setspecific`, `read`, `readlink`, `realloc`,
`realpath`, `stat64`, `statx`, `strlen`, `syscall`, `write`, `writev`.

**0 missing / undefined non-libc symbols.**

## Non-symbol ABI surface (must also match)

The header exports types, not just functions; these are checked by
`tests/differential.rs::abi_layout_matches_c`, which compares the Rust
`size_of`/`align_of`/field offsets against values computed by a C probe
compiled from `c_src/include/lib.h`.

| item | C | Rust |
|------|---|------|
| `sizeof(tflac_md5)` | 16 | 16 |
| `alignof(tflac_md5)` | 4 | 4 |
| `offsetof(a,b,c,d)` | 0, 4, 8, 12 | 0, 4, 8, 12 |
| `tflac_u8` | `uint8_t` | `u8` |
| `tflac_u32` | `uint32_t` | `u32` |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is also valid and
identical, since there are no default features). Both are exercised by
`run_all.sh`.

## Binary / driver

`CMakeLists.txt` contains a single `add_library(... SHARED ...)` and no
`add_executable`. The Rust crate declares `crate-type = ["cdylib"]` and has no
`src/main.rs` / `[[bin]]`. **There is no driver executable**, so the
"compare C and Rust stdout" gate is not applicable (vacuously satisfied).
