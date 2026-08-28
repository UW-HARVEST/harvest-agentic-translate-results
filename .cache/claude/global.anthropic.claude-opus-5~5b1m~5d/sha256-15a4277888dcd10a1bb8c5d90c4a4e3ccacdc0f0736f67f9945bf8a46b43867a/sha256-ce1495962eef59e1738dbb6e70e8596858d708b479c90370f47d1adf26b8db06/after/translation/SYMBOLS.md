# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-LqBoXw.so   (CMake derives the target name
#    from the *parent* directory name, see CMakeLists.txt lines 2-4)

cd translation && cargo build --release --offline
# -> translation/target/release/libpremultiply_lib.so
```

## C `.so` — defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `premultiply` | `T` (global text) | **YES** — `T premultiply` |

That is the complete list. `c_src` contains exactly one translation unit
(`src/lib.c`, 20 lines) declaring exactly one non-static function, so the
exported surface is a single symbol.

## C `.so` — undefined symbols

All weak/toolchain glue, none requiring a Rust counterpart:

`_ITM_deregisterTMCloneTable` (w), `_ITM_registerTMCloneTable` (w),
`__cxa_finalize@GLIBC_2.2.5` (w), `__gmon_start__` (w)

## Rust `.so` — defined dynamic symbols, non-mangled

| symbol | type |
|--------|------|
| `premultiply` | `T` |

No other `#[no_mangle]` symbols are exported. Remaining defined symbols are
`_ZN…` Rust-mangled std internals plus `rust_*` / `__rust_*` runtime hooks,
which are private to the Rust runtime and have no C analogue.

## Rust `.so` — undefined symbols

Every entry resolves to libc (`libc.so.6`) or the unwinder
(`libgcc_s.so.1`): `_Unwind_*`, `__errno_location`, `__tls_get_addr`,
`abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`,
`getcwd`, `getenv`, `gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`,
`memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`,
`pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`,
`readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`,
`write`, `writev`, plus the same weak toolchain glue as the C `.so`.

`ldd` confirms only `linux-vdso.so.1`, `libgcc_s.so.1`, `libc.so.6` and
`ld-linux-x86-64.so.2`.

## Symbol diff

```
symbols in C .so but NOT in Rust .so :  (none)
non-libc undefined symbols in Rust .so:  (none)
```

**Result: symbol parity is EMPTY-DIFF. 0 missing, 0 undefined non-libc.**

No C source file was left untranslated: `c_src/src/lib.c` is the only
`.c` file listed in `CMakeLists.txt` and its only function is
`premultiply`, which `translation/src/lib.rs` implements and exports.

Automated check: `tests/symbols.rs::c_and_rust_export_identical_symbol_sets`
re-derives both symbol lists with `nm -D` at test time and fails on any
difference, so this table cannot silently rot.
