# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libpow.so
```

## Defined public API

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `my_pow` | `T` | `my_pow` | [x] |

## Dynamic imports

These entries are undefined runtime/toolchain dependencies, not library API
exports. All non-weak imports are provided by `libc.so.6` or `libm.so.6`.

| Symbol | Type | Provider |
|--------|------|----------|
| `_ITM_deregisterTMCloneTable` | weak undefined | toolchain (optional) |
| `_ITM_registerTMCloneTable` | weak undefined | toolchain (optional) |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined | `libc.so.6` |
| `__errno_location@GLIBC_2.2.5` | undefined | `libc.so.6` |
| `__gmon_start__` | weak undefined | toolchain (optional) |
| `fprintf@GLIBC_2.2.5` | undefined | `libc.so.6` |
| `pow@GLIBC_2.29` | undefined | `libm.so.6` |
| `stderr@GLIBC_2.2.5` | undefined | `libc.so.6` |

Missing C API exports in Rust: **0**.
