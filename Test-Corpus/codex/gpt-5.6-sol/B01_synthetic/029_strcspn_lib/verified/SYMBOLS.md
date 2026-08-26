# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --extern-only c_src/build/libdriver.so
```

The C shared library defines one public dynamic symbol. ELF weak runtime
symbols and undefined libc imports reported by unfiltered `nm -D` are not
library API exports.

| # | symbol | C type | Rust export | status |
|---|--------|--------|-------------|--------|
| 1 | `driver` | `T` | `driver` (`T`) | [x] |

Missing C API symbols in Rust: **0**

The C library's undefined imports are `printf` and `strcspn`. They are libc
dependencies, not public symbols defined by the library.
