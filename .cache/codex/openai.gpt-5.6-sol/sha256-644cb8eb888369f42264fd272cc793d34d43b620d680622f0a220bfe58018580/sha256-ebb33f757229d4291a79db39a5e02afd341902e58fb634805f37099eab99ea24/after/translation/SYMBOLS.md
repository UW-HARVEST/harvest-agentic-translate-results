# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-esKwV7.so
nm -D --defined-only target/release/libhsl_to_rgb_lib.so
```

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `hsl_to_rgb` | `T` | `hsl_to_rgb` | `T` | [x] parity verified |

The C library has no other defined public dynamic symbols. Undefined libc,
libm, compiler-runtime, and loader symbols are dependencies rather than API
exports and are not part of the defined-symbol parity set.
