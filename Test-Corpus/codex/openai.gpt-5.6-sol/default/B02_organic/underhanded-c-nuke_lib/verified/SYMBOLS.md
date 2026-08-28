# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-fpT7sm.so
nm -D --defined-only target/release/libunderhanded_c_nuke_lib.so
```

| C symbol | C type | Rust type | Rust export | Status |
|----------|--------|-----------|-------------|--------|
| `match` | `T` | `T` | `match` | [x] |
| `spectral_contrast` | `T` | `T` | `spectral_contrast` | [x] |

The C library's only non-library undefined symbols are `memcpy` and `sqrt`.
Both are implementation dependencies, not public API symbols. There are zero
missing C API symbols in the Rust shared library.

The Rust library has zero undefined project or API symbols. Its dynamic imports
are limited to versioned GLIBC/GCC runtime symbols and weak toolchain hooks.
