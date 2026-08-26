# Dynamic Symbol Surface

Source: `nm -D --defined-only c_src/build/libdriver.so`, built from the
unmodified default CMake configuration.

| C symbol | C type | Rust export |
|----------|--------|-------------|
| `bad` | `T` | [x] |
| `driver` | `T` | [x] |
| `good` | `T` | [x] |
| `printLine` | `T` | [x] |

The complete `nm -D` output also contains the undefined runtime/libc symbols
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`, and `puts`. These are imports rather than library API
definitions.
