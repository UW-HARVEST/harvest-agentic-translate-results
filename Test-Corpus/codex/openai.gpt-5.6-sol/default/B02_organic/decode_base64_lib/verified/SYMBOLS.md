# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `decode_base64` | `T` (global function) | `decode_base64` | Present |

The C shared object's undefined dynamic symbols are libc/toolchain imports
(`calloc`, `free`, `malloc`, `strlen`, `__cxa_finalize`, `_ITM_*`, and
`__gmon_start__`), not library API exports.

Completion: [x] 0 C API symbols are missing from the Rust shared object.
