# Exported Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `rgb_to_hsv` | `T` | `rgb_to_hsv` | present |

The C shared object exports one public symbol. The Rust shared object exports
the same symbol, so the missing-symbol set is empty.
