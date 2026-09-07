# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

The C shared library has five globally defined dynamic symbols. The private
`static` functions `goodG2B` and `goodB2G` are not part of the dynamic surface.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `bad` | `T` | `bad` | [x] |
| `driver` | `T` | `driver` | [x] |
| `good` | `T` | `good` | [x] |
| `printIntLine` | `T` | `printIntLine` | [x] |
| `printLine` | `T` | `printLine` | [x] |

The C library's undefined dynamic symbols are libc/toolchain dependencies:
`printf`, `puts`, `__cxa_finalize`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, and `__gmon_start__`. They are not library API
symbols that the Rust shared library must define.

