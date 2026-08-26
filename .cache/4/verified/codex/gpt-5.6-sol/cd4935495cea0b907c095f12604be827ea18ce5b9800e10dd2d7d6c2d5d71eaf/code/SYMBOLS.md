# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `UTIL_createLinePointers` | `T` (global function) | `UTIL_createLinePointers` | [x] |

The C library's undefined dynamic symbols are the libc functions `malloc` and
`free`, plus weak ELF runtime hooks. They are not library API exports.

