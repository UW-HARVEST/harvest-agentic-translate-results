# Exported Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | Rust symbol | Status |
|----------|-------------|--------|
| `bad` | `bad` | present |
| `driver` | `driver` | present |
| `good` | `good` | present |
| `printIntLine` | `printIntLine` | present |
| `printLine` | `printLine` | present |

Missing C symbols in Rust: **0**.

