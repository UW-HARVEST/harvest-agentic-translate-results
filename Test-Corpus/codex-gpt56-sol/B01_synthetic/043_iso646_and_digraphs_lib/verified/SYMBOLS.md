# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` | present |

The C shared library also has undefined references to the libc functions
`printf` and `puts`. These are dependencies, not public symbols implemented by
this library.

Missing C symbols in the Rust shared library: **0**.
