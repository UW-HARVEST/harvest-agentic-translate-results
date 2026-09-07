# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Exported (defined, dynamic) symbols of the C `.so`

| # | symbol | type | present in Rust `.so`? | note |
|---|--------|------|------------------------|------|
| 1 | `driver` | `T` (global text) | YES (`T driver`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` in `src/lib.rs` |

`c_src/include/driver.h` declares exactly one function (`void driver(const char*, const char*)`)
and there are no namespacing/renaming macros, so the plain symbol `driver` is the
complete public surface. `c_src/CMakeLists.txt` builds a single target,
`add_library(driver SHARED src/driver.c)` — one translation unit, no other modules,
so no C source file was left untranslated.

Weak/loader-provided symbols (`_ITM_registerTMCloneTable`,
`_ITM_deregisterTMCloneTable`, `__cxa_finalize`, `__gmon_start__`) are toolchain
artifacts present in both objects and are not part of the API.

## Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so      | awk '$2=="T"{print $3}' | sort) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '$2=="T"{print $3}' | sort)
(empty)
```

**Missing from Rust: 0.** Nothing to add, nothing to translate.

## Undefined symbols in the Rust `.so`

All undefined imports of the Rust `.so` resolve to glibc or to the C++/Rust
unwinder shipped with the toolchain — there are no dangling non-libc symbols:

* glibc: `printf`, `strlen`, `memcpy`, `memmove`, `memset`, `bcmp`, `malloc`,
  `calloc`, `realloc`, `free`, `posix_memalign`, `abort`, `__errno_location`,
  `getenv`, `getcwd`, `readlink`, `realpath`, `open64`, `close`, `read`,
  `write`, `writev`, `lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`,
  `munmap`, `dl_iterate_phdr`, `syscall`, `gettid`, `__tls_get_addr`,
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`,
  `__cxa_thread_atexit_impl`
* unwinder (`libgcc_s`): `_Unwind_*`

The extra imports beyond the C library's (`printf`, `strcspn`) come from the
Rust standard library that `cdylib` links in (panic machinery, allocator,
backtrace support); they are not API.

`driver` re-uses glibc `printf` directly, so `%zu` formatting and stdio
buffering behaviour are bit-identical to the C library's by construction.
`strcspn` is reimplemented in Rust rather than imported, which is why it does
not appear in the Rust `.so`'s undefined list — that reimplementation is what
Phases B and C exercise.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, therefore the only
build configuration is the default one:

```
$ cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["features"])'
{}
```

`--no-default-features` and the default build are the same code. Phase D's
"repeat for every feature combination" therefore reduces to the single default
combination, which is additionally re-verified with `--no-default-features`.
