# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `wcscat` | `T` | `wcscat` | present |

The C shared library exports one public symbol. The Rust shared library exports
the same symbol with the exact name. Missing symbols: **0**.

- [x] `nm -D` shows 0 C public symbols missing from the Rust shared library.
