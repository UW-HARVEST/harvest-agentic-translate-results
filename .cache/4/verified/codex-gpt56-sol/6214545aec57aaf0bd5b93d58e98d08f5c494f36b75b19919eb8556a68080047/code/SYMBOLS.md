# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

The shared library is built from `c_src/src/lib.c`. The executable-only
`main` in `c_src/src/main.c` is not part of the library ABI. All helper
functions in `lib.c` are declared `static` and therefore are not public
symbols.

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `process_decisions` | `T` | `process_decisions` | [x] |

## External Dependencies

The C shared object has no required non-libc function dependencies. Its only
undefined dynamic symbols are weak toolchain/runtime hooks:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, and `__gmon_start__`.

