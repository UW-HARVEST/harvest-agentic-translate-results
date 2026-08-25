# Dynamic Symbol Surface

Generated from the default C shared object:

```text
cc -shared -fPIC -fno-strict-aliasing -o c_src/build/libdriver_c.so c_src/src/main.c
nm -D --defined-only --extern-only c_src/build/libdriver_c.so
```

| Symbol | C type | Rust parity |
|--------|--------|-------------|
| `driver` | `T` | [x] |
| `main` | `T` | [x] |

The remaining `nm -D` entries are undefined or weak runtime imports:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`, `__isoc99_scanf`, `printf`, and `putchar`. They are not
symbols defined by the C library.
