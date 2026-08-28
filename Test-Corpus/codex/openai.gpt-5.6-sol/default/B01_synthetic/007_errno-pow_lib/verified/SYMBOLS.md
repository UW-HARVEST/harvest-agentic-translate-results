# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libpow.so
nm -D --defined-only target/release/libpow.so
```

| C symbol | C type | Rust type | Status |
|----------|--------|-----------|--------|
| `my_pow` | `T` | `T` | [x] exact export present |

The C shared object has no other defined dynamic symbols. Its undefined
symbols (`__errno_location`, `fprintf`, `pow`, and `stderr`) are provided by
libc/libm; the weak toolchain symbols are not library API.

- [x] Missing C API symbols in Rust: 0
- [x] Undefined non-libc/non-libm C API symbols in Rust: 0
