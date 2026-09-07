# Exported Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` | present |
| `fma_array` | `T` | `fma_array` | present |

The C library's remaining dynamic-symbol entries are weak runtime hooks or
undefined libc imports (`memcpy` and `printf`), not library exports.

- [x] Every C-defined dynamic symbol is exported by the Rust shared library.
- [x] No C-defined dynamic symbol is missing from Rust.
