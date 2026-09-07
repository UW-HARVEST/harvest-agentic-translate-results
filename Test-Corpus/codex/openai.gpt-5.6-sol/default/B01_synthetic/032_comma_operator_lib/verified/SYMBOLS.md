# Dynamic symbol surface

Source: `nm -D --defined-only --format=posix ../c_src/build/libdriver.so`.

| C symbol | C type | Rust export present | Notes |
|----------|--------|---------------------|-------|
| `driver` | `T` | [x] | Exported as `extern "C"` with an unmangled name. |

Missing C symbols in the Rust shared library: **0**.
