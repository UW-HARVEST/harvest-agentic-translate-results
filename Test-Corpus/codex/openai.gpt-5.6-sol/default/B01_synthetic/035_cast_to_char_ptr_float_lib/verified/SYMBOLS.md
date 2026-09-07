# Dynamic symbol surface

Generated from:

```text
nm -D ../c_src/build/libdriver.so
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

## C-defined public symbols

| symbol | C `.so` | Rust `.so` | status |
|--------|----------|------------|--------|
| `driver` | `T` | `T` | [x] exact export present |

Missing C-defined symbols in Rust: **0**.

## C dynamic dependencies

These appear in the complete `nm -D` output but are not symbols implemented by
this library.

| symbol | kind | classification |
|--------|------|----------------|
| `_ITM_deregisterTMCloneTable` | weak undefined | compiler/runtime hook |
| `_ITM_registerTMCloneTable` | weak undefined | compiler/runtime hook |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined | libc/runtime |
| `__gmon_start__` | weak undefined | compiler/runtime hook |
| `printf@GLIBC_2.2.5` | undefined | libc |
| `putchar@GLIBC_2.2.5` | undefined | libc |

Undefined non-libc application/library symbols: **0**.
