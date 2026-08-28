# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-RrubW1.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `premultiply` | `T` | `premultiply` | present |

The C shared object has no other defined dynamic symbols. Its weak undefined
runtime symbols (`_ITM_*`, `__cxa_finalize`, and `__gmon_start__`) are not
library API exports.

Missing C API symbols in Rust: **0**
