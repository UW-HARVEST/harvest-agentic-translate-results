# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libdriver_c.so
nm -D target/release/libdriver.so
```

`c_src/build/libdriver_c.so` is built from the library translation unit
`c_src/src/lib.c`. `c_src/src/main.c` is the executable driver, not part of the
shared-library API.

## Defined public symbols

| symbol | C | Rust | status |
|--------|---|------|--------|
| `process_buffer` | `T` | `T` | present |

## C dynamic imports and weak runtime hooks

These are not library-defined public API symbols.

| symbol | kind |
|--------|------|
| `_ITM_deregisterTMCloneTable` | weak toolchain hook |
| `_ITM_registerTMCloneTable` | weak toolchain hook |
| `__cxa_finalize@GLIBC_2.2.5` | weak libc runtime hook |
| `__gmon_start__` | weak toolchain hook |
| `memcpy@GLIBC_2.14` | libc import |
| `memmove@GLIBC_2.2.5` | libc import |

Missing C-defined symbols in Rust: **0**.
