# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `parse_number` | `T` | `parse_number` | `T` | present |

Defined C API symbols: 1

Missing from Rust: 0

The C library's remaining dynamic symbols are weak toolchain symbols or
undefined libc dependencies (`free`, `malloc`, `memcpy`, and `strtod`), not
library exports.
