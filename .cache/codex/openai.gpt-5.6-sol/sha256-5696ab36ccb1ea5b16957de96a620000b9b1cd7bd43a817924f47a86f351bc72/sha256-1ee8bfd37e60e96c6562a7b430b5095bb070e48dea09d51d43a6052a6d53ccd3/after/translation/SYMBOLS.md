# Dynamic Symbol Surface

Derived from:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-BDhd95.so
nm -D --defined-only target/release/libpow43_lib.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `pow43` | `T` | `pow43` (`T`) | [x] present |

The C library's other `nm -D` entries are weak libc/toolchain imports, not
symbols defined by this library. The Rust library has no missing C-defined
symbols.
