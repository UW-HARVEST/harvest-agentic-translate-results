# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `call_fma` | `T` | `call_fma` | present |
| `driver` | `T` | `driver` | present |
| `fma_array` | `T` | `fma_array` | present |

Missing C symbols in the Rust shared library: **0**.

The C library's undefined symbols are the libc/toolchain symbols
`__isoc99_sscanf`, `printf`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`.
