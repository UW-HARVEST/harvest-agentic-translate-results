# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libdriver.so
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only target/debug/libdriver.so
```

## C-defined public symbols

| symbol | C type | Rust type | parity |
|--------|--------|-----------|--------|
| `driver` | `T` | `T` | [x] |

The C library has one defined public symbol. The Rust library exports the same
symbol with the exact name. Missing defined symbols: **0**.

## C dynamic imports

These entries appear in `nm -D` but are libc/toolchain imports, not symbols
defined by the C library:

| symbol | type |
|--------|------|
| `_ITM_deregisterTMCloneTable` | weak undefined |
| `_ITM_registerTMCloneTable` | weak undefined |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined |
| `__gmon_start__` | weak undefined |
| `printf@GLIBC_2.2.5` | undefined libc |
| `putchar@GLIBC_2.2.5` | undefined libc |

Undefined non-libc library symbols requiring a Rust implementation: **0**.
