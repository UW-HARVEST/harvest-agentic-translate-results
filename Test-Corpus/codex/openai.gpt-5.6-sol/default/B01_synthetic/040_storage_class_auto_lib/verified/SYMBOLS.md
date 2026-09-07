# C Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| # | C symbol | C type | Rust export present |
|---|----------|--------|---------------------|
| 1 | `driver` | `T` | yes |

The C shared object's remaining `nm -D` entries are undefined runtime
references (`printf`) or weak toolchain/runtime references, not symbols
defined and exported by this library.

