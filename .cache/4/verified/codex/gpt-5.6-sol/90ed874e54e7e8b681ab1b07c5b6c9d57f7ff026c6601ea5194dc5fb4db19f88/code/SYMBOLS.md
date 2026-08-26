# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/debug/libpow43_lib.so
```

Only globally defined dynamic symbols are part of this comparison. Undefined
toolchain/runtime symbols are not library API symbols.

| # | C symbol | C type | Rust symbol | implementation | status |
|---|----------|--------|-------------|----------------|--------|
| 1 | `pow43` | `T` (function) | `pow43` | `src/lib.rs` | [x] |

Missing C API symbols in Rust: **0**

