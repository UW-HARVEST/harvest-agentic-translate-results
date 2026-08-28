# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libString_Slice.so
nm -D translation/target/release/libString_Slice.so
```

## Public C exports

| C symbol | C type | Rust type | Status |
|----------|--------|-----------|--------|
| `slice` | `T` | `T` | [x] |

Missing Rust exports: **0**

## C dynamic imports

These are undefined dependencies, not public library exports.

| Symbol | Type | Provider |
|--------|------|----------|
| `_ITM_deregisterTMCloneTable` | weak undefined | compiler runtime |
| `_ITM_registerTMCloneTable` | weak undefined | compiler runtime |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined | libc |
| `__gmon_start__` | weak undefined | compiler runtime |
| `printf@GLIBC_2.2.5` | undefined | libc |
| `puts@GLIBC_2.2.5` | undefined | libc |
| `strlen@GLIBC_2.2.5` | undefined | libc |

Undefined non-libc application symbols: **0**
