# Dynamic symbol surface

Generated from:

```text
nm -D ../c_src/build/libharvest-work-OmBhnE.so
nm -D target/release/libto_barycentric_lib.so
```

## Public symbols defined by the C library

| C symbol | C type | Rust type | Status |
|----------|--------|-----------|--------|
| `to_barycentric` | `T` | `T` | present |

Missing public symbols: **0** — [x] complete

## Undefined/weak runtime symbols in the C object

These are dynamic-loader/compiler runtime references, not symbols defined by
the library:

| Symbol | C `nm -D` type |
|--------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` |
| `_ITM_registerTMCloneTable` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` |
| `__gmon_start__` | `w` |
