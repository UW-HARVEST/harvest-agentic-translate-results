# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | Kind | Rust export present |
|----------|------|---------------------|
| `driver` | `T` (global function) | yes |

The C shared library's undefined dynamic references are libc/toolchain
dependencies (`printf`, `strcspn`, `_ITM_*`, `__cxa_finalize`, and
`__gmon_start__`), not library API symbols.

Completion status: [x] zero C-defined dynamic symbols are missing from the
Rust shared library.
