# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | Rust symbol | Status |
|----------|-------------|--------|
| `driver` | `driver` | present |
| `run` | `run` | present |

The C shared object has no other defined dynamic symbols. The Rust shared
object is missing zero C-defined dynamic symbols.

Phase D symbol parity: [x] complete.
