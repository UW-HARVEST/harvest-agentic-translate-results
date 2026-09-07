# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-PaNz4Z.so
nm -D --defined-only translation/target/release/libgotomach_lib.so
```

The C library is built from a single translation unit (`c_src/src/lib.c`,
listed as the only source in `c_src/CMakeLists.txt`). The public header
`c_src/include/lib.h` declares only `gotomach`, but `process_value`,
`double_value` and `triple_value` are non-`static` definitions and therefore
land in the dynamic symbol table as well. The macros `MAKE_FUNC_NAME` and
`CREATE_LABEL` are defined but never expanded, so there are no
macro-generated symbol names to account for.

## Defined (exported) symbols

| # | C symbol | C type | present in Rust `.so` | Rust item |
|---|----------|--------|-----------------------|-----------|
| 1 | `process_value` | `T` (global text) | yes | `#[no_mangle] pub unsafe extern "C" fn process_value` |
| 2 | `double_value`  | `T` | yes | `#[no_mangle] pub unsafe extern "C" fn double_value` |
| 3 | `triple_value`  | `T` | yes | `#[no_mangle] pub unsafe extern "C" fn triple_value` |
| 4 | `gotomach`      | `T` | yes | `#[no_mangle] pub unsafe extern "C" fn gotomach` |

**Symbol diff (C defined − Rust defined): EMPTY.** 0 missing symbols.

No whole-module gaps: `lib.c` is the only C source file, and every non-static
definition in it has a real Rust implementation (no stubs, no
`unimplemented!()`).

## Static (non-exported) C functions — correctly absent from both `.so`s

These are `static` in `lib.c`, so they must NOT appear in `nm -D` output for
either library. Verified absent from both.

| C symbol | Rust counterpart (private) |
|----------|----------------------------|
| `is_valid_state`    | `unsafe fn is_valid_state` |
| `check_char_flag`   | `fn check_char_flag` |
| `init_processor`    | `unsafe fn init_processor` |
| `cleanup_processor` | `unsafe fn cleanup_processor` |

## Undefined (imported) symbols

| C undefined symbol | Rust `.so` |
|--------------------|------------|
| `malloc@GLIBC_2.2.5` | imported (`extern "C" fn malloc`) |
| `free@GLIBC_2.2.5`   | imported (`extern "C" fn free`) |
| `puts@GLIBC_2.2.5`   | imported (`extern "C" fn puts`) |

Note: the C source writes its log lines with `printf("[" #level "] " msg "\n")`,
which has no conversion specifiers, so the compiler lowers each call to `puts`.
`nm -D -u` on the C `.so` shows an undefined reference to `puts` and none to
`printf`; the Rust translation calls `puts` for the same reason, which keeps the
emitted stdout bytes identical.

The Rust `.so` additionally imports the usual libc/unwind support symbols that
any `cdylib` needs (`memcpy`, `__tls_get_addr`, `_Unwind_*`, `pthread_*`, …).
These are libc/runtime symbols, not untranslated C code.

## Result

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] 0 undefined non-libc symbols in the Rust `.so`.
