# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

Only globally defined text symbols are part of this library's callable API.

| C symbol | C type | Rust symbol present | Status |
|----------|--------|---------------------|--------|
| `flac_validate` | `T` | yes | [x] |
| `tflac_size_memory` | `T` | yes | [x] |

Missing Rust symbols: 0.
Undefined non-system Rust symbols: 0. The remaining dynamic imports resolve
through `libc`, `libgcc_s`, or the ELF loader.

The Rust comparison target is `target/debug/libflac_validate_lib.so`.
