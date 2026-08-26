# Dynamic Symbol Surface

Generated from the default C shared library with:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `driver` | `T`  | `driver`    | Present |

The C library's remaining `nm -D` entries are undefined runtime imports:
`printf@GLIBC_2.2.5`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, and
`__gmon_start__`. They are not public symbols defined by the library.

Missing C-defined public symbols in Rust: **0**.
