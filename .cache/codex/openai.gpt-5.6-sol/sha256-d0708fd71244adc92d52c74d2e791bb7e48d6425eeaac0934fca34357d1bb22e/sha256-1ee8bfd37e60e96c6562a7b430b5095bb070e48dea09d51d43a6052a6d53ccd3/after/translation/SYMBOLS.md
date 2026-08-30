# Dynamic Symbol Surface

Generated from:

```text
nm -D ../c_src/build/libdriver.so
```

## C-defined public symbols

| symbol | C type | Rust export | status |
|--------|--------|-------------|--------|
| `driver` | `T` | `T` | present |

The other C dynamic-symbol entries are undefined runtime dependencies or weak
toolchain hooks, not symbols defined by this library:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`, `printf`, and `putchar`.

Missing C-defined symbols in the Rust shared library: **0**.
