# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` exported symbols (`libdriver.so`, built from `c_src/src/lib.c`)

| # | symbol | type | present in Rust `.so`? | notes |
|---|--------|------|------------------------|-------|
| 1 | `tool_basename` | `T` (text, global) | YES (`T`) | Declared in `c_src/include/lib.h` as `char *tool_basename(char *path);` |

The C header `c_src/include/lib.h` contains exactly one declaration and no
namespace/renaming macros, so there are no macro-generated linker names to
account for. `c_src/src/lib.c` defines no other function and no global data;
`s1`/`s2` are locals.

## Rust `.so` exported symbols (`translation/target/release/libdriver.so`)

| # | symbol | type | in C `.so`? |
|---|--------|------|-------------|
| 1 | `tool_basename` | `T` | YES |

The Rust helper `strrchr` is a private `unsafe fn` (no `#[no_mangle]`), so it is
correctly **not** exported — it is not part of the C surface either (the C code
calls glibc's `strrchr`, which appears as an *undefined* import, not an export).

## Symbol diff

```
C exports not in Rust:   (none)
Rust exports not in C:   (none)
```

**Diff is EMPTY.** No symbol required a new `#[no_mangle]` wrapper, and no C
source file was left untranslated (`c_src` contains exactly one `.c` file,
`src/lib.c`, 22 lines, fully translated in `translation/src/lib.rs`).

## Undefined (imported) symbols

The Rust `.so` imports only libc / libgcc-unwind symbols — there are **0
missing or undefined non-libc symbols**:

`_ITM_*`, `_Unwind_*`, `__cxa_finalize`, `__cxa_thread_atexit_impl`,
`__errno_location`, `__gmon_start__`, `__tls_get_addr`, `abort`, `bcmp`,
`calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`,
`gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`,
`munmap`, `open64`, `posix_memalign`, `pthread_key_*`, `pthread_setspecific`,
`read`, `readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`,
`syscall`, `write`, `writev`.

(These come from the Rust standard library runtime — panic machinery, allocator,
std::io — not from the translated logic.)

The C `.so` imports `strrchr@GLIBC_2.2.5` plus the standard weak ELF symbols.
The Rust version implements `strrchr` in-crate instead of importing it; this is
an implementation detail with no effect on the exported ABI.

## Build configurations

`translation/Cargo.toml` has **no `[features]` section**, so there is exactly
one feature combination (the default, which is empty). `cargo test
--no-default-features` is therefore equivalent to the default build; both are
run in the test matrix script. `crate-type = ["cdylib"]`.
