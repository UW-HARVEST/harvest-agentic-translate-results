# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --format=posix ../c_src/build/libdriver.so
```

Only defined dynamic symbols are part of the library's public symbol surface.
Undefined libc/toolchain imports are dependencies, not exports.

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `driver` | `T` | `driver` | [x] |
| `run` | `T` | `run` | [x] |

Current missing C exports in `target/release/libdriver.so`: **0**.

