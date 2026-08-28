# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | C type | Rust type | Status |
|----------|--------|-----------|--------|
| `decode_base64` | `T` | `T` | Present |

The C library has no other defined dynamic symbols. Its undefined dynamic
symbols are the libc functions `calloc`, `free`, `malloc`, and `strlen`, plus
the weak toolchain symbols `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`. These are
imports, not library exports.

- [x] Final Phase D symbol diff is empty.
