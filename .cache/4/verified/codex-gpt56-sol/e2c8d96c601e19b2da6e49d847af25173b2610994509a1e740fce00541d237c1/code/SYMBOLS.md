# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

Only defined global/weak dynamic symbols are API exports. Undefined weak
toolchain entries are imports and are not part of the library's public API.

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `rev16` | `T` | `rev16` | present |

Missing C API symbols in Rust: **0**

