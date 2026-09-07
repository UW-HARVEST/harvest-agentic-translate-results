# SYMBOLS.md — Phase A: exported symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C `.so`:    `c_src/build/libharvest-work-cHZhAC.so`
* Rust `.so`: `translation/target/release/libencode_quant_lib.so`

## Source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

```
add_library(${project_name} SHARED src/lib.c)
```

There is **no** `add_executable` target, so the project builds **no driver
binary** (the Phase D "compare stdout of C and Rust binaries" item is
not-applicable, recorded below).

| C source file | lines | translated in Rust? | Rust location |
|---|---|---|---|
| `c_src/include/lib.h` | 1 (one prototype, no macros) | yes | `translation/src/lib.rs` |
| `c_src/src/lib.c` | 62 | yes (whole file) | `translation/src/lib.rs` |

No C module/file is missing from the translation, so no Phase A
"TRANSLATE the missing C source" work was required.

## `nm -D --defined-only` on the C `.so`

| # | symbol | type | exported by Rust `.so`? |
|---|--------|------|-------------------------|
| 1 | `encode_quant` | `T` (global text) | **yes** — `T encode_quant` |

Total C dynamic defined symbols: **1**. Total matched by Rust: **1**.
Symbol diff (C-exported minus Rust-exported): **empty**.

The header declares no namespace/renaming macros, so there are no
macro-generated alias symbols to reproduce.

## Rust `.so` undefined symbols

All undefined symbols in the Rust `.so` are libc / libgcc-unwind imports:
`memcpy`, `malloc`, `free`, `realloc`, `calloc`, `posix_memalign`, `memset`,
`memmove`, `bcmp`, `strlen`, `abort`, `getenv`, `getcwd`, `readlink`,
`realpath`, `open64`, `close`, `read`, `write`, `writev`, `lseek64`, `mmap64`,
`munmap`, `stat64`, `fstat64`, `statx`, `syscall`, `dl_iterate_phdr`,
`__errno_location`, `__tls_get_addr`, `pthread_key_*`, `pthread_setspecific`,
`gettid`, `_Unwind_*`, plus the usual weak `_ITM_*` / `__gmon_start__` /
`__cxa_*` stubs.

**0 missing / undefined non-libc symbols.**

## Verification command

```sh
diff <(nm -D --defined-only c_src/build/libharvest-work-cHZhAC.so \
        | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libencode_quant_lib.so \
        | awk '{print $3}' | grep -v '^_ZN' | sort)
```

Result: no output (identical sets).

## Feature configurations

`translation/Cargo.toml` contains **no `[features]` section** and no optional
dependencies, therefore the crate has exactly **one** build configuration
(default == `--no-default-features`). Phase D's "repeat for every feature
combination" collapses to that single configuration; it is nevertheless
re-run explicitly (see `FEATURES` section of the test report).
