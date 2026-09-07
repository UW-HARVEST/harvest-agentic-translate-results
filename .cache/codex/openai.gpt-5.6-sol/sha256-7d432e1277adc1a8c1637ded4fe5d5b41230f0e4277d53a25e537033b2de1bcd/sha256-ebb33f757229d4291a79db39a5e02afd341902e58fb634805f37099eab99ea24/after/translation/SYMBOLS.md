# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust `.so` status |
|----------|--------|--------------------|
| `driver` | `T` | present |
| `forward_goto_example` | `T` | present |
| `open_with_cleanup` | `T` | present |

The C library's remaining dynamic-symbol entries are undefined libc or
toolchain imports: `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`, `fclose`,
`ferror`, `fgets`, `fopen`, `fprintf`, `fwrite`, `printf`, and `stderr`.
They are dependencies, not symbols defined/exported by this library.

Missing C-defined symbols in Rust: **0**.
