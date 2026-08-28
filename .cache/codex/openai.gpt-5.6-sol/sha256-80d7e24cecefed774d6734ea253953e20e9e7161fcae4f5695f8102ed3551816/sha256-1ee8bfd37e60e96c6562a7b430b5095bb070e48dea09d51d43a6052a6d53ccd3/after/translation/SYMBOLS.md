# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-GPFm3T.so
```

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `hsv_to_rgb` | `T` | `hsv_to_rgb` | `T` | present |

The C shared object has one defined public dynamic symbol. The Rust shared
object exports the same symbol with the exact name. There are zero missing
symbols.

The C object's undefined dynamic symbols are runtime/libc/libm dependencies:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`, and `floorf`. None is part of this library's public API.
