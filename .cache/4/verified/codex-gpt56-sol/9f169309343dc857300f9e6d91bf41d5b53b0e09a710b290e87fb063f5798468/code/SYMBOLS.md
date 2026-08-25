# Dynamic Symbol Surface

Derived from the default C shared library with:

```sh
nm -D --defined-only --extern-only c_src/build/libdriver.so
```

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `driver` | `T`  | `driver`    | present |

The C shared library has one defined public dynamic symbol. The Rust shared
library exports the same symbol. The remaining `nm -D` entries in the C
library are undefined or weak compiler/libc runtime imports
(`printf`, `putchar`, `_ITM_*`, `__cxa_finalize`, and `__gmon_start__`), not
library API definitions.

Missing C API symbols in Rust: **0**.
