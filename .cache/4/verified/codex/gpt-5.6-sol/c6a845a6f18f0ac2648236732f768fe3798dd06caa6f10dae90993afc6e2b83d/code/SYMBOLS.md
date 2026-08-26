# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | Rust symbol | Status |
|----------|-------------|--------|
| `driver` | `driver` | Present |
| `printHexCharLine` | `printHexCharLine` | Present |

The C library has no other defined dynamic symbols. Its only strong undefined
symbol is the libc function `printf`.

Final symbol diff: **0 missing C symbols in the Rust shared library**.
