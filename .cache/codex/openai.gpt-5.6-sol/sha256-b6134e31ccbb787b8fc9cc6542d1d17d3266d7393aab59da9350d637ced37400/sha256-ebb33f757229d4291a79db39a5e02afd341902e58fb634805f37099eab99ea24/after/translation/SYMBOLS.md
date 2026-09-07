# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

The C shared object's defined public dynamic symbols are:

| symbol | C type | Rust export | status |
|--------|--------|-------------|--------|
| `driver` | `T` | `T` | present |
| `foo` | `T` | `T` | present |

The remaining entries printed by plain `nm -D` for the C object are undefined
runtime/libc imports (`printf`, `strchr`, `_ITM_*`, `__cxa_finalize`, and
`__gmon_start__`), not symbols implemented by this library.

- [x] Every C-defined public dynamic symbol is exported by the Rust `.so` with
      the exact same name.
- [x] Missing C-defined symbols: 0.
