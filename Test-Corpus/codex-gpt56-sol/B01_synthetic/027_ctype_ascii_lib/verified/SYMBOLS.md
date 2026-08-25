# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --format=posix c_src/build/libdriver.so
```

| # | C symbol | Type | Rust export | Status |
|---|----------|------|-------------|--------|
| 1 | `driver` | `T` | `driver` | [x] |

The C library has one defined public dynamic symbol. The C-minus-Rust symbol
diff is empty.
