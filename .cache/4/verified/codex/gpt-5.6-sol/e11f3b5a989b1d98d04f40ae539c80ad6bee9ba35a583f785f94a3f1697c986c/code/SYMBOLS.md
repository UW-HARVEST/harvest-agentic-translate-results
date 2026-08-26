# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --format=posix c_src/build/libtranslated_rust.so
```

Only externally defined text/data symbols are part of the comparison. Weak
toolchain symbols and undefined libc symbols are not library API symbols.

| # | C symbol | C type | Rust symbol present |
|---|----------|--------|---------------------|
| 1 | `dequantize_granule` | `T` | yes |

Rust comparison libraries:
`target/debug/libdequantize_granule_lib.so` and
`target/release/libdequantize_granule_lib.so`.

Missing C symbols in Rust: **0**.
Undefined non-libc API symbols in Rust: **0**.
