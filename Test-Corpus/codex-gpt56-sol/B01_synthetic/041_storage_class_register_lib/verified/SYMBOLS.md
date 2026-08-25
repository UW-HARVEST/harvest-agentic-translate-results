# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` | present |

The C shared object has one defined public dynamic symbol. The Rust shared
object exports the same symbol, so the missing-symbol diff is empty.
