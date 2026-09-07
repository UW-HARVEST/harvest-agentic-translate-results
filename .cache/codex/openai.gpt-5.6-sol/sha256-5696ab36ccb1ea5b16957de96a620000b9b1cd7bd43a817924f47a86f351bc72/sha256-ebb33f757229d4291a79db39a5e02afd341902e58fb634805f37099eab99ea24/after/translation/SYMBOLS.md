# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-3NSYMq.so
```

The C shared library has one globally defined public dynamic symbol.

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `pow43` | `T` | `pow43` | [x] present |

The undefined weak symbols printed by unfiltered `nm -D` are ELF/runtime
support symbols, not library API:

- `_ITM_deregisterTMCloneTable`
- `_ITM_registerTMCloneTable`
- `__cxa_finalize@GLIBC_2.2.5`
- `__gmon_start__`

Missing C API symbols in Rust: **0**.
