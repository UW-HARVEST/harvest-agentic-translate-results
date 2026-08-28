# Dynamic Symbol Surface

Generated from:

```text
nm -D --format=posix ../c_src/build/libharvest-work-Urz3Dq.so
nm -D --format=posix target/release/libgaussian_kernel_lib.so
```

## C dynamic symbols

| C symbol | C type | Classification | Rust `.so` status |
|----------|--------|----------------|-------------------|
| `_ITM_deregisterTMCloneTable` | `w` | toolchain weak import | present |
| `_ITM_registerTMCloneTable` | `w` | toolchain weak import | present |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc weak import | present |
| `__gmon_start__` | `w` | toolchain weak import | present |
| `expf@GLIBC_2.27` | `U` | libm import | present |
| `gaussian_kernel` | `T` | public library API | present as `T` |

## Export parity

- [x] Every C-defined public API symbol is defined by the Rust `.so`.
- [x] Missing C-defined symbols: 0.
- [x] Undefined non-libc/non-toolchain symbols in the Rust `.so`: 0.

The Rust runtime introduces additional libc and `libgcc_s` imports. They are
implementation dependencies, not missing C library exports.
