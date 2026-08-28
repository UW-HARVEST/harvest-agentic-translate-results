# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| # | symbol | C type | Rust export | status |
|---|--------|--------|-------------|--------|
| 1 | `FIO_createFilename_fromOutDir` | `T` | `FIO_createFilename_fromOutDir` | [x] |
| 2 | `extractFilename` | `T` | `extractFilename` | [x] |

Only symbols defined by the library are part of the implementation surface.
Undefined GLIBC imports and toolchain-generated weak symbols are recorded by
the final parity check but are not implementation exports.
