# Dynamic Symbol Surface

Generated from the default C shared object with:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

| C symbol | Type | Rust parity | Source/translation status |
|----------|------|-------------|---------------------------|
| `bad` | `T` | [x] | Exported by the exact-layout C ABI wrapper in `src/lib.rs`. |
| `good` | `T` | [x] | Exported by the exact-layout C ABI wrapper in `src/lib.rs`. |
| `main` | `T` | [x] | Exported by the exact-layout C ABI wrapper in `src/lib.rs`. |
| `printLine` | `T` | [x] | Exported with the exact C spelling and ABI in `src/lib.rs`. |

The C object also imports `__isoc99_scanf` and `puts` from libc. Its remaining
undefined dynamic symbols are weak compiler/runtime hooks:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
and `__gmon_start__`.
