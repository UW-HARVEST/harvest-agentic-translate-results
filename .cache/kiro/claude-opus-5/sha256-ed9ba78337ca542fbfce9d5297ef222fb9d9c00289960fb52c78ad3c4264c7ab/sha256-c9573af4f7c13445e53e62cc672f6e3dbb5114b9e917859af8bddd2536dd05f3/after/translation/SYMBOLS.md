# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C `.so`:    `c_src/build/libharvest-work-AVx3r9.so`
* Rust `.so`: `translation/target/{debug,release}/libwcscat_lib.so`

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

```
add_library(${project_name} SHARED
    src/lib.c)
```

`c_src/src/lib.c` (21 lines) defines exactly one function, `wcscat`, declared in
`c_src/include/lib.h`. There is no untranslated module: the whole C surface is
one function, and `translation/src/lib.rs` implements it. No `unimplemented!()`,
`todo!()` or stub exists in the Rust crate (verified by grep).

## `nm -D --defined-only` on the C `.so`

| # | symbol | type | also exported by Rust `.so`? |
|---|--------|------|------------------------------|
| 1 | `wcscat` | `T` (global text) | YES — `T wcscat` |

No macro-generated symbols exist in the C source (no function-generating macros
are used in `lib.c`/`lib.h`).

## Weak/implicit symbols present in the C `.so` (toolchain-generated, not API)

These come from the CRT/compiler, not from `lib.c`, and are *undefined weak*
references rather than exports. They are also present in the Rust `.so`:

| symbol | C `.so` | Rust `.so` |
|--------|---------|------------|
| `_ITM_deregisterTMCloneTable` | `w` (undef weak) | `w` (undef weak) |
| `_ITM_registerTMCloneTable`   | `w` (undef weak) | `w` (undef weak) |
| `__cxa_finalize@GLIBC_2.2.5`  | `w` (undef weak) | `w` (undef weak) |
| `__gmon_start__`              | `w` (undef weak) | `w` (undef weak) |

## Symbol diff

```
comm -23 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only RUST.so | awk '{print $NF}' | sort -u)
```

Result: **empty**. Every symbol exported by the C `.so` is exported by the Rust
`.so` under the exact same name.

## Undefined non-libc symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only:

* glibc imports (`malloc`, `free`, `memcpy`, `memmove`, `memset`, `realloc`,
  `calloc`, `posix_memalign`, `strlen`, `bcmp`, `abort`, `getenv`, `getcwd`,
  `readlink`, `realpath`, `open64`, `close`, `read`, `write`, `writev`,
  `lseek64`, `fstat64`, `stat64`, `statx`, `mmap64`, `munmap`,
  `dl_iterate_phdr`, `syscall`, `__errno_location`, `__tls_get_addr`,
  `pthread_key_*`, `pthread_setspecific`, `__cxa_thread_atexit_impl`, `gettid`)
* the platform unwinder (`_Unwind_*@GCC_*`), pulled in by `libstd`'s panic
  machinery.

Both groups are provided by the system at load time; the Rust `.so` loads
successfully via `dlopen` in the integration tests, which is the operational
proof there is no unresolved non-libc symbol.

**0 missing symbols, 0 unresolved non-libc symbols.** ✅

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is equivalent to the
default). The symbol table above therefore holds for every feature combination;
`ci/check_features.sh` enumerates the (single) combination mechanically rather
than by assumption.
