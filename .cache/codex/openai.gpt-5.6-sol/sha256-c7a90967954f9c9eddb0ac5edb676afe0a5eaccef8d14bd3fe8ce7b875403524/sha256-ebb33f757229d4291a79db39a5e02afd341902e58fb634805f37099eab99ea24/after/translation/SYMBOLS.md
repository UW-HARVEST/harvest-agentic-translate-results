# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` | [x] |
| `run` | `T` | `run` | [x] |

The C and Rust defined-symbol sets were also independently checked with
`readelf -Ws`, selecting defined global symbols. There are no missing C
symbols in the Rust shared object.
