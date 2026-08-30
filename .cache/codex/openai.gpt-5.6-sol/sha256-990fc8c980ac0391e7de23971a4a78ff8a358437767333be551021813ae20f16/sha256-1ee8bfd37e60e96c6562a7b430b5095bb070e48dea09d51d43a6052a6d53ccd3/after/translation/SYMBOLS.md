# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

Only globally defined API symbols are listed. Undefined libc imports are not
library exports.

| symbol | C type | Rust `.so` status |
|--------|--------|-------------------|
| `driver` | `T` | present |
| `foo` | `T` | present |

Missing C symbols in Rust: **0**.
