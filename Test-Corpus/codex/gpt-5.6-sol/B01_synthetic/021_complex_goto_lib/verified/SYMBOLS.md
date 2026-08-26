# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libdriver.so
nm -D target/release/libdriver.so
```

## C-defined public symbols

| symbol | C type | Rust type | status |
|--------|--------|-----------|--------|
| `driver` | `T` | `T` | present |

The C library's remaining dynamic symbols are undefined imports or weak
toolchain hooks:

```text
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
U puts@GLIBC_2.2.5
```

These are not library API definitions. There are zero C-defined public symbols
missing from the Rust shared library and zero undefined non-libc library
symbols.

