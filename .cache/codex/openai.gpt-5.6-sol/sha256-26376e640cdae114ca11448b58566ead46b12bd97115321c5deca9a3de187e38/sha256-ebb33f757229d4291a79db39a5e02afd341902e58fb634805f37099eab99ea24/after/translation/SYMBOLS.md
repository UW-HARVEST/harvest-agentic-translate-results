# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

`nm -D` also lists `printf` and `putchar` as undefined libc imports in the C
library. They are not C-library exports and therefore are not part of the
public API parity set.

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `driver` | `T` | `driver` | `T` | [x] exact export match |

Missing C exports in Rust: **0**

