# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libtranslated_rust.so
```

## Defined public API

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `jumpnode` | `T` | `jumpnode` | present |

`nm -D --defined-only target/debug/libjumpnode_lib.so` also reports exactly
`jumpnode`. There are no macro-generated or data exports.

## Dynamic imports

These are dependencies of the C shared object, not API definitions that the
Rust shared object must export:

| Symbol | Type |
|--------|------|
| `_ITM_deregisterTMCloneTable` | weak undefined |
| `_ITM_registerTMCloneTable` | weak undefined |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined |
| `__gmon_start__` | weak undefined |
| `sprintf@GLIBC_2.2.5` | undefined libc import |
| `sqrt` | undefined math import |
| `strlen@GLIBC_2.2.5` | undefined libc import |

Missing C API symbols in Rust: **0**.
