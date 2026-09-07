# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `driver` | `T` | `driver` | `T` | present |

Missing C symbols in Rust: **0**

The C library has no other defined public dynamic symbols.

Phase D status: **complete**. The final defined-symbol name/type diff is empty,
and `ldd -r target/release/libdriver.so` reports no unresolved symbols. Its
dynamic imports are supplied by GLIBC and `libgcc_s`.
