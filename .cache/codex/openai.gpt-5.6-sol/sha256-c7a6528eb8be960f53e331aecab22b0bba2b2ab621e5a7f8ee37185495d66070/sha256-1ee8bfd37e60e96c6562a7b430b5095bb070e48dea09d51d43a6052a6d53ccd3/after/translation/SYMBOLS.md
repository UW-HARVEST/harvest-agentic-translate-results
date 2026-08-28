# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-XIPnlZ.so
```

Toolchain/runtime undefined and weak symbols are not library API definitions and
are excluded. The C shared object has one defined public symbol.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `flip_horizontal` | `T` | `flip_horizontal` | present |

Missing C symbols in Rust: **0**.
