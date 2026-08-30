# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

Command:

```sh
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `call_fma` | `T` | `call_fma` | present |
| `driver` | `T` | `driver` | present |
| `fma_array` | `T` | `fma_array` | present |

The C library's undefined dynamic entries are the libc functions
`__isoc99_sscanf` and `printf`, plus weak ELF toolchain hooks. They are imports,
not public symbols defined by this library.

Missing C-defined symbols in the Rust library: **0**.
