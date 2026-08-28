# Dynamic Symbol Surface

Generated from:

```text
nm -D ../c_src/build/libharvest-work-C1uliL.so
nm -D target/release/libdiv_euclid_lib.so
```

## Complete C `nm -D` inventory

| C type | symbol | C status | Rust dynamic-table status |
|--------|--------|----------|---------------------------|
| `w` | `_ITM_deregisterTMCloneTable` | weak undefined toolchain hook | present, weak undefined |
| `w` | `_ITM_registerTMCloneTable` | weak undefined toolchain hook | present, weak undefined |
| `w` | `__cxa_finalize@GLIBC_2.2.5` | weak undefined libc hook | present, weak undefined |
| `w` | `__gmon_start__` | weak undefined toolchain hook | present, weak undefined |
| `T` | `div_euclid` | defined public API | present, defined public API |

## Required defined exports

| symbol | C | Rust | status |
|--------|---|------|--------|
| `div_euclid` | `T` | `T` | [x] |

Missing defined C exports in Rust: **0**

