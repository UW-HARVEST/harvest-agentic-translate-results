# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libharvest-work-5VbiqP.so
nm -D --defined-only c_src/build/libharvest-work-5VbiqP.so
```

## Public symbols defined by the C library

| symbol | C type | Rust `.so` status |
|---|---:|---|
| `hsv_to_rgb` | `T` | present as `T` |

Missing public symbols: **0**

Phase D exact defined-export diff:

```text
C only:    (empty)
Rust only: (empty)
```

## Other C dynamic-symbol entries

These entries appear in the complete `nm -D` output but are not public
symbols defined by this library.

| symbol | type | classification |
|---|---:|---|
| `_ITM_deregisterTMCloneTable` | `w` | optional toolchain runtime hook |
| `_ITM_registerTMCloneTable` | `w` | optional toolchain runtime hook |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | optional C runtime dependency |
| `__gmon_start__` | `w` | optional toolchain runtime hook |
| `floorf@GLIBC_2.2.5` | `U` | imported libm function |
