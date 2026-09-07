# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `driver` | `T` | `driver` | `T` | [x] present |

The unfiltered C `nm -D` output also contains only standard weak toolchain
symbols and libc imports: `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`, `printf`, and
`putchar`. These are not library-defined public API symbols.

Missing C-defined symbols in Rust: **0**.
