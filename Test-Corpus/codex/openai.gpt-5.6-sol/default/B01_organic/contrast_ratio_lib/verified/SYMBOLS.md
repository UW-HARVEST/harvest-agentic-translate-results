# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libharvest-work-rHFvwL.so
nm -D translation/target/release/libcontrast_ratio_lib.so
```

## Public C definitions

| symbol | C type | Rust export | status |
|--------|--------|-------------|--------|
| `contrast_ratio` | `T` | `T` | present |

The `nm -D --defined-only` C-to-Rust symbol difference is empty.

## C dynamic dependencies

| symbol | kind | Rust resolution |
|--------|------|-----------------|
| `_ITM_deregisterTMCloneTable` | weak runtime symbol | weak runtime symbol |
| `_ITM_registerTMCloneTable` | weak runtime symbol | weak runtime symbol |
| `__cxa_finalize@GLIBC_2.2.5` | weak libc symbol | weak libc symbol |
| `__gmon_start__` | weak runtime symbol | weak runtime symbol |
| `pow@GLIBC_2.29` | libm function | libm function |

- [x] Final release-build symbol difference is empty.
