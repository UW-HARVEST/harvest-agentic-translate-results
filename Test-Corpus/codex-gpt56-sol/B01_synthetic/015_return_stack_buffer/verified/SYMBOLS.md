# Dynamic Symbol Surface

Source: `nm -D --defined-only c_src/build/libdriver_c.so`.

The CMake project has no shared-library target, so `libdriver_c.so` is built
by relinking CMake's PIC `main.c.o` from the unchanged source with `cc -shared`.

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `printLine` | `T` | `printLine` | Present |
| `bad` | `T` | `bad` | Present |
| `good` | `T` | `good` | Present |
| `main` | `T` | `main` | Present |

The C library's undefined dynamic symbols are libc/toolchain symbols:
`__isoc99_scanf`, `puts`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`.

Missing C symbols in Rust: **0**.
