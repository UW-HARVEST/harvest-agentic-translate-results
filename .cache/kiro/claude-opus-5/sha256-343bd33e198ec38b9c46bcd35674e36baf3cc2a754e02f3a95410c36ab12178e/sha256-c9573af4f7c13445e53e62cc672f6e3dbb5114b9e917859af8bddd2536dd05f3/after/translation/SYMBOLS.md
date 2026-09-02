# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-JOS7Vx.so
nm -D --defined-only translation/target/release/libgotomach_lib.so
```

## C source inventory (`c_src/src/lib.c`, 199 lines — the ONLY C source file)

`c_src/CMakeLists.txt` builds exactly one translation unit: `src/lib.c`.
There is no other C module, so there is no "whole file never translated" gap.

Definitions in the C source and their linkage:

| C definition | linkage | in dynamic symtab? |
|---|---|---|
| `is_valid_state` | `static` | no |
| `check_char_flag` | `static` | no |
| `init_processor` | `static` | no |
| `cleanup_processor` | `static` | no |
| `process_value` | external | **yes** |
| `double_value` | external | **yes** |
| `triple_value` | external | **yes** |
| `gotomach` | external | **yes** |

Macro check: `MAKE_FUNC_NAME`, `LOG_MSG` and `CREATE_LABEL` are defined in
`lib.c` but `MAKE_FUNC_NAME`/`CREATE_LABEL` are **never expanded**, so no
symbol name is macro-generated. `LOG_MSG` only expands to a `printf` call.
Therefore the linker names equal the source-level names.

`operation_fn` is a typedef, `ProcessorState` a struct — no symbols.

## Symbol table diff

C `.so` defined symbols (T = text, global):

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `process_value` | T | T | OK |
| 2 | `double_value`  | T | T | OK |
| 3 | `triple_value`  | T | T | OK |
| 4 | `gotomach`      | T | T | OK |

**Missing from Rust: 0.**

The Rust `.so` additionally exports Rust-internal `_ZN…`/`__rust_*` symbols
from `std`; extra symbols are not a parity failure (only *missing* ones are).

## Undefined (imported) symbols

C `.so` imports, excluding weak toolchain markers
(`_ITM_*`, `__cxa_finalize`, `__gmon_start__`):

```
free@GLIBC_2.2.5
malloc@GLIBC_2.2.5
puts@GLIBC_2.2.5
```

Note `puts`, not `printf`: `LOG_MSG(level, msg)` expands to
`printf("[" #level "] " msg "\n")`, a format string with no conversion
specifiers, which GCC lowers to `puts`. The Rust translation calls `puts`
directly for byte-identical stdout.

Rust `.so` imports the same three plus only libc/`libgcc_s` unwinder symbols
pulled in by `std` (`malloc`, `calloc`, `realloc`, `free`, `posix_memalign`,
`memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `abort`, `__errno_location`,
`open64`/`read`/`write`/`close`/`lseek64`/`stat64`/`fstat64`/`statx`,
`mmap64`/`munmap`, `getcwd`/`getenv`/`readlink`/`realpath`, `syscall`,
`dl_iterate_phdr`, `pthread_key_*`, `pthread_setspecific`, `__tls_get_addr`,
`gettid`, `writev`, `_Unwind_*`).

**Non-libc / non-toolchain undefined symbols in the Rust `.so`: 0.**

## Verdict

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
