# Exported Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | C type | Rust export |
|----------|--------|-------------|
| `bad` | `T` | present |
| `driver` | `T` | present |
| `good` | `T` | present |
| `printIntLine` | `T` | present |
| `printLine` | `T` | present |

Missing from Rust: **0**

The C library's undefined dynamic symbols are only libc/toolchain symbols:
`printf`, `puts`, `__cxa_finalize`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, and `__gmon_start__`.
