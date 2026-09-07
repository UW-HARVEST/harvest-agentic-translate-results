# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | Rust export | Status |
|----------|-------------|--------|
| `bad` | `bad` | [x] |
| `driver` | `driver` | [x] |
| `good` | `good` | [x] |
| `printIntLine` | `printIntLine` | [x] |
| `printLine` | `printLine` | [x] |

Missing C exports in Rust: **0**.

