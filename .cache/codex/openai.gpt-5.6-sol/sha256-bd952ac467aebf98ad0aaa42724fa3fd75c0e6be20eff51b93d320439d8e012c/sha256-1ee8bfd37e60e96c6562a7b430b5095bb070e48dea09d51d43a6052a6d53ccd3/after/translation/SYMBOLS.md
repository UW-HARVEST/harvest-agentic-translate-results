# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-gOpvzM.so
nm -D --defined-only target/release/libmemchra2_lib.so
```

The C library has one defined public dynamic symbol. Runtime and libc
dependencies shown as undefined (`U`) or weak (`w`) entries by `nm -D` are not
library exports.

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `memchra2` | `T` | `memchra2` | [x] exact match |

Missing C exports in Rust: **0**.
