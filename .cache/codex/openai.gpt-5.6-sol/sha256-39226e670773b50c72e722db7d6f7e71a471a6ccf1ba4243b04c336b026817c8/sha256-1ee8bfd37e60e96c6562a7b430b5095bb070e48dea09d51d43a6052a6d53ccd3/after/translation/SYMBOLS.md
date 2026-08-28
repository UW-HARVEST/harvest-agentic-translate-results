# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-PdYL9r.so
```

Toolchain/runtime symbols that are undefined by the C shared object are not
library API symbols. The C shared object defines exactly one public symbol.

| C symbol | C declaration | Rust `.so` export | Status |
|----------|---------------|-------------------|--------|
| `normalize` | `void normalize(float *dest, const float *src, int size)` | `normalize` | [x] |

Missing C API symbols in the Rust shared object: **0**

