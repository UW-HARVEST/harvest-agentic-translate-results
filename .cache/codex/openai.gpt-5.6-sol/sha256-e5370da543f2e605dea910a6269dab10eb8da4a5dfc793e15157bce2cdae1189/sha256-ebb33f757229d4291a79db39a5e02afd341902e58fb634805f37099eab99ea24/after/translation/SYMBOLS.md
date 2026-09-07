# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-TYH8of.so
nm -D --defined-only target/release/libnext_double_lib.so
```

The C dynamic symbol table has one globally defined public symbol. The other
entries printed by plain `nm -D` are undefined weak C runtime symbols, not
library API exports.

| # | C symbol | C type | Rust symbol | Status |
|---|----------|--------|-------------|--------|
| 1 | `next_double` | `T` | `next_double` | [x] exact export present |

Missing C exports in Rust: **0**

