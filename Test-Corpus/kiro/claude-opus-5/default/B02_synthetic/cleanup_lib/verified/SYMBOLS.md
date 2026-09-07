# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D` on both shared objects.

- C  `.so`: `c_src/build/libharvest-work-abWLrD.so`
- Rust `.so`: `translation/target/release/libcleanup_lib.so`

Reproduce with:

```sh
nm -D --defined-only c_src/build/libharvest-work-abWLrD.so | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libcleanup_lib.so | awk '{print $3}' | sort > /tmp/r.syms
diff /tmp/c.syms /tmp/r.syms   # MUST be empty
```

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | source of truth | status |
|---|--------|---------|------------|-----------------|--------|
| 1 | `cleanup`           | T | T | `c_src/src/lib.c:34` (decl `include/lib.h:24`) | OK |
| 2 | `print_result`      | T | T | `c_src/src/lib.c:79`                            | OK |
| 3 | `cleanup_resources` | T | T | `c_src/src/lib.c:83`                            | OK |

There are no macro-generated exports, no exported data objects, and no
additional translation units in `c_src` (`CMakeLists.txt` compiles exactly
one file: `src/lib.c`). Nothing was skipped by the translation, so no
missing C module had to be translated.

**Symbol diff: EMPTY.** Both `.so`s export exactly `cleanup`,
`cleanup_resources`, `print_result` and nothing else.

## Undefined symbols (imports)

The C `.so` imports only libc:
`free`, `malloc`, `printf`, `puts`, `snprintf`, `strlen`, `strncmp`
(plus the weak toolchain symbols `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports the same libc set (`malloc`, `free`, `printf`,
`puts`, `snprintf`, `strlen`, plus `bcmp` where LLVM folded the constant
`strncmp`) and additionally the glibc/`libgcc` runtime surface that the Rust
standard library and its unwinder always pull in (`_Unwind_*`, `abort`,
`memcpy`, `mmap64`, `pthread_key_create`, …).

**0 missing / 0 undefined non-libc symbols in the Rust `.so`.** Every
undefined Rust symbol resolves to glibc or the platform unwinder; none is an
unresolved reference to translated code.
