# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-kefPtX.so
nm -D --defined-only target/release/libhsl_to_rgb_lib.so
```

## Full `nm -D` Surface

| C symbol | C type | Rust `nm -D` type | Classification | Status |
|----------|--------|-------------------|----------------|--------|
| `_ITM_deregisterTMCloneTable` | `w` | `w` | weak toolchain import | present |
| `_ITM_registerTMCloneTable` | `w` | `w` | weak toolchain import | present |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | `w` | weak libc import | present |
| `__gmon_start__` | `w` | `w` | weak toolchain import | present |
| `fmodf@GLIBC_2.2.5` | `U` | `U` | libm import | present |
| `hsl_to_rgb` | `T` | `T` | defined public API | present |

The C library has one defined public dynamic symbol. The other five rows are
undefined external dependencies and are not library API exports.

- [x] Every C-defined public dynamic symbol is exported by Rust with the exact name.
- [x] Missing C-defined public dynamic symbols: 0.
