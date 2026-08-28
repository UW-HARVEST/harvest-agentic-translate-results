# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

The public API list is derived with:

```sh
nm -D --defined-only ../c_src/build/libdriver.so
```

Undefined libc/toolchain imports shown by unfiltered `nm -D` are dependencies,
not symbols defined and exported by this library.

| # | C symbol | C type | Rust symbol | Status |
|---|----------|--------|-------------|--------|
| 1 | `custom_strdup` | `T` | `custom_strdup` (`T`) | [x] exact match |

## Parity Check

- [x] Every C-defined dynamic symbol is defined by the Rust shared object.
- [x] Missing C-defined symbols: 0.
