# Dynamic symbol surface

Generated from:

```text
nm -D ../c_src/build/libharvest-work-LTHfSI.so
nm -D target/release/libunderhanded_c_nuke_lib.so
```

## C-defined public symbols

| symbol | C type | Rust type | Rust export |
|---|---:|---:|---|
| `match` | `T` | `T` | present |
| `spectral_contrast` | `T` | `T` | present |

Missing C-defined symbols in Rust: **0**.

- [x] Final `nm -D --defined-only` subset diff is empty.
- [x] Rust has no undefined reference to `match` or `spectral_contrast`.

## Other entries in the C dynamic table

These are imports or weak toolchain hooks, not API symbols defined by the C
library:

| symbol | C entry |
|---|---|
| `_ITM_deregisterTMCloneTable` | weak undefined |
| `_ITM_registerTMCloneTable` | weak undefined |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined |
| `__gmon_start__` | weak undefined |
| `memcpy@GLIBC_2.14` | undefined libc import |
| `sqrt@GLIBC_2.2.5` | undefined libm import |

The Rust library has no undefined reference to either C-defined API symbol.
