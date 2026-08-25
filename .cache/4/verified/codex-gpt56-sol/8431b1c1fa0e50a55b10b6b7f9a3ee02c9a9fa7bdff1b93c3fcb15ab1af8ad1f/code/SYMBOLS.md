# Dynamic Symbol Surface

Generated from the default C build with:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| # | C symbol | Type | Rust symbol present | Notes |
|---|----------|------|---------------------|-------|
| 1 | `div_euclid` | `T` | yes | Declared by `c_src/include/lib.h`. |

The C library has no other defined dynamic symbols. Toolchain-generated weak
undefined entries shown by unfiltered `nm -D` are not public library exports.

