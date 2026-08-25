# Dynamic Symbol Surface

Generated from the default C shared library with:

```text
nm -D --defined-only --format=posix c_src/build/libdriver.so
```

The C library has one defined public dynamic symbol:

| symbol | type | C source | Rust export | status |
|--------|------|----------|-------------|--------|
| `driver` | `T` | `c_src/src/driver.c:59` | `src/lib.rs:39` | present |

There are no missing Rust exports and no macro-generated public symbols.
The C library's undefined dynamic symbols are C runtime/toolchain imports
(`printf`, `puts`, weak `_ITM_*`, weak `__cxa_finalize`, and weak
`__gmon_start__`), not library API symbols.

- [x] Symbol parity: zero C API symbols are missing from the Rust shared
  library.
